use jingle::sleigh::context::SleighContext;
use jingle::sleigh::Instruction;
use serde::Deserialize;
use crate::config::error::CrackersConfigError;
use crate::config::object::load_sleigh;
use crate::config::sleigh::SleighConfig;
use crate::error::CrackersError;

#[derive(Debug, Deserialize)]
pub struct SpecificationConfig {
    pub path: String,
    pub max_instructions: usize,
}

impl SpecificationConfig{
    pub fn load_sleigh(&self, sleigh_config: &SleighConfig) -> Result<SleighContext, CrackersConfigError>{
        load_sleigh(&self.path, sleigh_config)
    }

    pub fn get_spec(&self, sleigh_config: &SleighConfig) -> Result<Vec<Instruction>, CrackersConfigError> {
        let sleigh = self.load_sleigh(sleigh_config)?;
        todo!()
    }
}