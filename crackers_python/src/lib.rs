use ::crackers::config::constraint::{ConstraintConfig, MemoryEqualityConstraint, PointerRange, PointerRangeConstraints, StateEqualityConstraint};
use ::crackers::config::meta::{CrackersLogLevel, MetaConfig};
use ::crackers::config::sleigh::SleighConfig;
use ::crackers::config::specification::SpecificationConfig;
use ::crackers::config::synthesis::SynthesisConfig;
use ::crackers::config::CrackersConfig;
use ::crackers::gadget::library::builder::GadgetLibraryConfig;
use ::crackers::synthesis::builder::SynthesisSelectionStrategy;
use pyo3::prelude::*;

/// A Python module implemented in Rust.
#[pymodule]
fn crackers(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<CrackersConfig>()?;
    m.add_class::<MetaConfig>()?;
    m.add_class::<SpecificationConfig>()?;
    m.add_class::<SleighConfig>()?;
    m.add_class::<GadgetLibraryConfig>()?;
    m.add_class::<SynthesisConfig>()?;
    m.add_class::<CrackersLogLevel>()?;
    m.add_class::<SynthesisSelectionStrategy>()?;
    m.add_class::<PointerRange>()?;
    m.add_class::<MemoryEqualityConstraint>()?;
    m.add_class::<PointerRangeConstraints>()?;
    m.add_class::<StateEqualityConstraint>()?;
    m.add_class::<ConstraintConfig>()?;
    Ok(())
}
