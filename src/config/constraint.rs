use std::collections::HashMap;

use jingle::JingleError::UnmodeledSpace;
use jingle::modeling::{ModeledBlock, ModelingContext, State};
use jingle::sleigh::{IndirectVarNode, SpaceManager, VarNode};
use jingle::varnode::{ResolvedIndirectVarNode, ResolvedVarnode};
use serde::Deserialize;
use z3::ast::{Ast, Bool, BV};
use z3::Context;

use crate::error::CrackersError;

#[derive(Debug, Deserialize)]
pub struct Constraint {
    pub precondition: Option<StateEqualityConstraint>,
    pub postcondition: Option<StateEqualityConstraint>,
    pub pointer: Option<PointerRangeConstraints>,
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
pub struct PointerRangeConstraints {
    pub read: Option<PointerRange>,
    pub write: Option<PointerRange>,
}
#[derive(Copy, Clone, Debug, Deserialize)]
pub struct PointerRange {
    pub min: u64,
    pub max: u64,
}

/// Generates a state constraint that a given varnode must be equal to a given value
pub fn gen_memory_constraint(
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

/// Generates a state constraint that a given varnode must be equal to a given value
/// todo: can consolidate this with the above one I think
pub fn gen_register_constraint(
    vn: VarNode,
    value: u64,
) -> impl for<'a, 'b> Fn(&'a Context, &'b State<'a>) -> Result<Bool<'a>, CrackersError>
       + 'static
       + Send
       + Sync
       + Clone {
    return move |z3, state| {
        let data = state.read_varnode(&vn)?;
        let constraint = data._eq(&BV::from_u64(z3, value, data.get_size()));
        Ok(constraint)
    };
}

/// Generates a constraint enforcing that the given varnode contains a pointer into the default
/// code space, pointing to the provided string
pub fn gen_register_pointer_constraint<'ctx>(
    vn: VarNode,
    value: String,
    m: Option<PointerRangeConstraints>,
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
        if let Some(c) = m.and_then(|m| m.read) {
            let callback = gen_pointer_range_state_invariant(c);
            let cc = callback(z3, &resolved, state)?;
            if let Some(b) = cc {
                constraint = Bool::and(z3, &[constraint, b])
            }
        }
        Ok(constraint)
    };
}

/// Generates an invariant enforcing that the given varnode, read from a given state, is within
/// the given range.
pub fn gen_pointer_range_state_invariant<'ctx>(
    m: PointerRange,
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

pub fn gen_pointer_range_transition_invariant(
    m: PointerRangeConstraints,
) -> impl for<'a, 'b> Fn(&'a Context, &'b ModeledBlock<'a>) -> Result<Option<Bool<'a>>, CrackersError>
       + Send
       + Sync
       + Clone
       + 'static {
    return move |z3, block| {
        let mut bools = vec![];
        if let Some(r) = m.read {
            let inv = gen_pointer_range_state_invariant(r);
            for x in block.get_inputs() {
                if let Some(c) = inv(z3, &x, block.get_final_state())? {
                    bools.push(c);
                }
            }
        }
        if let Some(r) = m.write {
            let inv = gen_pointer_range_state_invariant(r);
            for x in block.get_outputs() {
                if let Some(c) = inv(z3, &x, block.get_final_state())? {
                    bools.push(c);
                }
            }
        }
        Ok(Some(Bool::and(z3, &bools)))
    };
}
