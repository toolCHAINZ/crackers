mod model_varnode_iterator;

use crate::decision::assignment_model::model_varnode_iterator::ModelVarNodeIterator;
use crackers::synthesis::assignment_model::AssignmentModel;
use jingle::display::{JingleDisplay, JingleDisplayable};
use jingle::modeling::{ModeledBlock, ModelingContext, State};
use jingle::python::modeled_block::PythonModeledBlock;
use jingle::python::state::PythonState;
use jingle::python::varode_iterator::VarNodeIterator;
use jingle::python::z3::ast::PythonAst;
use jingle::sleigh::SpaceType;
use jingle::varnode::{ResolvedIndirectVarNode, ResolvedVarnode};
use pyo3::exceptions::PyRuntimeError;
use pyo3::{Py, PyAny, PyErr, PyResult, Python, pyclass, pymethods};
use std::rc::Rc;
use z3::ast::BV;

#[pyclass(unsendable, name = "AssignmentModel")]
#[derive(Clone)]
pub struct PythonAssignmentModel {
    pub inner: Rc<AssignmentModel<ModeledBlock>>,
}

impl PythonAssignmentModel {
    fn eval_vn(
        &self,
        state: &State,
        vn: JingleDisplay<ResolvedVarnode>,
        completion: bool,
    ) -> Option<(String, BV)> {
        let info = vn.info();
        match vn.inner() {
            ResolvedVarnode::Direct(a) => {
                let bv = state.read_varnode(a).ok()?;
                let val = self.inner.model().eval(&bv, completion)?;
                let a = a.display(info);
                Some((format!("{a}"), val))
            }
            ResolvedVarnode::Indirect(i) => {
                let pointer_value = self.inner.model().eval(&i.pointer, completion)?;
                let space_name = info.get_space(i.pointer_space_idx).unwrap().name.clone();
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
        Ok(PythonAssignmentModel {
            inner: Rc::new(value),
        })
    }
}

#[pymethods]
impl PythonAssignmentModel {
    fn eval_bv(&self, bv: Py<PyAny>, model_completion: bool) -> PyResult<Py<PyAny>> {
        Python::attach(|py| {
            let bv = BV::try_from_python(bv, py)?;
            let val = self
                .inner
                .model()
                .eval(&bv, model_completion)
                .ok_or(PyRuntimeError::new_err("Could not eval model"))?;
            val.try_into_python(py)
        })
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
                if let ResolvedVarnode::Direct(b) = a.inner.inner() {
                    a.inner.info().get_space(b.space_index).unwrap()._type
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
                if let ResolvedVarnode::Direct(b) = a.inner.inner() {
                    a.inner.info().get_space(b.space_index).unwrap()._type
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
            .flat_map(|p| self.eval_vn(initial, p.inner, model_completion))
            .collect();
        Some(ModelVarNodeIterator::new(iter.into_iter()))
    }

    pub fn output_summary(&self, model_completion: bool) -> Option<ModelVarNodeIterator> {
        let initial = self.final_state()?;
        let initial = initial.state();
        let iter: Vec<_> = self
            .outputs()?
            .flat_map(|p| self.eval_vn(initial, p.inner, model_completion))
            .collect();
        Some(ModelVarNodeIterator::new(iter.into_iter()))
    }
}
