mod model_varnode_iterator;

use crate::decision::assignment_model::model_varnode_iterator::ModelVarNodeIterator;
use crackers::synthesis::assignment_model::AssignmentModel;
use jingle::modeling::{ModeledBlock, ModelingContext, State};
use jingle::python::modeled_block::PythonModeledBlock;
use jingle::python::resolved_varnode::PythonResolvedVarNode;
use jingle::python::state::PythonState;
use jingle::python::varode_iterator::VarNodeIterator;
use jingle::python::z3::ast::{TryFromPythonZ3, TryIntoPythonZ3};
use jingle::sleigh::{SpaceType, VarNode, VarNodeDisplay};
use jingle::varnode::{ResolvedIndirectVarNode, ResolvedVarnode};
use pyo3::exceptions::PyRuntimeError;
use pyo3::{Py, PyAny, PyResult, pyclass, pymethods};
use std::rc::Rc;
use z3::ast::BV;

#[pyclass(unsendable, name = "AssignmentModel")]
#[derive(Clone)]
pub struct PythonAssignmentModel {
    pub inner: Rc<AssignmentModel<'static, ModeledBlock<'static>>>,
}

impl PythonAssignmentModel {
    fn eval_vn(
        &self,
        state: &State<'static>,
        vn: PythonResolvedVarNode,
        completion: bool,
    ) -> Option<(String, BV<'static>)> {
        match vn {
            PythonResolvedVarNode::Direct(a) => {
                let bv = state.read_varnode(&VarNode::from(a.clone())).ok()?;
                let val = self.inner.model().eval(&bv, completion)?;
                Some((format!("{}", a), val))
            }
            PythonResolvedVarNode::Indirect(i) => {
                let pointer_value = self.inner.model().eval(&i.inner.pointer, completion)?;
                let space_name = i.inner.pointer_space_info.name.clone();
                let access_size = i.inner.access_size_bytes;
                let pointed_value = self.inner.model().eval(
                    &state
                        .read_resolved(&ResolvedVarnode::Indirect(ResolvedIndirectVarNode {
                            access_size_bytes: i.inner.access_size_bytes,
                            pointer_location: i.inner.pointer_location,
                            pointer: i.inner.pointer,
                            pointer_space_idx: i.inner.pointer_space_info.index,
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
                if let PythonResolvedVarNode::Direct(VarNodeDisplay::Raw(r)) = a {
                    r.space_info._type == SpaceType::IPTR_PROCESSOR
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
                if let PythonResolvedVarNode::Direct(VarNodeDisplay::Raw(r)) = a {
                    r.space_info._type == SpaceType::IPTR_PROCESSOR
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
