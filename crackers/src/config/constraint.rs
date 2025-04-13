use crate::error::CrackersError;
use crate::synthesis::builder::{StateConstraintGenerator, TransitionConstraintGenerator};
use jingle::JingleContext;
use jingle::modeling::{ModeledBlock, ModelingContext, State};
use jingle::sleigh::{ArchInfoProvider, VarNode};
use jingle::varnode::{ResolvedIndirectVarNode, ResolvedVarnode};
#[cfg(feature = "pyo3")]
use pyo3::pyclass;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::ops::Add;
use std::sync::Arc;
use tracing::{Level, event};
use z3::ast::{Ast, BV, Bool};

#[derive(Clone, Debug, Deserialize, Serialize, Default)]
#[cfg_attr(feature = "pyo3", pyclass(get_all, set_all))]
pub struct ConstraintConfig {
    pub precondition: Option<StateEqualityConstraint>,
    pub postcondition: Option<StateEqualityConstraint>,
    pub pointer: Option<PointerRangeConstraints>,
}

impl ConstraintConfig {
    pub fn get_preconditions<'a, T: ArchInfoProvider>(
        &'a self,
        sleigh: &'a T,
    ) -> impl Iterator<Item = Arc<StateConstraintGenerator>> + 'a {
        self.precondition
            .iter()
            .flat_map(|c| c.constraints(sleigh, self.pointer.clone()))
    }

    pub fn get_postconditions<'a, T: ArchInfoProvider>(
        &'a self,
        sleigh: &'a T,
    ) -> impl Iterator<Item = Arc<StateConstraintGenerator>> + 'a {
        self.postcondition
            .iter()
            .flat_map(|c| c.constraints(sleigh, self.pointer.clone()))
    }

    pub fn get_pointer_constraints(
        &self,
    ) -> impl Iterator<Item = Arc<TransitionConstraintGenerator>> + '_ {
        self.pointer.iter().map(|c| c.constraints())
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[cfg_attr(feature = "pyo3", pyclass(get_all, set_all))]
pub struct StateEqualityConstraint {
    pub register: Option<HashMap<String, i64>>,
    pub pointer: Option<HashMap<String, String>>,
    pub memory: Option<MemoryEqualityConstraint>,
}

