use derive_builder::Builder;
use jingle::sleigh::OpCode;
#[cfg(feature = "pyo3")]
use pyo3::pyclass;
#[cfg(feature = "pyo3")]
use pyo3::pymethods;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

use crate::config::error::CrackersConfigError;
use crate::config::object::load_sleigh;
use crate::config::sleigh::SleighConfig;
use crate::gadget::library::GadgetLibrary;
use tracing::{Level, event};

const LIB_ALIGNMENT: u64 = 0x4000; // 16 KiB alignment for loaded libraries
const LIB_GAP: u64 = 0x1000; // small gap between libraries when placing

fn align_up(x: u64, align: u64) -> u64 {
    if align == 0 {
        return x;
    }
    x.div_ceil(align) * align
}

#[derive(Clone, Debug, Default, Builder, Deserialize, Serialize)]
#[builder(default)]
#[cfg_attr(feature = "pyo3", pyclass)]
pub struct LoadedLibraryConfig {
    pub path: String,
    pub base_address: Option<u64>,
}

#[derive(Clone, Debug, Default, Builder, Deserialize, Serialize)]
#[builder(default)]
#[cfg_attr(feature = "pyo3", pyclass)]
pub struct GadgetLibraryConfig {
    pub max_gadget_length: usize,
    #[serde(skip, default = "default_blacklist")]
    pub operation_blacklist: HashSet<OpCode>,
    pub path: String,
    pub sample_size: Option<usize>,
    pub base_address: Option<u64>,
    /// Additional libraries to load alongside the primary one. Each entry may
    /// optionally specify a base address. If no base address is provided the
    /// builder will attempt to place the library in an address region that does
    /// not conflict with the main library or previously placed libraries.
    pub loaded_libraries: Option<Vec<LoadedLibraryConfig>>,
}

impl GadgetLibraryConfig {
    pub fn build(&self, sleigh: &SleighConfig) -> Result<GadgetLibrary, CrackersConfigError> {
        let mut library_sleigh = load_sleigh(&self.path, sleigh)?;
        if let Some(addr) = self.base_address {
            let aligned = align_up(addr, LIB_ALIGNMENT);
            if aligned != addr {
                event!(
                    Level::WARN,
                    "Main library base address {:#x} is not {:#x}-aligned; aligning to {:#x}",
                    addr,
                    LIB_ALIGNMENT,
                    aligned
                );
            }
            library_sleigh.set_base_address(aligned)
        }

        // Prepare a vector of sleigh contexts (main + any additional libraries)
        // Start with the primary library context.
        let mut sleighs = vec![library_sleigh];

        // If there are additional libraries to load, load them and
        // assign base addresses so they do not conflict with the main
        // library or each other.
        if let Some(loaded) = &self.loaded_libraries {
            // Collect occupied ranges from the main library (first entry in `sleighs`)
            let mut occupied: Vec<(u64, u64)> = sleighs[0]
                .get_sections()
                .map(|s| {
                    let start = s.base_address as u64;
                    let end = start + s.data.len() as u64;
                    (start, end)
                })
                .collect();

            let mut current_max: u64 = occupied.iter().map(|(_, e)| *e).max().unwrap_or(0);

            for cfg in loaded {
                let mut other = load_sleigh(&cfg.path, sleigh)?;
                // Use module-level alignment constants
                if let Some(addr) = cfg.base_address {
                    // If user provided a base address, ensure it is aligned to LIB_ALIGNMENT.
                    let aligned = align_up(addr, LIB_ALIGNMENT);
                    if aligned != addr {
                        event!(
                            Level::WARN,
                            "Provided base address {:#x} for '{}' is not {:#x}-aligned; adjusting to {:#x}",
                            addr,
                            cfg.path,
                            LIB_ALIGNMENT,
                            aligned
                        );
                    }
                    other.set_base_address(aligned);
                } else {
                    // Place the library after the current known max address with a small gap,
                    // then align up to LIB_ALIGNMENT to guarantee alignment.
                    let base_hint = if current_max == 0 {
                        LIB_GAP
                    } else {
                        current_max + LIB_GAP
                    };
                    let candidate = align_up(base_hint, LIB_ALIGNMENT);
                    event!(
                        Level::INFO,
                        "Auto-placing '{}' at aligned address {:#x} (base hint {:#x})",
                        cfg.path,
                        candidate,
                        base_hint
                    );
                    other.set_base_address(candidate);
                }

                // Update occupied ranges and current_max with this library's sections
                for s in other.get_sections() {
                    let start = s.base_address as u64;
                    let end = start + s.data.len() as u64;
                    occupied.push((start, end));
                    if end > current_max {
                        current_max = end;
                    }
                }

                // Keep the loaded context so we can pass all contexts to the
                // gadget library builder.
                sleighs.push(other);
            }
        }

        // Build gadget library from all provided sleigh contexts.
        GadgetLibrary::build_from_image(sleighs, self).map_err(CrackersConfigError::Sleigh)
    }
}

