use crate::config::error::CrackersConfigError;
use jingle::JingleContext;
use jingle::modeling::ModeledInstruction;
use jingle::sleigh::Instruction;
use std::fmt::{Display, Formatter};

#[derive(Debug, Clone, Default)]
pub struct Step {
    instructions: Vec<Instruction>,
}

impl Step {
    pub fn new<'a, T: Iterator<Item = &'a Instruction>>(instructions: T) -> Self {
        Self {
            instructions: instructions.cloned().collect(),
        }
    }

    pub fn combine<'a, T: Iterator<Item = &'a Step>>(steps: T) -> Self {
        let instructions = steps.flat_map(|step| step.instructions.clone()).collect();
        Self { instructions }
    }
    pub fn from_instr(instr: Instruction) -> Self {
        Self {
            instructions: vec![instr],
        }
    }

    pub fn instructions(&self) -> &[Instruction] {
        &self.instructions
    }

    pub fn model<'ctx>(
        &self,
        ctx: &JingleContext<'ctx>,
    ) -> Result<ModeledInstruction<'ctx>, CrackersConfigError> {
        let i: Instruction = self.instructions.as_slice().try_into()?;
        ModeledInstruction::new(i, ctx).map_err(CrackersConfigError::from)
    }
}

impl Display for Step {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        for x in &self.instructions {
            writeln!(f, "{}", x.disassembly)?;
        }
        Ok(())
    }
}
