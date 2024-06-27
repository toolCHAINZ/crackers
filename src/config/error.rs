use jingle::sleigh::JingleSleighError;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum CrackersConfigError{
    #[error("An error reading a file referenced from the config")]
    Io(#[from] std::io::Error),
    #[error("An error parsing a file with gimli object")]
    Gimli(#[from] object::Error),
    #[error("Unable to determine the architecture of the provided object file. This is a config file limitation and not a sleigh limitation.")]
    UnrecognizedArchitecture(String),
    #[error("An error initializing sleigh for a file specified in the config")]
    Sleigh(#[from] JingleSleighError)
}