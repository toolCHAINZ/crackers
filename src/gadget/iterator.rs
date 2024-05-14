use jingle::modeling::{ModeledBlock, ModeledInstruction, ModelingContext};
use jingle::sleigh::Instruction;
use z3::{Context, SatResult, Solver};

use crate::error::CrackersError;
use crate::gadget::Gadget;
use crate::gadget::library::GadgetLibrary;
use crate::gadget::signature::OutputSignature;

pub struct GadgetIterator<'a, 'ctx> {
    z3: &'ctx Context,
    solver: Solver<'ctx>,
    library: &'a GadgetLibrary,
    offset: usize,
    instr: ModeledInstruction<'ctx>,
}

impl<'a, 'ctx> GadgetIterator<'a, 'ctx> {
    pub fn new(
        z3: &'ctx Context,
        library: &'a GadgetLibrary,
        sig: Instruction,
    ) -> Result<Self, CrackersError> {
        Ok(Self {
            z3,
            library,
            solver: Solver::new_for_logic(z3, "QF_ABV").unwrap(),
            offset: 0,
            instr: ModeledInstruction::new(sig, library, z3)?,
        })
    }
}

impl<'a, 'ctx> Iterator for GadgetIterator<'a, 'ctx> {
    type Item = &'a Gadget;

    fn next(&mut self) -> Option<Self::Item> {
        for x in self.library.gadgets[self.offset..].iter() {
            let syscall_cond = !self.instr.instr.has_syscall()
                || x.instructions
                    .iter()
                    .any(|i| i.ops_equal(&self.instr.instr));
            if OutputSignature::from(x).covers(&OutputSignature::from(&self.instr.instr))
                && syscall_cond
            {
                let h =
                    ModeledBlock::read(self.z3, self.library, x.instructions.clone().into_iter())
                        .ok()?;
                let isolated_check = h.refines(&self.instr).ok()?;
                if self.solver.check_assumptions(&[isolated_check]) == SatResult::Sat {
                    return Some(x);
                }
            }
        }
        None
    }
}
