use crate::config::error::CrackersConfigError;
use crate::config::error::CrackersConfigError::{SpecMissingStartSymbol, SpecMissingTextSection};
use crate::config::object::load_sleigh;
use crate::config::sleigh::SleighConfig;
use jingle::sleigh::context::SleighContext;
use jingle::sleigh::Instruction;
use object::{File, Object, ObjectSection, ObjectSymbol};
use serde::Deserialize;
use std::fs;

#[derive(Debug, Deserialize)]
pub struct SpecificationConfig {
    pub path: String,
    pub max_instructions: usize,
}

impl SpecificationConfig {
    pub fn load_sleigh(
        &self,
        sleigh_config: &SleighConfig,
    ) -> Result<SleighContext, CrackersConfigError> {
        load_sleigh(&self.path, sleigh_config)
    }

    pub fn get_spec(
        &self,
        sleigh_config: &SleighConfig,
    ) -> Result<Vec<Instruction>, CrackersConfigError> {
        let data = fs::read(&self.path)?;
        let gimli_file = File::parse(&*data)?;
        let sym = gimli_file
            .symbol_by_name("_start")
            .ok_or(SpecMissingStartSymbol)?;
        let section = gimli_file
            .section_by_name(".text")
            .ok_or(SpecMissingTextSection)?;
        let offset = sym.address() - section.address();
        let file_offset = offset + section.file_range().ok_or(SpecMissingTextSection)?.0;
        let sleigh = self.load_sleigh(sleigh_config)?;
        let instrs: Vec<Instruction> = sleigh.read(file_offset, self.max_instructions).collect();
        Ok(instrs)
    }
}
