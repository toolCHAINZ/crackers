use crate::error::CrackersError;
use crate::synthesis::builder::{StateConstraintGenerator, TransitionConstraintGenerator};
use jingle::modeling::{ModeledBlock, ModelingContext, State};
use jingle::sleigh::{SleighArchInfo, VarNode};
use jingle::varnode::{ResolvedIndirectVarNode, ResolvedVarnode};
#[cfg(feature = "pyo3")]
use pyo3::pyclass;
use serde::{Deserialize, Serialize};
use std::borrow::Borrow;
use std::collections::HashMap;
use std::ops::Add;
use std::sync::Arc;
use tracing::{Level, event};
use z3::ast::{BV, Bool};

#[derive(Clone, Debug, Deserialize, Serialize, Default)]
#[cfg_attr(feature = "pyo3", pyclass(get_all, set_all))]
pub struct ConstraintConfig {
    pub precondition: Option<StateEqualityConstraint>,
    pub postcondition: Option<StateEqualityConstraint>,
    pub pointer: Option<PointerRangeConstraints>,
}

impl ConstraintConfig {
    pub fn get_preconditions<T: Borrow<SleighArchInfo>>(
        &self,
        sleigh: T,
    ) -> impl Iterator<Item = Arc<StateConstraintGenerator>> {
        let sleigh = sleigh.borrow().clone();
        self.precondition
            .iter()
            .flat_map(move |c| c.constraints(sleigh.clone(), self.pointer.clone()))
    }

    pub fn get_postconditions<T: Borrow<SleighArchInfo>>(
        &self,
        sleigh: T,
    ) -> impl Iterator<Item = Arc<StateConstraintGenerator>> {
        self.postcondition
            .iter()
            .flat_map(move |c| c.constraints(sleigh.borrow().clone(), self.pointer.clone()))
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
    pub fn constraints<T: Borrow<SleighArchInfo>>(
        &self,
        info: T,
        c: Option<PointerRangeConstraints>,
    ) -> impl Iterator<Item = Arc<StateConstraintGenerator>> {
        let info = info.borrow().clone();
        let info2 = info.clone();
        let register_iterator = self.register.iter().flat_map(move |map| {
            let info = info.clone();
            map.iter().filter_map(move |(name, value)| {
                if let Some(vn) = info.register(name) {
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
        let info = info2;
        let pointer_iterator = self.pointer.iter().flat_map(move |map| {
            let c1 = c.clone();
            let info = info.clone();
            map.iter().filter_map(move |(name, value)| {
                if let Some(vn) = info.register(name) {
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
) -> impl Fn(&State, u64) -> Result<Bool, CrackersError> + Send + Sync + Clone + 'static {
    move |state, _addr| {
        let data = state.read_varnode(
            &state
                .arch_info()
                .varnode(&m.space, m.address, m.size)
                .unwrap(),
        )?;
        let constraint = data.eq(BV::from_u64(m.value as u64, data.get_size()));
        Ok(constraint)
    }
}

/// Generates a state constraint that a given varnode must be equal to a given value
/// todo: can consolidate this with the above one I think
pub fn gen_register_constraint(
    vn: VarNode,
    value: u64,
) -> impl Fn(&State, u64) -> Result<Bool, CrackersError> + 'static + Send + Sync + Clone {
    move |state, _addr| {
        let data = state.read_varnode(&vn)?;
        let constraint = data.eq(BV::from_u64(value, data.get_size()));
        Ok(constraint)
    }
}

/// Generates a constraint enforcing that the given varnode contains a pointer into the default
/// code space, pointing to the provided string
pub fn gen_register_pointer_constraint(
    vn: VarNode,
    value: String,
    m: Option<PointerRangeConstraints>,
) -> impl Fn(&State, u64) -> Result<Bool, CrackersError> + Clone {
    move |state, _addr| {
        let m = m.clone();
        let mut bools = vec![];
        let pointer = state.read_varnode(&vn)?;
        for (i, byte) in value.as_bytes().iter().enumerate() {
            let expected = BV::from_u64(*byte as u64, 8);
            let char_ptr = ResolvedVarnode::Indirect(ResolvedIndirectVarNode {
                // dumb but whatever
                pointer_location: vn.clone(),
                pointer: pointer.clone().add(i as u64),
                access_size_bytes: 1,
                pointer_space_idx: state.get_default_code_space_info().index,
            });
            let actual = state.read_resolved(&char_ptr)?;
            bools.push(actual.eq(&expected))
        }
        let pointer = state.read_varnode(&vn)?;
        let resolved = ResolvedVarnode::Indirect(ResolvedIndirectVarNode {
            pointer_location: vn.clone(),
            pointer_space_idx: state.get_default_code_space_info().index,
            access_size_bytes: value.len(),
            pointer,
        });
        let mut constraint = Bool::and(&bools);
        if let Some(c) = m.and_then(|m| m.read) {
            let callback = gen_pointer_range_state_invariant(c);
            let cc = callback(&resolved, state)?;
            if let Some(b) = cc {
                constraint = Bool::and(&[constraint, b])
            }
        }
        Ok(constraint)
    }
}

/// Generates an invariant enforcing that the given varnode, read from a given state, is within
/// the given range.
pub fn gen_pointer_range_state_invariant(
    m: Vec<PointerRange>,
) -> impl Fn(&ResolvedVarnode, &State) -> Result<Option<Bool>, CrackersError> + Clone {
    move |vn, state| {
        match vn {
            ResolvedVarnode::Direct(d) => {
                // todo: this is gross
                let should_constrain =
                    state.arch_info().default_code_space_index() == d.space_index;
                match should_constrain {
                    false => Ok(None),
                    true => {
                        let bool = m
                            .iter()
                            .any(|mm| d.offset >= mm.min && (d.offset + d.size as u64) <= mm.max);
                        Ok(Some(Bool::from_bool(bool)))
                    }
                }
            }
            ResolvedVarnode::Indirect(vn) => {
                let mut terms = vec![];
                for mm in &m {
                    let min = BV::from_u64(mm.min, vn.pointer.get_size());
                    let max = BV::from_u64(mm.max, vn.pointer.get_size());
                    let constraint = Bool::and(&[vn.pointer.bvuge(&min), vn.pointer.bvule(&max)]);
                    terms.push(constraint);
                }

                Ok(Some(Bool::or(terms.as_slice())))
            }
        }
    }
}

pub fn gen_pointer_range_transition_invariant(
    m: PointerRangeConstraints,
) -> impl Fn(&ModeledBlock) -> Result<Option<Bool>, CrackersError> + Send + Sync + Clone + 'static {
    move |block| {
        let mut bools = vec![];
        if let Some(r) = &m.read {
            let inv = gen_pointer_range_state_invariant(r.clone());
            for x in block.get_inputs() {
                if let Some(c) = inv(&x, block.get_final_state())? {
                    bools.push(c);
                }
            }
        }
        if let Some(r) = &m.write {
            let inv = gen_pointer_range_state_invariant(r.clone());
            for x in block.get_outputs() {
                if let Some(c) = inv(&x, block.get_final_state())? {
                    bools.push(c);
                }
            }
        }
        Ok(Some(Bool::and(&bools)))
    }
}
