use std::{collections::HashMap, ops::Range};

use jingle::{modeling::State, sleigh::VarNode};
use z3::ast::Bool;

use crate::error::CrackersError;

#[derive(Debug, Clone, Default)]
pub struct MemoryValuation(pub(super) HashMap<VarNode, Vec<u8>>);

impl MemoryValuation {
    pub fn to_constraint(&self) -> impl Fn(&State) -> Result<Bool, CrackersError> {
        let map = self.0.clone();
        move |state| {
            let mut v = vec![];
            for (vn, value) in &map {
                let mut temp_vn: VarNode = VarNode {
                    space_index: vn.space_index,
                    size: 1,
                    offset: vn.offset,
                };
                let r: Range<u64> = vn.into();
                for (index, offset) in r.enumerate() {
                    temp_vn.offset = offset;
                    v.push(state.read_varnode(&temp_vn)?.eq(value[index]))
                }
            }
            Ok(Bool::and(&v))
        }
    }
}
