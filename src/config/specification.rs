use std::fs;

use jingle::sleigh::context::loaded::LoadedSleighContext;
use jingle::sleigh::Instruction;
use object::{File, Object, ObjectSymbol};
use serde::{Deserialize, Serialize};

use crate::config::error::CrackersConfigError;
use crate::config::error::CrackersConfigError::{SpecMissingStartSymbol, SpecMissingTextSection};
use crate::config::object::load_sleigh_spec;
use crate::config::sleigh::SleighConfig;

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct SpecificationConfig {
    pub path: String,
    pub max_instructions: usize,
    pub base_address: Option<u64>,
}

impl SpecificationConfig {
    fn load_sleigh<'a>(
        &self,
        sleigh_config: &'a SleighConfig,
    ) -> Result<LoadedSleighContext<'a>, CrackersConfigError> {
        load_sleigh_spec(&self.path, sleigh_config)
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
        let _section = gimli_file
            .section_by_name(".text")
            .ok_or(SpecMissingTextSection)?;
        let mut sleigh = self.load_sleigh(sleigh_config)?;
        let mut addr = sym.address();
        if let Some(o) = self.base_address {
            sleigh.set_base_address(o);
            addr = addr.wrapping_add(o);
        }
        let instrs: Vec<Instruction> = sleigh
            .read_until_branch(addr, self.max_instructions)
            .collect();
        Ok(instrs)
    }
}
