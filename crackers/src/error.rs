use std::fmt::{Display, Formatter};

use jingle::JingleError;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum CrackersError {
    LibraryDeserialization,
    LibrarySerialization,
    TheoryTimeout,
    ModelGenerationError,
    BooleanAssignmentTimeout,
    Jingle(#[from] JingleError),
}

impl Display for CrackersError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        todo!()
    }
}
