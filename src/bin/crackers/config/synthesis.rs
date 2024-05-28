use serde::Deserialize;
use tracing::Level;

use crackers::synthesis::builder::SynthesisSelectionStrategy;
#[derive(Copy, Clone, Debug, Deserialize)]
pub enum CrackersLogLevel {
    TRACE,
    DEBUG,
    WARN,
    INFO,
    ERROR,
}

impl From<CrackersLogLevel> for Level {
    fn from(value: CrackersLogLevel) -> Self {
        match value {
            CrackersLogLevel::TRACE => Level::TRACE,
            CrackersLogLevel::DEBUG => Level::DEBUG,
            CrackersLogLevel::WARN => Level::WARN,
            CrackersLogLevel::INFO => Level::INFO,
            CrackersLogLevel::ERROR => Level::ERROR,
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct SynthesisConfig {
    pub strategy: SynthesisSelectionStrategy,
    pub max_candidates_per_slot: usize,
    pub parallel: usize,
    pub log_level: CrackersLogLevel,
}

impl Default for SynthesisConfig {
    fn default() -> Self {
        SynthesisConfig {
            strategy: SynthesisSelectionStrategy::OptimizeStrategy,
            max_candidates_per_slot: 50,
            parallel: 4,
            log_level: CrackersLogLevel::INFO,
        }
    }
}
