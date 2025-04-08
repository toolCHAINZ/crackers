#[cfg(feature = "pyo3")]
use pyo3::pyclass;
use rand::random;
use serde::{Deserialize, Serialize};
use tracing::Level;

#[derive(Copy, Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "UPPERCASE")]
#[cfg_attr(feature = "pyo3", pyclass)]
pub enum CrackersLogLevel {
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

#[derive(Clone, Debug, Deserialize, Serialize)]
#[cfg_attr(feature = "pyo3", pyclass)]
pub struct MetaConfig {
    #[serde(default = "random")]
    pub seed: i64,
    pub log_level: CrackersLogLevel,
}

impl Default for MetaConfig {
    fn default() -> Self {
        Self {
            seed: random(),
            log_level: CrackersLogLevel::Info,
        }
    }
}
