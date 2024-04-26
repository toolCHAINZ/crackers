use std::collections::HashSet;

use jingle::JingleError;
use jingle::sleigh::context::SleighContext;
use jingle::sleigh::OpCode;

use crate::gadget::library::GadgetLibrary;

pub struct GadgetLibraryBuilder {
    pub(crate) max_gadget_length: usize,
    pub(crate)operation_blacklist: HashSet<OpCode>,
    pub(crate)random_sample_size: Option<usize>,
    pub(crate) random_sample_seed: Option<u64>
}

impl GadgetLibraryBuilder {
    pub fn max_gadget_length(mut self, l: usize) -> Self {
        self.max_gadget_length = l;
        self
    }

    pub fn random_sample_size(mut self, l: Option<usize>) -> Self {
        self.random_sample_size = l;
        self
    }

    pub fn random_sample_seed(mut self, l: Option<u64>) -> Self {
        self.random_sample_seed = l;
        self
    }
    pub fn build(&self, sleigh: &SleighContext) -> Result<GadgetLibrary, JingleError> {
        GadgetLibrary::build_from_image(sleigh, self)
    }
}

impl Default for GadgetLibraryBuilder {
    fn default() -> Self {
        Self {
            max_gadget_length: 4,
            operation_blacklist: HashSet::from([OpCode::CPUI_BRANCH, OpCode::CPUI_CALL, OpCode::CPUI_CBRANCH]),
            random_sample_size: None, random_sample_seed: None
        }
    }
}
