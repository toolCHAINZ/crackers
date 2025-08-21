mod model_varnode_iterator;

use crate::decision::assignment_model::model_varnode_iterator::ModelVarNodeIterator;
use crackers::synthesis::assignment_model::AssignmentModel;
use jingle::modeling::{ModeledBlock, ModelingContext, State};
use jingle::python::modeled_block::PythonModeledBlock;
use jingle::python::resolved_varnode::{PythonResolvedVarNode, PythonResolvedVarNodeInner};
use jingle::python::state::PythonState;
use jingle::python::varode_iterator::VarNodeIterator;
use jingle::python::z3::ast::{PythonAst, TryFromPythonZ3, TryIntoPythonZ3};
use jingle::sleigh::{ArchInfoProvider, SpaceType};
use jingle::varnode::{ResolvedIndirectVarNode, ResolvedVarnode};
use pyo3::exceptions::PyRuntimeError;
use pyo3::{Py, PyAny, PyResult, pyclass, pymethods, PyErr};
use std::rc::Rc;
use jingle::python::z3::get_python_z3;
use z3::ast::BV;
use z3::Translate;

#[pyclass(unsendable, name = "AssignmentModel")]
#[derive(Clone)]
pub struct PythonAssignmentModel {
    pub inner: Rc<AssignmentModel<ModeledBlock>>,
}

impl PythonAssignmentModel {
    fn eval_vn(
        &self,
        state: &State,
        vn: PythonResolvedVarNode,
        completion: bool,
    ) -> Option<(String, BV)> {
        match vn.inner {
            PythonResolvedVarNodeInner::Direct(a) => {
                let bv = state.read_varnode(a.inner()).ok()?;
                let val = self.inner.model().eval(&bv, completion)?;
                Some((format!("{a}"), val))
            }
            PythonResolvedVarNodeInner::Indirect(i) => {
                let info = i.info();
                let i = i.inner();
                let pointer_value = self.inner.model().eval(&i.pointer, completion)?;
                let space_name = info
                    .get_space_info(i.pointer_space_idx)
                    .unwrap()
                    .name
                    .clone();
                let access_size = i.access_size_bytes;
                let pointed_value = self.inner.model().eval(
                    &state
                        .read_resolved(&ResolvedVarnode::Indirect(ResolvedIndirectVarNode {
                            access_size_bytes: i.access_size_bytes,
                            pointer_location: i.pointer_location.clone(),
                            pointer: i.pointer.clone(),
                            pointer_space_idx: i.pointer_space_idx,
                        }))
                        .ok()?,
                    completion,
                )?;
                Some((
                    format!("{space_name}[{pointer_value}]:{access_size:x}"),
                    pointed_value,
                ))
            }
        }
    }
}

impl TryFrom<AssignmentModel<ModeledBlock>> for PythonAssignmentModel {
    type Error = PyErr;

    fn try_from(value: AssignmentModel<ModeledBlock>) -> Result<Self, Self::Error> {
        let z3 = get_python_z3()?;
        let value = value.translate(&z3);
        Ok(PythonAssignmentModel {
            inner: Rc::new(value),
        })
    }
}

#[pymethods]
impl PythonAssignmentModel {
    fn eval_bv(&self, bv: Py<PyAny>, model_completion: bool) -> PyResult<Py<PyAny>> {
        let bv = BV::try_from_python(bv)?;
        let val = self
            .inner
            .model()
            .eval(&bv, model_completion)
            .ok_or(PyRuntimeError::new_err("Could not eval model"))?;
        val.try_into_python()
    }

    pub fn initial_state(&self) -> Option<PythonState> {
        self.inner
            .gadgets
            .first()
            .map(|f| PythonState::from(f.get_original_state().clone()))
    }

    pub fn final_state(&self) -> Option<PythonState> {
        self.inner
            .gadgets
            .last()
            .map(|f| PythonState::from(f.get_final_state().clone()))
    }

    pub fn gadgets(&self) -> Vec<PythonModeledBlock> {
        self.inner
            .gadgets
            .clone()
            .into_iter()
            .map(|g| PythonModeledBlock { instr: g })
            .collect()
    }

    pub fn inputs(&self) -> Option<VarNodeIterator> {
        let hi = self
            .gadgets()
            .into_iter()
            .flat_map(|g| g.get_input_vns().ok())
            .flatten()
            .filter(|a| {
                if let PythonResolvedVarNode {
                    inner: PythonResolvedVarNodeInner::Direct(a),
                } = a
                {
                    a.info()
                        .get_space_info(a.inner().space_index)
                        .unwrap()
                        ._type
                        == SpaceType::IPTR_PROCESSOR
                } else {
                    true
                }
            });
        Some(VarNodeIterator::new(hi))
    }

    pub fn outputs(&self) -> Option<VarNodeIterator> {
        let hi = self
            .gadgets()
            .into_iter()
            .flat_map(|g| g.get_output_vns().ok())
            .flatten()
            .filter(|a| {
                if let PythonResolvedVarNode {
                    inner: PythonResolvedVarNodeInner::Direct(a),
                } = a
                {
                    a.info()
                        .get_space_info(a.inner().space_index)
                        .unwrap()
                        ._type
                        == SpaceType::IPTR_PROCESSOR
                } else {
                    true
                }
            });
        Some(VarNodeIterator::new(hi))
    }

    pub fn input_summary(&self, model_completion: bool) -> Option<ModelVarNodeIterator> {
        let initial = self.initial_state()?;
        let initial = initial.state();
        let iter: Vec<_> = self
            .inputs()?
            .flat_map(|p| self.eval_vn(initial, p, model_completion))
            .collect();
        Some(ModelVarNodeIterator::new(iter.into_iter()))
    }

    pub fn output_summary(&self, model_completion: bool) -> Option<ModelVarNodeIterator> {
        let initial = self.final_state()?;
        let initial = initial.state();
        let iter: Vec<_> = self
            .outputs()?
            .flat_map(|p| self.eval_vn(initial, p, model_completion))
            .collect();
        Some(ModelVarNodeIterator::new(iter.into_iter()))
    }
}
