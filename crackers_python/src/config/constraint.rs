use crackers::config::constraint::{
    ConstraintConfig, MemoryEqualityConstraint, PointerRange, PointerRangeConstraints,
    StateEqualityConstraint,
};
use pyo3::{pyclass, Py, PyErr, Python};
use std::collections::HashMap;

#[pyclass(get_all, set_all)]
pub struct PythonConstraintConfig {
    pub precondition: Py<PythonStateEqualityConstraint>,
    pub postcondition: Py<PythonStateEqualityConstraint>,
    pub pointer: Py<PythonPointerRangeConstraints>,
}

impl TryFrom<ConstraintConfig> for PythonConstraintConfig {
    type Error = PyErr;

    fn try_from(value: ConstraintConfig) -> Result<Self, Self::Error> {
        let precondition: PythonStateEqualityConstraint = value
            .precondition
            .and_then(|f| f.try_into().ok())
            .unwrap_or_default();
        let postcondition: PythonStateEqualityConstraint = value
            .postcondition
            .and_then(|f| f.try_into().ok())
            .unwrap_or_default();
        let pointer: PythonPointerRangeConstraints = value
            .pointer
            .and_then(|f| f.try_into().ok())
            .unwrap_or_default();
        Python::with_gil(|py| {
            Ok(Self {
                precondition: Py::new(py, precondition)?,
                postcondition: Py::new(py, postcondition)?,
                pointer: Py::new(py, pointer)?,
            })
        })
    }
}

#[derive(Default)]
#[pyclass(get_all)]
pub struct PythonStateEqualityConstraint {
    pub register: HashMap<String, i64>,
    pub pointer: HashMap<String, String>,
    #[pyo3(set)]
    pub memory: Option<Py<MemoryEqualityConstraint>>,
}



impl TryFrom<StateEqualityConstraint> for PythonStateEqualityConstraint {
    type Error = PyErr;

    fn try_from(value: StateEqualityConstraint) -> Result<Self, Self::Error> {
        Python::with_gil(|py| {
            let mem = value.memory.map(|f| Py::new(py, f).unwrap());
            Ok(Self {
                register: value.register.clone().unwrap_or_default(),
                pointer: value.pointer.clone().unwrap_or_default(),
                memory: mem,
            })
        })
    }
}

#[pyclass(get_all)]
#[derive(Default)]
pub struct PythonPointerRangeConstraints {
    pub read: Vec<Py<PointerRange>>,
    pub write: Vec<Py<PointerRange>>,
}

impl TryFrom<PointerRangeConstraints> for PythonPointerRangeConstraints {
    type Error = PyErr;
    fn try_from(value: PointerRangeConstraints) -> Result<Self, Self::Error> {
        Python::with_gil(|py| {
            let read: Result<Vec<Py<PointerRange>>, PyErr> = value
                .read
                .into_iter()
                .flatten()
                .map(|f| Py::new(py, f))
                .collect();
            let write: Result<Vec<Py<PointerRange>>, PyErr> = value
                .write
                .into_iter()
                .flatten()
                .map(|f| Py::new(py, f))
                .collect();
            Ok(Self {
                read: read?,
                write: write?,
            })
        })
    }
}
