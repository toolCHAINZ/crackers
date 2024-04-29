use serde::Deserialize;

use crackers::synthesis::builder::SynthesisSelectionStrategy;

#[derive(Debug, Deserialize)]
pub struct SynthesisConfig{
    pub strategy: SynthesisSelectionStrategy
}

impl Default for SynthesisConfig{
    fn default() -> Self {
        SynthesisConfig{
            strategy: SynthesisSelectionStrategy::OptimizeStrategy
        }
    }
}