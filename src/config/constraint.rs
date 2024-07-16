use std::collections::HashMap;
use std::sync::Arc;

use jingle::modeling::{ModeledBlock, ModelingContext, State};
use jingle::sleigh::context::SleighContext;
use jingle::sleigh::{IndirectVarNode, RegisterManager, SpaceManager, VarNode};
use jingle::varnode::{ResolvedIndirectVarNode, ResolvedVarnode};
use jingle::JingleError::UnmodeledSpace;
use serde::Deserialize;
use tracing::{event, Level};
use z3::ast::{Ast, Bool, BV};
use z3::Context;

use crate::error::CrackersError;
use crate::synthesis::builder::{StateConstraintGenerator, TransitionConstraintGenerator};

#[derive(Clone, Debug, Deserialize)]
pub struct Constraint {
    pub precondition: Option<StateEqualityConstraint>,
    pub postcondition: Option<StateEqualityConstraint>,
    pub pointer: Option<PointerRangeConstraints>,
}

impl Constraint {
    pub fn get_preconditions<'a, T: SpaceManager + RegisterManager>(
        &'a self,
        sleigh: &'a T,
    ) -> impl Iterator<Item = Arc<StateConstraintGenerator>> + 'a {
        self.precondition
            .iter()
            .flat_map(|c| c.constraints(sleigh, self.pointer))
    }

    pub fn get_postconditions<'a, T: SpaceManager + RegisterManager>(
        &'a self,
        sleigh: &'a T,
    ) -> impl Iterator<Item = Arc<StateConstraintGenerator>> + 'a {
        self.postcondition
            .iter()
            .flat_map(|c| c.constraints(sleigh, self.pointer))
    }

    pub fn get_pointer_constraints(
        &self,
    ) -> impl Iterator<Item = Arc<TransitionConstraintGenerator>> + '_ {
        self.pointer.iter().map(|c| c.constraints())
    }
}

#[derive(Clone, Debug, Deserialize)]
pub struct StateEqualityConstraint {
    pub register: Option<HashMap<String, i64>>,
    pub pointer: Option<HashMap<String, String>>,
    pub memory: Option<MemoryEqualityConstraint>,
}

impl StateEqualityConstraint {
    pub fn constraints<'a, T: SpaceManager + RegisterManager>(
        &'a self,
        sleigh: &'a T,
        c: Option<PointerRangeConstraints>,
    ) -> impl Iterator<Item = Arc<StateConstraintGenerator>> + 'a {
        let c1 = c;
        let register_iterator = self.register.iter().flat_map(|map| {
            map.iter().filter_map(|(name, value)| {
                if let Some(vn) = sleigh.get_register(name) {
                    Some(Arc::new(gen_register_constraint(vn, *value as u64))
                        as Arc<StateConstraintGenerator>)
                } else {
                    event!(Level::WARN, "Unrecognized register name: {}", name);
                    None
                }
            })
        });
        let memory_iterator = self
            .memory
            .iter()
            .map(|c| Arc::new(gen_memory_constraint(c.clone())) as Arc<StateConstraintGenerator>);
        let pointer_iterator = self.pointer.iter().flat_map(move |map| {
            map.iter().filter_map(move |(name, value)| {
                if let Some(vn) = sleigh.get_register(name) {
                    Some(
                        Arc::new(gen_register_pointer_constraint(vn, value.clone(), c1))
                            as Arc<StateConstraintGenerator>,
                    )
                } else {
                    event!(Level::WARN, "Unrecognized register name: {}", name);
                    None
                }
            })
        });
        register_iterator
            .chain(memory_iterator)
            .chain(pointer_iterator)
    }
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

impl PointerRangeConstraints {
    pub fn constraints(&self) -> Arc<TransitionConstraintGenerator> {
        Arc::new(gen_pointer_range_transition_invariant(*self))
    }
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
