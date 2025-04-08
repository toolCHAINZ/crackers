#[cfg(feature = "pyo3")]
use pyo3::{pyclass, pymethods};
use pyo3::PyResult;
use serde::{Deserialize, Serialize};

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
#[cfg_attr(feature = "pyo3", pyclass)]
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

    #[getter]
    fn get_meta(&self) -> MetaConfig {
        self.meta.clone()
    }

    #[setter]
    fn set_meta(&mut self, meta: MetaConfig) {
        self.meta = meta
    }

    #[getter]
    fn get_specification(&self) -> SpecificationConfig {
        self.specification.clone()
    }

    #[setter]
    fn set_specification(&mut self, spec: SpecificationConfig) {
        self.specification = spec
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
