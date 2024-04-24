use std::fmt::{Debug, Display};

use jingle::sleigh::{Instruction, SpaceManager};
use serde::{Deserialize, Serialize};

mod error;
mod iterator;
pub mod library;
pub mod signature;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Gadget {
    pub instructions: Vec<Instruction>,
}

impl Gadget {
    pub fn address(&self) -> Option<u64> {
        self.instructions.first().map(|f| f.address)
    }

    pub fn ops_equal(&self, other: &Self) -> bool {
        if self.instructions.len() != other.instructions.len() {
            false
        } else {
            self.instructions
                .iter()
                .zip(other.instructions.iter())
                .all(|(o, e)| o.ops_equal(e))
        }
    }
}
