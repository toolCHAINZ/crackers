#[cfg(feature = "pyo3")]
use pyo3::exceptions::PyRuntimeError;
#[cfg(feature = "pyo3")]
use pyo3::{Bound, PyResult};
#[cfg(feature = "pyo3")]
use pyo3::{pymethods};
#[cfg(feature = "pyo3")]
use pyconfig::wrap_config;
use serde::{Deserialize, Serialize};
#[cfg(feature = "pyo3")]
use pyo3::types::PyType;
use crate::config::constraint::ConstraintConfig;
use crate::config::meta::MetaConfig;
use crate::config::sleigh::SleighConfig;
use crate::config::specification::SpecificationConfig;
use crate::config::synthesis::SynthesisConfig;
use crate::error::CrackersError;
use crate::gadget::library::builder::GadgetLibraryConfig;
use crate::synthesis::builder::{SynthesisParams, SynthesisParamsBuilder};

pub mod constraint;
pub mod error;
pub mod meta;
pub mod object;
pub mod sleigh;
pub mod specification;
pub mod synthesis;

#[derive(Clone, Debug, Deserialize, Serialize)]
#[cfg_attr(feature = "pyo3", wrap_config)]
/// This struct represents the serializable configuration found
/// in a crackers .toml file. Once parsed from a file or constructed
/// programmatically, it can be used to produce a [crate::synthesis::builder::SynthesisParams]
/// struct, which can run the actual algorithm
pub struct CrackersConfig {
    #[serde(default)]
    pub meta: MetaConfig,
    pub specification: SpecificationConfig,
    pub library: GadgetLibraryConfig,
    pub sleigh: SleighConfig,
    pub synthesis: SynthesisConfig,
    pub constraint: Option<ConstraintConfig>,
}

#[cfg(feature = "pyo3")]
#[pymethods]
impl CrackersConfig {
    #[new]
    pub fn new(
        specification: SpecificationConfig,
        sleigh: SleighConfig,
        meta: Option<MetaConfig>,
        library: Option<GadgetLibraryConfig>,
        synthesis: Option<SynthesisConfig>,
        constraint: Option<ConstraintConfig>,
    ) -> Self {
        Self {
            meta: meta.unwrap_or_default(),
            specification,
            library: library.unwrap_or_default(),
            sleigh,
            synthesis: synthesis.unwrap_or_default(),
            constraint,
        }
    }

    #[classmethod]
    pub fn from_toml(_: &Bound<'_, PyType>, path: &str) -> PyResult<Self> {
        let cfg_bytes = std::fs::read(path)?;
        let s = String::from_utf8(cfg_bytes)?;
        let p: CrackersConfig =
            toml_edit::de::from_str(&s).map_err(|e| PyRuntimeError::new_err(format!("{}", e)))?;
        Ok(p)
    }

}

impl CrackersConfig {
    pub fn resolve(&self) -> Result<SynthesisParams, CrackersError> {
        let library = self.library.build(&self.sleigh)?;
        let mut b = SynthesisParamsBuilder::default();
        if let Some(c) = &self.constraint {
            b.preconditions(c.get_preconditions(&library).collect());
            b.postconditions(c.get_postconditions(&library).collect());
            b.pointer_invariants(c.get_pointer_constraints().collect());
        }
        b.gadget_library(library)
            .seed(self.meta.seed)
            .instructions(self.specification.get_spec(&self.sleigh)?);
        b.selection_strategy(self.synthesis.strategy);
        b.combine_instructions(self.synthesis.combine_instructions);
        b.candidates_per_slot(self.synthesis.max_candidates_per_slot);
        b.parallel(self.synthesis.parallel).seed(self.meta.seed);

        let params = b.build()?;
        Ok(params)
    }
}
