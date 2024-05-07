use std::collections::HashMap;

use jingle::modeling::State;
use jingle::sleigh::{IndirectVarNode, SpaceManager, VarNode};
use jingle::varnode::{ResolvedIndirectVarNode, ResolvedVarnode};
use jingle::JingleError::UnmodeledSpace;
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
    pub register: Option<HashMap<String, i64>>,
    pub pointer: Option<HashMap<String, String>>,
    pub memory: Option<MemoryEqualityConstraint>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct MemoryEqualityConstraint {
    pub space: String,
    pub address: u64,
    pub size: usize,
    pub value: u8,
}

#[derive(Copy, Clone, Debug, Deserialize)]
pub struct PointerRangeConstraint {
    pub min: u64,
    pub max: u64,
}

pub fn gen_memory_constraint<'ctx>(
    m: MemoryEqualityConstraint,
) -> impl for<'a, 'b> Fn(&'a Context, &'b State<'a>) -> Result<Bool<'a>, CrackersError>
       + Send
       + Sync
       + Clone
       + 'static {
    return move |z3, state| {
        let data = state.read_varnode(&state.varnode(&m.space, m.address, m.size).unwrap())?;
        let constraint = data._eq(&BV::from_u64(z3, m.value as u64, data.get_size()));
        Ok(constraint)
    };
}

pub fn gen_register_constraint<'ctx>(
    vn: VarNode,
    value: u64,
) -> impl for<'a, 'b> Fn(&'a Context, &'b State<'a>) -> Result<Bool<'a>, CrackersError> + 'static + Send + Sync + Clone
{
    return move |z3, state| {
        let data = state.read_varnode(&vn)?;
        let constraint = data._eq(&BV::from_u64(z3, value, data.get_size()));
        Ok(constraint)
    };
}

pub fn gen_register_pointer_constraint<'ctx>(
    vn: VarNode,
    value: String,
    m: Option<PointerRangeConstraint>,
) -> impl for<'a, 'b> Fn(&'a Context, &'b State<'a>) -> Result<Bool<'a>, CrackersError> + 'ctx + Clone
{
    return move |z3, state| {
        let val = value
            .as_bytes()
            .iter()
            .map(|b| BV::from_u64(z3, *b as u64, 8))
            .reduce(|a, b| a.concat(&b))
            .unwrap();
        let pointer = state.read_varnode(&vn)?;
        let data = state.read_varnode_indirect(&IndirectVarNode {
            pointer_space_index: state.get_code_space_idx(),
            access_size_bytes: value.len(),
            pointer_location: vn.clone(),
        })?;
        let resolved = ResolvedVarnode::Indirect(ResolvedIndirectVarNode {
            pointer_space_idx: state.get_code_space_idx(),
            access_size_bytes: value.len(),
            pointer,
        });
        let mut constraint = data._eq(&val);
        if let Some(c) = &m {
            let callback = gen_pointer_range_invariant(c.clone());
            let cc = callback(z3, &resolved, &state)?;
            if let Some(b) = cc {
                constraint = Bool::and(z3, &[constraint, b])
            }
        }
        Ok(constraint)
    };
}

pub fn gen_pointer_range_invariant<'ctx>(
    m: PointerRangeConstraint,
) -> impl for<'a, 'b> Fn(
    &'a Context,
    &'b ResolvedVarnode<'a>,
    &'b State<'a>,
) -> Result<Option<Bool<'a>>, CrackersError>
       + 'ctx
       + Clone {
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
