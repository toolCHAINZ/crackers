use serde::Deserialize;
use tracing::Level;

use crate::synthesis::builder::SynthesisSelectionStrategy;

#[derive(Copy, Clone, Debug, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum CrackersLogLevel {
    #[serde(rename = "TRACE")]
    Trace,
    Debug,
    Warn,
    Info,
    Error,
}

impl From<CrackersLogLevel> for Level {
    fn from(value: CrackersLogLevel) -> Self {
        match value {
            CrackersLogLevel::Trace => Level::TRACE,
            CrackersLogLevel::Debug => Level::DEBUG,
            CrackersLogLevel::Warn => Level::WARN,
            CrackersLogLevel::Info => Level::INFO,
            CrackersLogLevel::Error => Level::ERROR,
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
            log_level: CrackersLogLevel::Info,
        }
    }
}
