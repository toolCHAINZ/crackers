use thiserror::Error;

use crackers::error::CrackersError;

#[derive(Debug, Error)]
pub enum CrackersBinError {
    #[error("Error Loading Config ({0})")]
    ConfigLoad(String),
    #[error("Library Error: {0}")]
    Library(#[from] CrackersError),
}
