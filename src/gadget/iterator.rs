use jingle::modeling::ModeledBlock;
use jingle::sleigh::Instruction;
use z3::Context;

use crate::gadget::library::GadgetLibrary;
use crate::gadget::signature::OutputSignature;

pub struct ModeledGadgetIterator<'a, 'ctx> {
    z3: &'ctx Context,
    library: &'a GadgetLibrary,
    offset: usize,
    instr: Instruction,
}

impl<'a, 'ctx> ModeledGadgetIterator<'a, 'ctx> {
    pub fn new(z3: &'ctx Context, library: &'a GadgetLibrary, sig: Instruction) -> Self {
        Self {
            z3,
            library,
            offset: library.size(),
            instr: sig,
        }
    }
}

impl<'a, 'ctx> Iterator for ModeledGadgetIterator<'a, 'ctx> {
    type Item = ModeledBlock<'ctx>;

    fn next(&mut self) -> Option<Self::Item> {
        for x in self.library.gadgets[0..self.offset].iter().rev() {
            self.offset -= 1;
            let syscall_cond = !self.instr.has_syscall()
                || x.instructions.iter().any(|i| i.ops_equal(&self.instr));
            if OutputSignature::from(x).covers(&OutputSignature::from(&self.instr)) && syscall_cond
            {
                let h =
                    ModeledBlock::read(self.z3, self.library, x.instructions.clone().into_iter());
                match h {
                    Ok(block) => return Some(block),
                    Err(e) => println!("{:?}", e),
                }
            }
        }
        None
    }
}
