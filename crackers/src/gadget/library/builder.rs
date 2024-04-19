use jingle::sleigh::context::SleighContext;
use jingle::JingleError;

use crate::gadget::library::GadgetLibrary;

pub struct GadgetLibraryBuilder {
    max_gadget_length: usize,
}

impl GadgetLibraryBuilder {
    pub fn max_gadget_length(mut self, l: &usize) -> Self {
        self.max_gadget_length = *l;
        self
    }

    pub fn build(&self, sleigh: &SleighContext) -> Result<GadgetLibrary, JingleError> {
        GadgetLibrary::build_from_image(sleigh, self.max_gadget_length)
    }
}

impl Default for GadgetLibraryBuilder {
    fn default() -> Self {
        Self {
            max_gadget_length: 4,
        }
    }
}
