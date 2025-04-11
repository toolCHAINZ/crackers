use jingle::sleigh::context::SleighContextBuilder;
#[cfg(feature = "pyo3")]
use pyo3::{pyclass, pymethods};
use serde::{Deserialize, Serialize};

use crate::config::error::CrackersConfigError;

#[derive(Clone, Debug, Deserialize, Serialize)]
#[cfg_attr(feature = "pyo3", pyclass(get_all, set_all))]
pub struct SleighConfig {
    pub ghidra_path: String,
}

impl SleighConfig {
    pub fn context_builder(&self) -> Result<SleighContextBuilder, CrackersConfigError> {
        let b = SleighContextBuilder::load_ghidra_installation(&self.ghidra_path)?;
        Ok(b)
    }
}

#[cfg(feature = "pyo3")]
#[pymethods]
impl SleighConfig {
    #[new]
    fn new(ghidra_path: String) -> Self {
        SleighConfig { ghidra_path }
    }

    #[getter]
    fn ghidra_path(&self) -> String {
        self.ghidra_path.clone()
    }

    #[setter]
    fn set_ghidra_path(&mut self, ghidra_path: String) {
        self.ghidra_path = ghidra_path;
    }
}
