use crackers::synthesis::assignment_model::AssignmentModel;
use jingle::modeling::{ModeledBlock, ModelingContext};
use jingle::python::modeled_block::PythonModeledBlock;
use jingle::python::state::PythonState;
use jingle::python::varode_iterator::VarNodeIterator;
use jingle::python::z3::ast::{TryFromPythonZ3, TryIntoPythonZ3};
use pyo3::exceptions::PyRuntimeError;
use pyo3::{pyclass, pymethods, Py, PyAny, PyResult};
use z3::ast::BV;

#[pyclass(unsendable)]
pub struct PythonAssignmentModel {
    pub inner: AssignmentModel<'static, ModeledBlock<'static>>,
}

#[pymethods]
impl PythonAssignmentModel {

    fn eval_bv(&self, bv: Py<PyAny>, model_completion: bool) -> PyResult<Py<PyAny>> {
        let bv = BV::try_from_python(bv)?;
        let val = self
            .inner
            .model()
            .eval(&bv, model_completion)
            .ok_or(PyRuntimeError::new_err("Could not eval model"))?;
        val.try_into_python()
    }

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

    pub fn inputs(&self) -> Option<VarNodeIterator> {
        let state = self.inner.initial_state()?;
        Some(VarNodeIterator::new(
            state.clone(),
            self.inner
                .gadgets
                .iter()
                .flat_map(|g| g.get_inputs().into_iter()),
        ))
    }

    pub fn outputs(&self) -> Option<VarNodeIterator> {
        let state = self.inner.initial_state()?;
        Some(VarNodeIterator::new(
            state.clone(),
            self.inner
                .gadgets
                .iter()
                .flat_map(|g| g.get_outputs().into_iter()),
        ))
    }
}
