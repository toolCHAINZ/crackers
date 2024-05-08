use std::collections::HashSet;
use std::fmt::Debug;

use jingle::sleigh::{Instruction, OpCode};
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

    pub fn has_blacklisted_op(&self, blacklist: &HashSet<OpCode>) -> bool {
        self.instructions
            .iter()
            .any(|i| i.ops.iter().any(|o| blacklist.contains(&o.opcode())))
    }
}
