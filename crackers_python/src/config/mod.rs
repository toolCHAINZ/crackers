use crate::config::constraint::PythonConstraintConfig;
use crackers::config::meta::MetaConfig;
use crackers::config::sleigh::SleighConfig;
use crackers::config::specification::SpecificationConfig;
use crackers::config::synthesis::SynthesisConfig;
use crackers::config::CrackersConfig;
use crackers::gadget::library::builder::GadgetLibraryConfig;
use pyo3::exceptions::PyRuntimeError;
use pyo3::{pyclass, pymethods, Bound, Py, PyErr, PyResult, Python};
use pyo3::types::PyType;
use crackers::config::constraint::ConstraintConfig;

mod constraint;

#[pyclass(get_all, set_all, name = "CrackersConfig")]
pub struct PythonCrackersConfig {
    pub meta: Py<MetaConfig>,
    pub spec: Py<SpecificationConfig>,
    pub library: Py<GadgetLibraryConfig>,
    pub sleigh: Py<SleighConfig>,
    pub synthesis: Py<SynthesisConfig>,
    pub constraint: Py<PythonConstraintConfig>,
}

impl TryFrom<CrackersConfig> for PythonCrackersConfig {
    type Error = PyErr;

    fn try_from(value: CrackersConfig) -> Result<Self, Self::Error> {
        Python::with_gil(|py| {
            let constraint: PythonConstraintConfig = value
                .constraint
                .ok_or(PyRuntimeError::new_err("Bad constraint"))?
                .try_into()?;
            Ok(Self {
                meta: Py::new(py, value.meta)?,
                spec: Py::new(py, value.specification)?,
                library: Py::new(py, value.library)?,
                sleigh: Py::new(py, value.sleigh)?,
                synthesis: Py::new(py, value.synthesis)?,
                constraint: Py::new(py, constraint)?,
            })
        })
    }
}

#[pymethods]
impl PythonCrackersConfig {

    #[classmethod]
    pub fn from_toml(_: &Bound<'_, PyType>, path: &str) -> PyResult<Self> {
        let cfg_bytes = std::fs::read(path)?;
        let s = String::from_utf8(cfg_bytes)?;
        let p: CrackersConfig =
            toml_edit::de::from_str(&s).map_err(|e| PyRuntimeError::new_err(format!("{}", e)))?;
        p.try_into()
    }
}