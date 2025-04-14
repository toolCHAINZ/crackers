use crate::decision::assignment_model::PythonAssignmentModel;
use crate::decision::PythonDecisionResult;
use crackers::error::CrackersError;
use crackers::synthesis::builder::SynthesisParams;
use crackers::synthesis::DecisionResult;
use jingle::modeling::State;
use jingle::python::state::PythonState;
use jingle::python::z3::ast::TryFromPythonZ3;
use jingle::python::z3::get_python_z3;
use jingle::JingleContext;
use pyo3::{pyclass, pymethods, Py, PyAny, PyResult, Python};
use std::mem;
use std::sync::Arc;
use z3::ast::Bool;

#[pyclass]
#[derive(Clone)]
pub struct PythonSynthesisParams {
    pub inner: SynthesisParams,
}

#[pymethods]
impl PythonSynthesisParams {
    pub fn run(&self) -> PyResult<PythonDecisionResult> {
        Python::with_gil(|py| {
            py.allow_threads(|| {
                let z3 = get_python_z3()?;
                let res = match self.inner.combine_instructions {
                    false => self.inner.build_single(z3)?.decide()?,
                    true => self.inner.build_combined(z3)?.decide()?,
                };
                let res = match res {
                    DecisionResult::AssignmentFound(a) => Ok(PythonDecisionResult::AssignmentFound(
                        PythonAssignmentModel { inner: Arc::new(a) },
                    )),
                    DecisionResult::Unsat(u) => Ok(PythonDecisionResult::Unsat(u)),
                };
                res
            })
        })
    }

    pub fn add_precondition(&mut self, obj: Py<PyAny>) {
        let closure: Arc<PythonStateConstraintGenerator> = Arc::new(move |jingle, s, a| {
            Python::with_gil(|py| {
                let state = PythonState { state: s.clone() };
                println!("Hello!");
                let res = dbg!(obj.call(py, (state, a), None))?;
                println!("Called");
                let bool = Bool::try_from_python(res, jingle.z3).map_err(|e| CrackersError::PythonError(e))?;
                Ok(bool)
            })
        });
        self.inner
            .preconditions
            .push(unsafe { mem::transmute(closure) });
    }
}

pub type PythonStateConstraintGenerator = dyn Fn(&JingleContext<'static>, &State<'static>, u64) -> Result<Bool<'static>, CrackersError>
    + Send
    + Sync
    + 'static;
