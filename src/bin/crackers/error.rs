use thiserror::Error;

use crackers::error::CrackersError;

#[derive(Debug, Error)]
pub enum CrackersBinError{
    #[error("Config Load Error")]
    ConfigLoad,
    #[error("Library Error")]
    Library(#[from] CrackersError)
}