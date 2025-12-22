use std::collections::HashSet;
use std::fmt::{Debug, Display, Formatter};

use jingle::sleigh::OpCode;

use crate::config::error::CrackersConfigError;
use crate::config::sleigh::SleighConfig;
use crate::config::specification::SpecificationConfig;
use crate::reference_program::step::Step;
use crate::reference_program::valuation::MemoryValuation;

pub(crate) mod binary;
pub(crate) mod parsed_pcode;
pub(crate) mod step;
pub(crate) mod valuation;

#[derive(Debug, Clone, Default)]
pub struct ReferenceProgram {
    steps: Vec<Step>,
    initial_memory: MemoryValuation,
}

impl Display for ReferenceProgram {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        for (index, step) in self.steps.iter().enumerate() {
            writeln!(f, "Step {index}:")?;
            for x in step.instructions() {
                writeln!(f, "  {}", x.disassembly)?;
            }
        }
        Ok(())
    }
}

impl ReferenceProgram {
    pub fn try_load(
        spec: &SpecificationConfig,
        sleigh_config: &SleighConfig,
        blacklist: &HashSet<OpCode>,
        lang_id: &str,
    ) -> Result<Self, CrackersConfigError> {
        match spec {
            SpecificationConfig::BinaryFile(binary_file_specification) => {
                ReferenceProgram::try_load_binary(
                    binary_file_specification,
                    sleigh_config,
                    blacklist,
                )
            }
            SpecificationConfig::RawPcode(s) => {
                let sleigh = sleigh_config.context_builder()?.build(lang_id)?;
                ReferenceProgram::try_load_parsed_pcode(&sleigh, s)
            }
        }
    }
}
