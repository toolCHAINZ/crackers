use jingle::sleigh::Instruction;
use std::fmt::{Display, Formatter, LowerExp, LowerHex};

#[derive(Debug, Clone)]
pub struct Step {
    instructions: Vec<Instruction>,
}

impl Step {
    pub fn new<'a, T: Iterator<Item=&'a Instruction>>(instructions: T) -> Self {
        Self { instructions: instructions.cloned().collect() }
    }

    pub fn combine<'a, T: Iterator<Item=&'a Step>>(steps: T) -> Self{
        let instructions = steps.map(|step| step.instructions.clone()).flatten().collect();
        Self { instructions }
    }
    pub fn from_instr(instr: Instruction) -> Self {
        Self { instructions: vec![instr] }
    }
    
    pub fn instructions(&self) -> &[Instruction] {
        &self.instructions
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
