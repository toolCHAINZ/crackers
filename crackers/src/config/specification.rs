use std::fs;

use jingle::sleigh::Instruction;
use jingle::sleigh::context::loaded::LoadedSleighContext;
use object::{File, Object, ObjectSymbol};
#[cfg(feature = "pyo3")]
use pyo3::{pyclass, pymethods};
use serde::{Deserialize, Serialize};

use crate::config::error::CrackersConfigError;
use crate::config::error::CrackersConfigError::{SpecMissingStartSymbol, SpecMissingTextSection};
use crate::config::object::load_sleigh_spec;
use crate::config::sleigh::SleighConfig;

#[derive(Clone, Debug, Deserialize, Serialize)]
#[cfg_attr(feature = "pyo3", pyclass(get_all, set_all))]
pub struct BinaryFileSpecification {
    pub path: String,
    pub max_instructions: usize,
    pub base_address: Option<u64>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[cfg_attr(feature = "pyo3", pyclass)]
pub enum SpecificationConfig {
    BinaryFile(BinaryFileSpecification),
    RawPcode(String),
}

#[cfg(feature = "pyo3")]
#[pymethods]
impl BinaryFileSpecification {
    #[new]
    fn new(path: String, max_instructions: usize, base_address: Option<u64>) -> Self {
        Self {
            path,
            max_instructions,
            base_address,
        }
    }

    #[getter]
    fn get_path(&self) -> String {
        self.path.clone()
    }

    #[setter]
    fn set_path(&mut self, path: String) {
        self.path = path;
    }

    #[getter]
    fn get_max_instructions(&self) -> usize {
        self.max_instructions
    }

    #[setter]
    fn set_max_instructions(&mut self, max_instructions: usize) {
        self.max_instructions = max_instructions;
    }

    #[getter]
    fn get_base_address(&self) -> Option<u64> {
        self.base_address
    }

    #[setter]
    fn set_base_address(&mut self, address: u64) {
        self.base_address = Some(address);
    }
}

impl BinaryFileSpecification {
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
