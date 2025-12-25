use jingle::JingleError;
use jingle::sleigh::{JingleSleighError, OpCode};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum CrackersConfigError {
    #[error("Invalid log level")]
    InvalidLogLevel,
    #[error("An error reading a file referenced from the config: {0}")]
    Io(#[from] std::io::Error),
    #[error("An error parsing a file with gimli object: {0}")]
    Gimli(#[from] object::Error),
    #[error("Spec objects must have a '_start' symbol")]
    SpecMissingStartSymbol,
    #[error("Unable to parse a segment from the target binary")]
    LibraryParse,
    #[error("The reference program had an illegal p-code operation: {0:?}")]
    IllegalPcodeOperation(OpCode),
    #[error("Spec objects must have a '.text' symbol")]
    SpecMissingTextSection,
    #[error(
        "Unable to determine the architecture of the provided object file. This is a config file limitation and not a sleigh limitation."
    )]
    UnrecognizedArchitecture(String),
    #[error("An error initializing sleigh for a file specified in the config: {0}")]
    Sleigh(#[from] JingleError),
}

impl From<JingleSleighError> for CrackersConfigError {
    fn from(value: JingleSleighError) -> Self {
        CrackersConfigError::Sleigh(JingleError::Sleigh(value))
    }
}
