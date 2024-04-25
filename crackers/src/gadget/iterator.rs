use jingle::modeling::ModeledBlock;
use z3::Context;

use crate::gadget::library::GadgetLibrary;
use crate::gadget::signature::OutputSignature;

pub struct ModeledGadgetIterator<'a, 'ctx> {
    z3: &'ctx Context,
    library: &'a GadgetLibrary,
    offset: usize,
    spec_signature: OutputSignature,
}

impl<'a, 'ctx> ModeledGadgetIterator<'a, 'ctx> {
    pub fn new(z3: &'ctx Context, library: &'a GadgetLibrary, sig: OutputSignature) -> Self {
        Self {
            z3,
            library,
            offset: library.size(),
            spec_signature: sig,
        }
    }
}

impl<'a, 'ctx> Iterator for ModeledGadgetIterator<'a, 'ctx> {
    type Item = ModeledBlock<'ctx>;

    fn next(&mut self) -> Option<Self::Item> {
        for x in self.library.gadgets[0..self.offset].iter().rev() {
            self.offset -= 1;
            if OutputSignature::from(x).covers(&self.spec_signature) {
                let h =  ModeledBlock::read(self.z3, self.library, x.instructions.clone().into_iter());
                match h {
                    Ok(block) => return Some(block),
                    Err(e) => println!("{:?}", e)
                }
            }
        }
        None
    }
}
