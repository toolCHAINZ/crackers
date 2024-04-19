use std::fmt::{Debug, Display};

use jingle::sleigh::{Instruction, SpaceManager};
use serde::{Deserialize, Serialize};

mod error;
mod iterator;
pub mod signature;
pub mod library;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Gadget {
    pub instructions: Vec<Instruction>,
}

impl Gadget {
    pub fn address(&self) -> Option<u64> {
        self.instructions.first().map(|f| f.address)
    }
}