fn default_blacklist() -> HashSet<OpCode> {
    HashSet::from([
        // Unlikely to be in any useful chains that we're currently considering
        // While call is potentially going to exist in certain cases (e.g. mmap), we
        // can just as easily redirect to such functions with an indirect jump, so we still remove
        // it from consideration
        OpCode::CPUI_BRANCH,
        OpCode::CPUI_CALL,
        // The following operations are not yet modeled by jingle, so let's save some trees
        // and not even try to model them for the time being
        OpCode::CPUI_CBRANCH,
        OpCode::CPUI_FLOAT_ADD,
        OpCode::CPUI_FLOAT_ABS,
        OpCode::CPUI_FLOAT_CEIL,
        OpCode::CPUI_FLOAT_DIV,
        OpCode::CPUI_FLOAT_EQUAL,
        OpCode::CPUI_FLOAT_FLOAT2FLOAT,
        OpCode::CPUI_FLOAT_FLOOR,
        OpCode::CPUI_FLOAT_INT2FLOAT,
        OpCode::CPUI_FLOAT_LESS,
        OpCode::CPUI_FLOAT_LESSEQUAL,
        OpCode::CPUI_FLOAT_MULT,
        OpCode::CPUI_FLOAT_NAN,
        OpCode::CPUI_FLOAT_NEG,
        OpCode::CPUI_FLOAT_NOTEQUAL,
        OpCode::CPUI_FLOAT_ROUND,
        OpCode::CPUI_FLOAT_SQRT,
        OpCode::CPUI_FLOAT_SUB,
        OpCode::CPUI_FLOAT_TRUNC,
        OpCode::CPUI_CPOOLREF,
        OpCode::CPUI_CAST,
        OpCode::CPUI_MULTIEQUAL,
    ])
}

#[cfg(feature = "pyo3")]
#[pymethods]
impl LoadedLibraryConfig {
    #[getter]
    pub fn get_path(&self) -> &str {
        self.path.as_str()
    }

    #[setter]
    pub fn set_path(&mut self, p: String) {
        self.path = p;
    }

    #[getter]
    pub fn get_base_address(&self) -> Option<u64> {
        self.base_address
    }

    #[setter]
    pub fn set_base_address(&mut self, a: Option<u64>) {
        self.base_address = a;
    }
}

#[cfg(feature = "pyo3")]
#[pymethods]
impl GadgetLibraryConfig {
    #[getter]
    pub fn get_max_gadget_length(&self) -> usize {
        self.max_gadget_length
    }

    #[setter]
    pub fn set_max_gadget_length(&mut self, l: usize) {
        self.max_gadget_length = l;
    }

    #[getter]
    pub fn get_path(&self) -> &str {
        self.path.as_str()
    }

    #[setter]
    pub fn set_path(&mut self, l: String) {
        self.path = l;
    }

    #[getter]
    pub fn get_sample_size(&self) -> Option<usize> {
        self.sample_size
    }

    #[setter]
    pub fn set_sample_size(&mut self, l: Option<usize>) {
        self.sample_size = l;
    }

    #[getter]
    pub fn get_base_address(&self) -> Option<u64> {
        self.base_address
    }

    #[setter]
    pub fn set_base_address(&mut self, l: Option<u64>) {
        self.base_address = l;
    }

    #[getter]
    pub fn get_loaded_libraries(&self) -> Option<Vec<LoadedLibraryConfig>> {
        self.loaded_libraries.clone()
    }

    #[setter]
    pub fn set_loaded_libraries(&mut self, l: Option<Vec<LoadedLibraryConfig>>) {
        self.loaded_libraries = l;
    }
}
