#[cfg(feature = "pyo3")]
use pyo3::pyclass;
use pyo3::{pymethods, PyErr, PyResult};
use rand::random;
use serde::{Deserialize, Serialize};
use tracing::Level;
use crate::config::CrackersConfig;
use crate::config::error::CrackersConfigError;
use crate::error::CrackersError;

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

impl TryFrom<&str> for CrackersLogLevel {
    type Error = CrackersError;

    fn try_from(value: &str) -> Result<Self, CrackersError> {
        match value.trim().to_lowercase().as_str() {
            "trace" => Ok(CrackersLogLevel::Trace),
            "debug" => Ok(CrackersLogLevel::Debug),
            "warn" => Ok(CrackersLogLevel::Warn),
            "info" => Ok(CrackersLogLevel::Info),
            "error" => Ok(CrackersLogLevel::Error),
            _ => Err(CrackersError::Config(CrackersConfigError::InvalidLogLevel))
        }
    }
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

#[cfg(feature = "pyo3")]
#[pymethods]
impl MetaConfig {

    #[new]
    fn new() -> MetaConfig {
        MetaConfig::default()
    }
    #[getter]
    fn get_log_level(&self) -> CrackersLogLevel {
        self.log_level.clone()
    }

    #[setter]
    fn set_log_level(&mut self, log_level: &str) -> Result<(), PyErr> {
        let log_level = log_level.try_into()?;
        self.log_level = log_level;
        Ok(())
    }

    #[getter]
    fn get_seed(&self) -> i64 {
        self.seed
    }

    #[setter]
    fn set_seed(&mut self, seed: i64) {
        self.seed = seed;
    }
}
