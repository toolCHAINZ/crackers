use crate::decision::PythonDecisionResult;
use crate::decision::assignment_model::PythonAssignmentModel;
use crackers::error::CrackersError;
use crackers::synthesis::DecisionResult;
use crackers::synthesis::builder::{
    StateConstraintGenerator, SynthesisParams, TransitionConstraintGenerator,
};
use jingle::modeling::{ModeledBlock, State};
use jingle::python::modeled_block::PythonModeledBlock;
use jingle::python::state::PythonState;
use jingle::python::z3::ast::PythonAst;
use lazy_static::lazy_static;
use pyo3::{Py, PyAny, PyResult, Python, pyclass, pymethods};
use std::sync::{Arc, Mutex};
use z3::ast::Bool;

lazy_static! {
    static ref MUTEX: Arc<Mutex<()>> = Arc::new(Mutex::new(()));
}

#[pyclass(name = "SynthesisParams")]
#[derive(Clone)]
pub struct PythonSynthesisParams {
    pub inner: SynthesisParams,
}

#[pymethods]
impl PythonSynthesisParams {
    pub fn run(&self) -> PyResult<PythonDecisionResult> {
        let res = Python::with_gil(|py| {
            py.allow_threads(|| match self.inner.combine_instructions {
                false => self.inner.build_single()?.decide(),
                true => self.inner.build_combined()?.decide(),
            })
        })?;
        match res {
            DecisionResult::AssignmentFound(a) => {
                let a = a.build()?;
                let a = PythonAssignmentModel::try_from(a)?;
                Ok(PythonDecisionResult::AssignmentFound(a))
            }
            DecisionResult::Unsat(u) => Ok(PythonDecisionResult::Unsat(u)),
        }
    }

    pub fn add_precondition(&mut self, obj: Py<PyAny>) {
        let closure: Arc<PythonStateConstraintGenerator> = Arc::new(move |s, a| {
            let g = MUTEX.lock().unwrap();
            let r = Python::with_gil(|py| {
                let state = PythonState::from(s.clone());
                let res = obj.call(py, (state, a), None)?;
                let bool = Bool::try_from_python(res, py).map_err(CrackersError::PythonError)?;
                Ok(bool)
            });
            drop(g);
            r
        });
        let transmuted_closure: Arc<StateConstraintGenerator> = closure;
        self.inner.preconditions.push(transmuted_closure);
    }

    pub fn add_postcondition(&mut self, obj: Py<PyAny>) {
        let closure: Arc<PythonStateConstraintGenerator> = Arc::new(move |s, a| {
            let g = MUTEX.lock().unwrap();
            let r = Python::with_gil(|py| {
                let state = PythonState::from(s.clone());
                let res = obj.call(py, (state, a), None)?;
                let bool = Bool::try_from_python(res, py).map_err(CrackersError::PythonError)?;
                Ok(bool)
            });
            drop(g);
            r
        });
        let transmuted_closure: Arc<StateConstraintGenerator> = closure;
        self.inner.postconditions.push(transmuted_closure);
    }

    pub fn add_transition_constraint(&mut self, obj: Py<PyAny>) {
        let closure: Arc<PythonTransitionConstraintGenerator> = Arc::new(move |b| {
            let g = MUTEX.lock().unwrap();
            let r = Python::with_gil(|py| {
                let block = PythonModeledBlock { instr: b.clone() };
                let res = obj.call(py, (block,), None)?;
                if res.is_none(py) {
                    Ok(None)
                } else {
                    let bool =
                        Bool::try_from_python(res, py).map_err(CrackersError::PythonError)?;
                    Ok(Some(bool))
                }
            });
            drop(g);
            r
        });
        let transmuted_closure: Arc<TransitionConstraintGenerator> = closure;
        self.inner.pointer_invariants.push(transmuted_closure);
    }
}

pub type PythonStateConstraintGenerator =
    dyn Fn(&State, u64) -> Result<Bool, CrackersError> + Send + Sync + 'static;

pub type PythonTransitionConstraintGenerator =
    dyn Fn(&ModeledBlock) -> Result<Option<Bool>, CrackersError> + Send + Sync + 'static;
