use jingle::modeling::ModeledBlock;
use z3::Context;

use crate::gadget::GadgetLibrary;
use crate::gadget::signature::OutputSignature;

pub struct GadgetIterator<'a, 'ctx> {
    z3: &'ctx Context,
    library: &'a GadgetLibrary,
    offset: usize,
    spec_signature: OutputSignature,
}

impl<'a, 'ctx> GadgetIterator<'a, 'ctx> {
    pub fn new(z3: &'ctx Context, library: &'a GadgetLibrary, sig: OutputSignature) -> Self {
        Self {
            z3,
            library,
            offset: 0,
            spec_signature: sig,
        }
    }
}

impl<'a, 'ctx> Iterator for GadgetIterator<'a, 'ctx> {
    type Item = ModeledBlock<'ctx>;

    fn next(&mut self) -> Option<Self::Item> {
        for x in self.library.gadgets[self.offset..].iter() {
            if OutputSignature::from(x).may_cover(&self.spec_signature) {
                if let Ok(block) = self.library.model_gadget(self.z3, x) {
                    return Some(block);
                } else {
                    self.offset += 1;
                }
            } else {
                self.offset += 1;
            }
        }
        None
    }
}
