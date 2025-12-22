#[cfg(feature = "pyo3")]
use pyo3::{pyclass, pymethods};
use serde::{Deserialize, Serialize};

use crate::synthesis::builder::SynthesisSelectionStrategy;

#[derive(Clone, Debug, Deserialize, Serialize)]
#[cfg_attr(feature = "pyo3", pyclass(get_all, set_all))]
pub struct SynthesisConfig {
    pub strategy: SynthesisSelectionStrategy,
    pub max_candidates_per_slot: usize,
    pub parallel: usize,
    pub combine_instructions: bool,
}

impl Default for SynthesisConfig {
    fn default() -> Self {
        SynthesisConfig {
            strategy: SynthesisSelectionStrategy::SatStrategy,
            max_candidates_per_slot: 200,
            parallel: 6,
            combine_instructions: true,
        }
    }
}

#[cfg(feature = "pyo3")]
#[pymethods]
impl SynthesisConfig {
    #[new]
    fn new(
        strategy: SynthesisSelectionStrategy,
        max_candidates_per_slot: usize,
        parallel: usize,
        combine_instructions: bool,
    ) -> Self {
        SynthesisConfig {
            strategy,
            max_candidates_per_slot,
            parallel,
            combine_instructions,
        }
    }
}
