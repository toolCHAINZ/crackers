use crate::decision::PythonDecisionResult;
use crate::decision::assignment_model::PythonAssignmentModel;
use crackers::error::CrackersError;
use crackers::synthesis::DecisionResult;
use crackers::synthesis::builder::{
    StateConstraintGenerator, SynthesisParams, TransitionConstraintGenerator,
};
use jingle::JingleContext;
use jingle::modeling::{ModeledBlock, State};
use jingle::python::modeled_block::PythonModeledBlock;
use jingle::python::state::PythonState;
use jingle::python::z3::ast::TryFromPythonZ3;
use jingle::python::z3::get_python_z3;
use pyo3::{Py, PyAny, PyResult, Python, pyclass, pymethods};
use std::rc::Rc;
use std::sync::Arc;
use z3::ast::Bool;

#[pyclass(name = "SynthesisParams")]
#[derive(Clone)]
pub struct PythonSynthesisParams {
    pub inner: SynthesisParams,
}

#[pymethods]
impl PythonSynthesisParams {
    pub fn run(&self) -> PyResult<PythonDecisionResult> {
        let res = Python::with_gil(|py| {
            py.allow_threads(|| {
                let z3 = get_python_z3()?;

                match self.inner.combine_instructions {
                    false => self.inner.build_single(z3)?.decide_single_threaded(),
                    true => self.inner.build_combined(z3)?.decide_single_threaded(),
                }
            })
        })?;
        match res {
            DecisionResult::AssignmentFound(a) => {
                let a = a.build(get_python_z3()?)?;
                Ok(PythonDecisionResult::AssignmentFound(
                    PythonAssignmentModel { inner: Rc::new(a) },
                ))
            }
            DecisionResult::Unsat(u) => Ok(PythonDecisionResult::Unsat(u)),
        }
    }

    pub fn add_precondition(&mut self, obj: Py<PyAny>) {
        let closure: Arc<PythonStateConstraintGenerator> = Arc::new(move |_, s, a| {
            let state = PythonState::from(s.clone());
            Python::with_gil(|py| {
                let res = obj.call(py, (state, a), None)?;
                let bool = Bool::try_from_python(res).map_err(CrackersError::PythonError)?;
                Ok(bool)
            })
        });
        let transmuted_closure: Arc<StateConstraintGenerator> =
            unsafe { std::mem::transmute(closure) };
        self.inner.preconditions.push(transmuted_closure);
    }

    pub fn add_postcondition(&mut self, obj: Py<PyAny>) {
        let closure: Arc<PythonStateConstraintGenerator> = Arc::new(move |_, s, a| {
            let state = PythonState::from(s.clone());
            Python::with_gil(|py| {
                let res = obj.call(py, (state, a), None)?;
                let bool = Bool::try_from_python(res).map_err(CrackersError::PythonError)?;
                Ok(bool)
            })
        });
        let transmuted_closure: Arc<StateConstraintGenerator> =
            unsafe { std::mem::transmute(closure) };
        self.inner.postconditions.push(transmuted_closure);
    }

    pub fn add_transition_constraint(&mut self, obj: Py<PyAny>) {
        let closure: Arc<PythonTransitionConstraintGenerator> = Arc::new(move |_, b| {
            let block = PythonModeledBlock { instr: b.clone() };
            Python::with_gil(|py| {
                let res = obj.call(py, (block,), None)?;
                if res.is_none(py) {
                    Ok(None)
                } else {
                    let bool = Bool::try_from_python(res).map_err(CrackersError::PythonError)?;
                    Ok(Some(bool))
                }
            })
        });
        let transmuted_closure: Arc<TransitionConstraintGenerator> =
            unsafe { std::mem::transmute(closure) };
        self.inner.pointer_invariants.push(transmuted_closure);
    }
}

pub type PythonStateConstraintGenerator = dyn Fn(&JingleContext<'static>, &State<'static>, u64) -> Result<Bool<'static>, CrackersError>
    + Send
    + Sync
    + 'static;

pub type PythonTransitionConstraintGenerator = dyn Fn(
        &JingleContext<'static>,
        &ModeledBlock<'static>,
    ) -> Result<Option<Bool<'static>>, CrackersError>
    + Send
    + Sync
    + 'static;
