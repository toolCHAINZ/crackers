use crate::decision::assignment_model::PythonAssignmentModel;
use crate::decision::PythonDecisionResult;
use crackers::synthesis::builder::SynthesisParams;
use crackers::synthesis::DecisionResult;
use jingle::python::z3::get_python_z3;
use pyo3::{pyclass, pymethods, Py, PyResult, Python};

#[pyclass]
pub struct PythonSynthesisParams {
    pub inner: SynthesisParams,
}

#[pymethods]
impl PythonSynthesisParams {
    pub fn run(&self) -> PyResult<PythonDecisionResult> {
        let z3 = get_python_z3()?;
        let res = match self.inner.combine_instructions {
            false => self.inner.build_single(z3)?.decide()?,
            true => self.inner.build_combined(z3)?.decide()?,
        };
        let res = Python::with_gil(|py| -> PyResult<PythonDecisionResult> {
            match res {
                DecisionResult::AssignmentFound(a) => Ok(PythonDecisionResult::AssignmentFound(
                    Py::new(py, PythonAssignmentModel { inner: a })?,
                )),
                DecisionResult::Unsat(u) => Ok(PythonDecisionResult::Unsat(u)),
            }
        })?;
        Ok(res)
    }
}
