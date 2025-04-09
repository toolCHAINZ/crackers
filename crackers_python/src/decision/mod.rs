use pyo3::{pyclass, Py};
use crackers::synthesis::selection_strategy::SelectionFailure;
use crate::decision::assignment_model::PythonAssignmentModel;

pub mod assignment_model;

#[pyclass(unsendable)]
pub enum PythonDecisionResult{
    AssignmentFound(Py<PythonAssignmentModel>),
    Unsat(SelectionFailure)
}