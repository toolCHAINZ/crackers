use std::fmt::{Display, Formatter};

use thiserror::Error;

#[derive(Debug, Error)]
pub enum CrackersError {
    LibraryDeserialization,
    LibrarySerialization,
}

impl Display for CrackersError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        todo!()
    }
}
