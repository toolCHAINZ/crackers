use jingle::JingleError;
#[cfg(feature = "pyo3")]
use pyo3::PyErr;
#[cfg(feature = "pyo3")]
use pyo3::exceptions::PyRuntimeError;
use thiserror::Error;

use crate::config::error::CrackersConfigError;
use crate::gadget::library::builder::GadgetLibraryConfigBuilderError;
use crate::synthesis::builder::SynthesisParamsBuilderError;

#[derive(Debug, Error)]
pub enum CrackersError {
    #[error("The specification computation had no operations")]
    EmptySpecification,
    #[error("Attempted to evaluate an empty gadget assignment")]
    EmptyAssignment,
    #[error("Encountered an error deserializing a gadget library")]
    LibraryDeserialization,
    #[error("Encountered an error serializing a gadget library")]
    LibrarySerialization,
    #[error("Specification Operation #{index} has no match")]
    UnsimulatedOperation { index: usize },
    #[error("Inner Pcode Theory Solver timed out")]
    TheoryTimeout,
    #[error("Z3 failed to return a model for a given assignment")]
    ModelGenerationError,
    #[error("Outer gadget assignment solver timed out.")]
    BooleanAssignmentTimeout,
    #[error("Unexpected terms found in assignment model")]
    ModelParsingError,
    #[error("Config error: {0}")]
    Config(#[from] CrackersConfigError),
    #[error("Jingle error")]
    Jingle(#[from] JingleError),
    #[error("Invalid gadget library params")]
    LibraryConfig(#[from] GadgetLibraryConfigBuilderError),
    #[error("Invalid synthesis params")]
    SynthesisParams(#[from] SynthesisParamsBuilderError),
    #[cfg(feature = "pyo3")]
    #[error("Python error: {0}")]
    PythonError(#[from] PyErr),
}

#[cfg(feature = "pyo3")]
impl From<CrackersError> for PyErr {
    fn from(value: CrackersError) -> Self {
        PyRuntimeError::new_err(value.to_string())
    }
}
