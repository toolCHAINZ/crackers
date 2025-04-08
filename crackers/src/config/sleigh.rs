use jingle::sleigh::context::SleighContextBuilder;
use pyo3::pyclass;
use serde::{Deserialize, Serialize};

use crate::config::error::CrackersConfigError;

#[derive(Clone, Debug, Deserialize, Serialize)]
#[cfg_attr(feature = "pyo3", pyclass)]
pub struct SleighConfig {
    pub ghidra_path: String,
}

impl SleighConfig {
    pub fn context_builder(&self) -> Result<SleighContextBuilder, CrackersConfigError> {
        let b = SleighContextBuilder::load_ghidra_installation(&self.ghidra_path)?;
        Ok(b)
    }
}
