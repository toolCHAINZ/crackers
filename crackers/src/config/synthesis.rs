#[cfg(feature = "pyo3")]
use pyo3::pyclass;
use pyo3::pymethods;
use serde::{Deserialize, Serialize};

use crate::synthesis::builder::SynthesisSelectionStrategy;

#[derive(Clone, Debug, Deserialize, Serialize)]
#[cfg_attr(feature = "pyo3", pyclass)]
pub struct SynthesisConfig {
    pub strategy: SynthesisSelectionStrategy,
    pub max_candidates_per_slot: usize,
    pub parallel: usize,
    pub combine_instructions: bool,
}

impl Default for SynthesisConfig {
    fn default() -> Self {
        SynthesisConfig {
            strategy: SynthesisSelectionStrategy::OptimizeStrategy,
            max_candidates_per_slot: 50,
            parallel: 4,
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

    #[getter]
    fn strategy(&self) -> SynthesisSelectionStrategy {
        self.strategy
    }

    #[setter]
    fn set_strategy(&mut self, strategy: SynthesisSelectionStrategy) {
        self.strategy = strategy;
    }

    #[getter]
    fn max_candidates_per_slot(&self) -> usize {
        self.max_candidates_per_slot
    }

    #[setter]
    fn set_max_candidates_per_slot(&mut self, max_candidates_per_slot: usize) {
        self.max_candidates_per_slot = max_candidates_per_slot
    }

    #[getter]
    fn parallel(&self) -> usize {
        self.parallel
    }

    #[setter]
    fn set_parallel(&mut self, parallel: usize) {
        self.parallel = parallel
    }

    #[getter]
    fn combine_instructions(&self) -> bool {
        self.combine_instructions
    }

    #[setter]
    fn set_combine_instructions(&mut self, combine_instructions: bool) {
        self.combine_instructions = combine_instructions
    }
}