impl StateEqualityConstraint {
    pub fn constraints<'a, T: ArchInfoProvider>(
        &'a self,
        sleigh: &'a T,
        c: Option<PointerRangeConstraints>,
    ) -> impl Iterator<Item = Arc<StateConstraintGenerator>> + 'a {
        let register_iterator = self.register.iter().flat_map(|map| {
            map.iter().filter_map(|(name, value)| {
                if let Some(vn) = sleigh.get_register(name) {
                    Some(Arc::new(gen_register_constraint(vn.clone(), *value as u64))
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
            let c1 = c.clone();
            map.iter().filter_map(move |(name, value)| {
                if let Some(vn) = sleigh.get_register(name) {
                    Some(Arc::new(gen_register_pointer_constraint(
                        vn.clone(),
                        value.clone(),
                        c1.clone(),
                    )) as Arc<StateConstraintGenerator>)
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

#[derive(Clone, Debug, Deserialize, Serialize)]
#[cfg_attr(feature = "pyo3", pyclass(get_all, set_all))]
pub struct MemoryEqualityConstraint {
    pub space: String,
    pub address: u64,
    pub size: usize,
    pub value: u8,
}

#[derive(Clone, Debug, Deserialize, Serialize, Default)]
#[cfg_attr(feature = "pyo3", pyclass(get_all, set_all))]
pub struct PointerRangeConstraints {
    pub read: Option<Vec<PointerRange>>,
    pub write: Option<Vec<PointerRange>>,
}

impl PointerRangeConstraints {
    pub fn constraints(&self) -> Arc<TransitionConstraintGenerator> {
        Arc::new(gen_pointer_range_transition_invariant(self.clone()))
    }
}
#[derive(Copy, Clone, Debug, Deserialize, Serialize)]
#[cfg_attr(feature = "pyo3", pyclass(get_all, set_all))]
pub struct PointerRange {
    pub min: u64,
    pub max: u64,
}

/// Generates a state constraint that a given varnode must be equal to a given value
pub fn gen_memory_constraint(
    m: MemoryEqualityConstraint,
) -> impl for<'a> Fn(&JingleContext<'a>, &State<'a>, u64) -> Result<Bool<'a>, CrackersError>
+ Send
+ Sync
+ Clone
+ 'static {
    move |jingle, state, _addr| {
        let data = state.read_varnode(&state.varnode(&m.space, m.address, m.size).unwrap())?;
        let constraint = data._eq(&BV::from_u64(jingle.z3, m.value as u64, data.get_size()));
        Ok(constraint)
    }
}

/// Generates a state constraint that a given varnode must be equal to a given value
/// todo: can consolidate this with the above one I think
pub fn gen_register_constraint(
    vn: VarNode,
    value: u64,
) -> impl for<'a> Fn(&JingleContext<'a>, &State<'a>, u64) -> Result<Bool<'a>, CrackersError>
+ 'static
+ Send
+ Sync
+ Clone {
    move |jingle, state, _addr| {
        let data = state.read_varnode(&vn)?;
        let constraint = data._eq(&BV::from_u64(jingle.z3, value, data.get_size()));
        Ok(constraint)
    }
}

/// Generates a constraint enforcing that the given varnode contains a pointer into the default
/// code space, pointing to the provided string
pub fn gen_register_pointer_constraint<'ctx>(
    vn: VarNode,
    value: String,
    m: Option<PointerRangeConstraints>,
) -> impl for<'a> Fn(&JingleContext<'a>, &State<'a>, u64) -> Result<Bool<'a>, CrackersError> + 'ctx + Clone
{
    move |jingle, state, _addr| {
        let m = m.clone();
        let mut bools = vec![];
        let pointer = state.read_varnode(&vn)?;
        for (i, byte) in value.as_bytes().iter().enumerate() {
            let expected = BV::from_u64(jingle.z3, *byte as u64, 8);
            let char_ptr = ResolvedVarnode::Indirect(ResolvedIndirectVarNode {
                // dumb but whatever
                pointer_location: vn.clone(),
                pointer: pointer.clone().add(i as u64),
                access_size_bytes: 1,
                pointer_space_idx: state.get_code_space_idx(),
            });
            let actual = state.read_resolved(&char_ptr)?;
            bools.push(actual._eq(&expected))
        }
        let pointer = state.read_varnode(&vn)?;
        let resolved = ResolvedVarnode::Indirect(ResolvedIndirectVarNode {
            pointer_location: vn.clone(),
            pointer_space_idx: state.get_code_space_idx(),
            access_size_bytes: value.len(),
            pointer,
        });
        let mut constraint = Bool::and(jingle.z3, &bools);
        if let Some(c) = m.and_then(|m| m.read) {
            let callback = gen_pointer_range_state_invariant(c);
            let cc = callback(jingle, &resolved, state)?;
            if let Some(b) = cc {
                constraint = Bool::and(jingle.z3, &[constraint, b])
            }
        }
        Ok(constraint)
    }
}

/// Generates an invariant enforcing that the given varnode, read from a given state, is within
/// the given range.
pub fn gen_pointer_range_state_invariant<'ctx>(
    m: Vec<PointerRange>,
) -> impl for<'a> Fn(
    &JingleContext<'a>,
    &ResolvedVarnode<'a>,
    &State<'a>,
) -> Result<Option<Bool<'a>>, CrackersError>
+ 'ctx
+ Clone {
    move |jingle, vn, state| {
        match vn {
            ResolvedVarnode::Direct(d) => {
                // todo: this is gross
                let should_constrain = state.get_code_space_idx() == d.space_index;
                match should_constrain {
                    false => Ok(None),
                    true => {
                        let bool = m
                            .iter()
                            .any(|mm| d.offset >= mm.min && (d.offset + d.size as u64) <= mm.max);
                        Ok(Some(Bool::from_bool(jingle.z3, bool)))
                    }
                }
            }
            ResolvedVarnode::Indirect(vn) => {
                let mut terms = vec![];
                for mm in &m {
                    let min = BV::from_u64(jingle.z3, mm.min, vn.pointer.get_size());
                    let max = BV::from_u64(jingle.z3, mm.max, vn.pointer.get_size());
                    let constraint =
                        Bool::and(jingle.z3, &[vn.pointer.bvuge(&min), vn.pointer.bvule(&max)]);
                    terms.push(constraint);
                }

                Ok(Some(Bool::or(jingle.z3, terms.as_slice())))
            }
        }
    }
}

pub fn gen_pointer_range_transition_invariant(
    m: PointerRangeConstraints,
) -> impl for<'a> Fn(&JingleContext<'a>, &ModeledBlock<'a>) -> Result<Option<Bool<'a>>, CrackersError>
+ Send
+ Sync
+ Clone
+ 'static {
    move |jingle, block| {
        let mut bools = vec![];
        if let Some(r) = &m.read {
            let inv = gen_pointer_range_state_invariant(r.clone());
            for x in block.get_inputs() {
                if let Some(c) = inv(jingle, &x, block.get_final_state())? {
                    bools.push(c);
                }
            }
        }
        if let Some(r) = &m.write {
            let inv = gen_pointer_range_state_invariant(r.clone());
            for x in block.get_outputs() {
                if let Some(c) = inv(jingle, &x, block.get_final_state())? {
                    bools.push(c);
                }
            }
        }
        Ok(Some(Bool::and(jingle.z3, &bools)))
    }
}
