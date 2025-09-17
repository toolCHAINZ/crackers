use crackers::config::constraint::{
    ConstraintConfig, MemoryEqualityConstraint, PointerRange, PointerRangeConstraints,
    StateEqualityConstraint,
};
use pyo3::{Py, PyErr, Python, pyclass};
use std::collections::HashMap;

#[pyclass(get_all, set_all)]
#[derive(Clone)]
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
        Python::attach(|py| {
            Ok(Self {
                precondition: Py::new(py, precondition)?,
                postcondition: Py::new(py, postcondition)?,
                pointer: Py::new(py, pointer)?,
            })
        })
    }
}

impl TryFrom<PythonConstraintConfig> for ConstraintConfig {
    type Error = PyErr;

    fn try_from(value: PythonConstraintConfig) -> Result<Self, Self::Error> {
        Python::attach(|py| {
            let precondition = Some(value.precondition.borrow(py).clone().try_into()?);
            let postcondition = Some(value.postcondition.borrow(py).clone().try_into()?);
            let pointer = Some(value.pointer.borrow(py).clone().try_into()?);
            Ok(ConstraintConfig {
                precondition,
                postcondition,
                pointer,
            })
        })
    }
}

#[derive(Default, Clone)]
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
        Python::attach(|py| {
            let mem = value.memory.map(|f| Py::new(py, f).unwrap());
            Ok(Self {
                register: value.register.clone().unwrap_or_default(),
                pointer: value.pointer.clone().unwrap_or_default(),
                memory: mem,
            })
        })
    }
}

impl TryFrom<PythonStateEqualityConstraint> for StateEqualityConstraint {
    type Error = PyErr;

    fn try_from(value: PythonStateEqualityConstraint) -> Result<Self, Self::Error> {
        let register = if value.register.is_empty() {
            None
        } else {
            Some(value.register)
        };
        let pointer = if value.pointer.is_empty() {
            None
        } else {
            Some(value.pointer)
        };
        Python::attach(|py| {
            let memory = value.memory.map(|a| a.borrow(py).clone());
            Ok(Self {
                register,
                pointer,
                memory,
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

impl Clone for PythonPointerRangeConstraints {
    fn clone(&self) -> Self {
        Python::attach(|_| Self {
            read: self.read.to_vec(),
            write: self.write.to_vec(),
        })
    }
}

impl TryFrom<PointerRangeConstraints> for PythonPointerRangeConstraints {
    type Error = PyErr;
    fn try_from(value: PointerRangeConstraints) -> Result<Self, Self::Error> {
        Python::attach(|py| {
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

impl TryFrom<PythonPointerRangeConstraints> for PointerRangeConstraints {
    type Error = PyErr;
    fn try_from(value: PythonPointerRangeConstraints) -> Result<Self, Self::Error> {
        Python::attach(|py| {
            let read: Vec<_> = value.read.iter().map(|f| *f.borrow(py)).collect();
            let read = if !read.is_empty() { Some(read) } else { None };
            let write: Vec<_> = value.write.iter().map(|f| *f.borrow(py)).collect();
            let write = if !write.is_empty() { Some(write) } else { None };
            Ok(Self { read, write })
        })
    }
}
