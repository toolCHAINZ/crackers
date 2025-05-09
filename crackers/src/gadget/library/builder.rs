use derive_builder::Builder;
use jingle::sleigh::OpCode;
#[cfg(feature = "pyo3")]
use pyo3::pyclass;
use pyo3::pymethods;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

use crate::config::error::CrackersConfigError;
use crate::config::object::load_sleigh;
use crate::config::sleigh::SleighConfig;
use crate::gadget::library::GadgetLibrary;

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
}

impl GadgetLibraryConfig {
    pub fn build(&self, sleigh: &SleighConfig) -> Result<GadgetLibrary, CrackersConfigError> {
        let mut library_sleigh = load_sleigh(&self.path, sleigh)?;
        if let Some(addr) = self.base_address {
            library_sleigh.set_base_address(addr)
        }
        GadgetLibrary::build_from_image(library_sleigh, self).map_err(CrackersConfigError::Sleigh)
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

/**

pub sample_size: Option<usize>,
pub base_address: Option<u64>,
*/

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
}
