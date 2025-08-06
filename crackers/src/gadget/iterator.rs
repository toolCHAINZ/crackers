use jingle::modeling::{ModeledBlock, ModeledInstruction};
use jingle::sleigh::Instruction;
use tracing::{event, Level};
use z3::Context;

use crate::error::CrackersError;
use crate::gadget::library::GadgetLibrary;
use crate::gadget::signature::OutputSignature;
use crate::gadget::Gadget;

pub struct GadgetIterator<'a> {
    z3: & Context,
    library: &'a GadgetLibrary,
    offset: usize,
    instr: ModeledInstruction,
}

impl<'a> GadgetIterator<'a> {
    pub fn new(
        z3: & Context,
        library: &'a GadgetLibrary,
        sig: Instruction,
    ) -> Result<Self, CrackersError> {
        Ok(Self {
            z3,
            library,
            offset: 0,
            instr: ModeledInstruction::new(sig, library, z3)?,
        })
    }
}

impl<'a> Iterator for GadgetIterator<'a> {
    type Item = &'a Gadget;

    fn next(&mut self) -> Option<Self::Item> {
        for x in self.library.gadgets[self.offset..].iter() {
            self.offset += 1;
            let syscall_cond = !self.instr.instr.has_syscall()
                || x.instructions
                    .iter()
                    .any(|i| i.ops_equal(&self.instr.instr));
            if OutputSignature::from(x).covers(&OutputSignature::from(&self.instr.instr))
                && syscall_cond
            {
                match ModeledBlock::read(self.ctx(), self.library, x.instructions.clone().into_iter())
                {
                    Ok(h) => h,
                    Err(e) => {
                        event!(Level::TRACE, "{:?}", e);
                        continue;
                    }
                };
                return Some(x);
            }
        }
        None
    }
}
