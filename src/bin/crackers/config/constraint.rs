use std::collections::HashMap;

use jingle::JingleError::UnmodeledSpace;
use jingle::modeling::State;
use jingle::sleigh::{SpaceManager, VarNode};
use jingle::varnode::ResolvedVarnode;
use serde::Deserialize;
use z3::ast::{Ast, Bool, BV};
use z3::Context;

use crackers::error::CrackersError;

#[derive(Debug, Deserialize)]
pub struct Constraint {
    pub precondition: Option<StateEqualityConstraint>,
    pub postcondition: Option<StateEqualityConstraint>,
    pub pointer: Option<PointerRangeConstraint>,
}

#[derive(Debug, Deserialize)]
pub struct StateEqualityConstraint {
    pub register: Option<HashMap<String, u64>>,
    pub memory: Option<MemoryEqualityConstraint>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct MemoryEqualityConstraint {
    pub space: String,
    pub address: u64,
    pub size: usize,
    pub value: u8,
}

#[derive(Clone, Debug, Deserialize)]
pub struct PointerRangeConstraint {
    pub min: u64,
    pub max: u64,
}

pub fn gen_memory_constraint<'ctx>(
    m: MemoryEqualityConstraint,
) -> impl Fn(&'ctx Context, &State<'ctx>) -> Result<Bool<'ctx>, CrackersError> + 'ctx {
    return move |z3, state| {
        let data = state.read_varnode(&state.varnode(&m.space, m.address, m.size).unwrap())?;
        let constraint = data._eq(&BV::from_u64(z3, m.value as u64, data.get_size()));
        Ok(constraint)
    };
}

pub fn gen_register_constraint<'ctx>(
    vn: VarNode,
    value: u64,
) -> impl Fn(&'ctx Context, &State<'ctx>) -> Result<Bool<'ctx>, CrackersError> + 'ctx {
    return move |z3, state| {
        let data = state.read_varnode(&vn)?;
        let constraint = data._eq(&BV::from_u64(z3, value, data.get_size()));
        Ok(constraint)
    };
}

pub fn gen_pointer_constraint<'ctx>(
    m: PointerRangeConstraint,
) -> impl Fn(
    &'ctx Context,
    &ResolvedVarnode<'ctx>,
    &State<'ctx>,
) -> Result<Option<Bool<'ctx>>, CrackersError>
       + 'ctx {
    return move |z3, vn, state| {
        match vn {
            ResolvedVarnode::Direct(d) => {
                // todo: this is gross
                let should_constrain = state
                    .get_space_info(d.space_index)
                    .ok_or(UnmodeledSpace)?
                    .name
                    .eq("ram");
                match should_constrain {
                    false => Ok(None),
                    true => {
                        let bool = d.offset >= m.min && (d.offset + d.size as u64) <= m.max;
                        Ok(Some(Bool::from_bool(z3, bool)))
                    }
                }
            }
            ResolvedVarnode::Indirect(vn) => {
                let min = BV::from_u64(z3, m.min, vn.pointer.get_size());
                let max = BV::from_u64(z3, m.max, vn.pointer.get_size());
                let constraint = Bool::and(z3, &[vn.pointer.bvuge(&min), vn.pointer.bvule(&max)]);
                Ok(Some(constraint))
            }
        }
    };
}
