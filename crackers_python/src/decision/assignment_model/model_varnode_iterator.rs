use jingle::python::z3::ast::TryIntoPythonZ3;
use pyo3::{Py, PyAny, PyRef, PyRefMut, pyclass, pymethods};
use z3::ast::BV;

#[pyclass(unsendable)]
pub struct ModelVarNodeIterator {
    vn: Box<dyn Iterator<Item = (String, BV<'static>)>>,
}

impl ModelVarNodeIterator {
    pub fn new<T: Iterator<Item = (String, BV<'static>)> + 'static>(vn: T) -> Self {
        Self { vn: Box::new(vn) }
    }
}

#[pymethods]
impl ModelVarNodeIterator {
    pub fn __iter__(slf: PyRef<Self>) -> PyRef<Self> {
        slf
    }

    pub fn __next__(mut slf: PyRefMut<Self>) -> Option<(String, Py<PyAny>)> {
        let (name, bv) = slf.vn.next()?;
        match bv.try_into_python() {
            Ok(bv) => Some((name, bv)),
            _ => None,
        }
    }
}
