use crate::decision::PythonDecisionResult;
use crate::decision::assignment_model::PythonAssignmentModel;
use crackers::synthesis::DecisionResult;
use crackers::synthesis::builder::SynthesisParams;
use jingle::python::z3::get_python_z3;
use pyo3::{Py, PyResult, Python, pyclass, pymethods};

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
