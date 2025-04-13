use jingle::JingleError;
use jingle::sleigh::JingleSleighError;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum CrackersConfigError {
    #[error("Invalid log level")]
    InvalidLogLevel,
    #[error("An error reading a file referenced from the config")]
    Io(#[from] std::io::Error),
    #[error("An error parsing a file with gimli object: {0}")]
    Gimli(#[from] object::Error),
    #[error("Spec objects must have a '_start' symbol")]
    SpecMissingStartSymbol,
    #[error("Unable to parse a segment from the target binary")]
    LibraryParse,
    #[error("Spec objects must have a '.text' symbol")]
    SpecMissingTextSection,
    #[error(
        "Unable to determine the architecture of the provided object file. This is a config file limitation and not a sleigh limitation."
    )]
    UnrecognizedArchitecture(String),
    #[error("An error initializing sleigh for a file specified in the config")]
    Sleigh(#[from] JingleError),
}

impl From<JingleSleighError> for CrackersConfigError {
    fn from(value: JingleSleighError) -> Self {
        CrackersConfigError::Sleigh(JingleError::Sleigh(value))
    }
}
