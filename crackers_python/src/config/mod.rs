use crate::config::constraint::PythonConstraintConfig;
use crate::synthesis::PythonSynthesisParams;
use crackers::config::CrackersConfig;
use crackers::config::meta::MetaConfig;
use crackers::config::sleigh::SleighConfig;
use crackers::config::specification::SpecificationConfig;
use crackers::config::synthesis::SynthesisConfig;
use crackers::gadget::library::builder::GadgetLibraryConfig;
use pyo3::exceptions::PyRuntimeError;
use pyo3::types::PyType;
use pyo3::{Bound, Py, PyErr, PyResult, Python, pyclass, pymethods};

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
        Python::attach(|py| {
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

impl TryFrom<&PythonCrackersConfig> for CrackersConfig {
    type Error = PyErr;

    fn try_from(value: &PythonCrackersConfig) -> Result<Self, Self::Error> {
        Python::attach(|py| {
            Ok(CrackersConfig {
                meta: value.meta.borrow(py).clone(),
                specification: value.spec.borrow(py).clone(),
                library: value.library.borrow(py).clone(),
                sleigh: value.sleigh.borrow(py).clone(),
                synthesis: value.synthesis.borrow(py).clone(),
                constraint: value
                    .constraint
                    .extract::<PythonConstraintConfig>(py)?
                    .try_into()
                    .ok(),
            })
        })
    }
}

#[pymethods]
impl PythonCrackersConfig {
    #[classmethod]
    pub fn from_toml_file(_: &Bound<'_, PyType>, path: &str) -> PyResult<Self> {
        let cfg_bytes = std::fs::read(path)?;
        let s = String::from_utf8(cfg_bytes)?;
        let p: CrackersConfig =
            toml_edit::de::from_str(&s).map_err(|e| PyRuntimeError::new_err(format!("{e}")))?;
        p.try_into()
    }

    pub fn to_json(&self) -> PyResult<String> {
        let unwrapped: CrackersConfig = self.try_into()?;
        let json = serde_json::to_string_pretty(&unwrapped)
            .map_err(|e| PyRuntimeError::new_err(format!("{e}")))?;
        Ok(json)
    }

    #[classmethod]
    pub fn from_json(_: &Bound<'_, PyType>, json: &str) -> PyResult<Self> {
        let p: CrackersConfig =
            serde_json::from_str(json).map_err(|e| PyRuntimeError::new_err(format!("{e}")))?;
        p.try_into()
    }

    pub fn resolve_config(&self) -> PyResult<PythonSynthesisParams> {
        let cfg = CrackersConfig::try_from(self)?;
        let syn = cfg.resolve()?;
        Ok(PythonSynthesisParams { inner: syn })
    }
}
