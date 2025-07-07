pub use crate::synthesis::pcode_theory::pcode_assignment::{
    assert_compatible_semantics, assert_concat, assert_state_constraints,
};

#[cfg(feature = "bin")]
pub mod bench;
pub mod config;
pub mod error;
pub mod gadget;
mod reference_program;
pub mod synthesis;
