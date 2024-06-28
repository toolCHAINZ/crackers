use serde::Deserialize;

use crate::synthesis::builder::SynthesisSelectionStrategy;

#[derive(Debug, Deserialize)]
pub struct SynthesisConfig {
    pub strategy: SynthesisSelectionStrategy,
    pub max_candidates_per_slot: usize,
    pub parallel: usize,
}

impl Default for SynthesisConfig {
    fn default() -> Self {
        SynthesisConfig {
            strategy: SynthesisSelectionStrategy::OptimizeStrategy,
            max_candidates_per_slot: 50,
            parallel: 4,
        }
    }
}
