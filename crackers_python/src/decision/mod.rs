use crate::decision::assignment_model::PythonAssignmentModel;
use crackers::synthesis::selection_strategy::SelectionFailure;
use pyo3::{pyclass, Py};

pub mod assignment_model;

#[pyclass(unsendable)]
pub enum PythonDecisionResult {
    AssignmentFound(Py<PythonAssignmentModel>),
    Unsat(SelectionFailure),
}
