use crackers::synthesis::assignment_model::AssignmentModel;
use jingle::modeling::{ModeledBlock, ModelingContext};
use jingle::python::modeled_block::PythonModeledBlock;
use jingle::python::state::PythonState;
use pyo3::{pyclass, pymethods};

#[pyclass(unsendable)]
pub struct PythonAssignmentModel {
    pub inner: AssignmentModel<'static, ModeledBlock<'static>>,
}

#[pymethods]
impl PythonAssignmentModel {
    pub fn initial_state(&self) -> Option<PythonState> {
        self.inner.gadgets.first().map(|f| PythonState {
            state: f.get_original_state().clone(),
        })
    }

    pub fn final_state(&self) -> Option<PythonState> {
        self.inner.gadgets.last().map(|f| PythonState {
            state: f.get_final_state().clone(),
        })
    }

    pub fn gadgets(&self) -> Vec<PythonModeledBlock> {
        self.inner
            .gadgets
            .clone()
            .into_iter()
            .map(|g| PythonModeledBlock { instr: g })
            .collect()
    }
}
