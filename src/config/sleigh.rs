use jingle::sleigh::context::SleighContextBuilder;
use serde::Deserialize;

use crate::config::error::CrackersConfigError;

#[derive(Clone, Debug, Deserialize)]
pub struct SleighConfig {
    pub ghidra_path: String,
}

impl SleighConfig {
    pub fn context_builder(&self) -> Result<SleighContextBuilder, CrackersConfigError> {
        let b = SleighContextBuilder::load_ghidra_installation(&self.ghidra_path)?;
        Ok(b)
    }
}
