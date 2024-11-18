use std::collections::HashSet;
use std::fmt::{Debug, Display, Formatter};

use jingle::modeling::ModeledBlock;
use jingle::sleigh::{Instruction, OpCode, PcodeOperation, SpaceInfo, SpaceManager};
use serde::{Deserialize, Serialize};
use z3::Context;

use crate::error::CrackersError;

mod another_iterator;
pub mod candidates;
mod error;
// mod iterator;
pub mod library;
pub mod signature;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Gadget {
    // todo: This is obviously not ideal, but it's not that much extra data and
    // I'd rather not deal with another lifetime
    spaces: Vec<SpaceInfo>,
    code_space_idx: usize,
    pub instructions: Vec<Instruction>,
}

impl Gadget {
    pub fn address(&self) -> u64 {
        self.instructions.first().map(|f| f.address).unwrap()
    }

    pub fn ops(&self) -> impl Iterator<Item = &PcodeOperation> {
        self.instructions.iter().flat_map(|i| i.ops.iter())
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

    pub fn model<'ctx>(&self, z3: &'ctx Context) -> Result<ModeledBlock<'ctx>, CrackersError> {
        let blk = ModeledBlock::read(z3, self, self.instructions.clone().into_iter())?;
        Ok(blk)
    }
}

impl SpaceManager for Gadget {
    fn get_space_info(&self, idx: usize) -> Option<&SpaceInfo> {
        self.spaces.get(idx)
    }

    fn get_all_space_info(&self) -> &[SpaceInfo] {
        self.spaces.as_slice()
    }

    fn get_code_space_idx(&self) -> usize {
        self.code_space_idx
    }
}

impl Display for Gadget {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        for x in &self.instructions {
            writeln!(f, "{:x}\t{}", x.address, x.disassembly)?;
        }
        Ok(())
    }
}
