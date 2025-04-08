use pyo3::prelude::*;
use ::crackers::config::CrackersConfig;
use ::crackers::config::meta::{CrackersLogLevel, MetaConfig};
use ::crackers::config::sleigh::SleighConfig;
use ::crackers::config::specification::SpecificationConfig;
use ::crackers::config::synthesis::SynthesisConfig;
use ::crackers::gadget::library::builder::GadgetLibraryConfig;
use ::crackers::synthesis::builder::SynthesisSelectionStrategy;

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
    Ok(())
}
