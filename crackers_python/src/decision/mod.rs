use crate::decision::assignment_model::PythonAssignmentModel;
use crackers::synthesis::selection_strategy::SelectionFailure;
use pyo3::pyclass;

pub mod assignment_model;

#[pyclass(unsendable)]
pub enum PythonDecisionResult {
    AssignmentFound(PythonAssignmentModel),
    Unsat(SelectionFailure),
}

unsafe impl Send for PythonDecisionResult {}