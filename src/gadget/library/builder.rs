use std::collections::HashSet;

use jingle::JingleError;
use jingle::sleigh::context::SleighContext;
use jingle::sleigh::OpCode;

use crate::gadget::library::GadgetLibrary;

pub struct GadgetLibraryBuilder {
    max_gadget_length: usize,
    operation_blacklist: HashSet<OpCode>
}

impl GadgetLibraryBuilder {
    pub fn max_gadget_length(mut self, l: &usize) -> Self {
        self.max_gadget_length = *l;
        self
    }

    pub fn build(&self, sleigh: &SleighContext) -> Result<GadgetLibrary, JingleError> {
        GadgetLibrary::build_from_image(sleigh, self.max_gadget_length, &self.operation_blacklist)
    }
}

impl Default for GadgetLibraryBuilder {
    fn default() -> Self {
        Self {
            max_gadget_length: 4,
            operation_blacklist: HashSet::from([OpCode::CPUI_BRANCH, OpCode::CPUI_CALL, OpCode::CPUI_CBRANCH])
        }
    }
}
