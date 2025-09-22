mod config;
mod decision;
mod python_logger_layer;
mod synthesis;

use crate::config::PythonCrackersConfig;
use crate::decision::PythonDecisionResult;
use crate::decision::assignment_model::PythonAssignmentModel;
use crate::synthesis::PythonSynthesisParams;
use ::crackers::config::constraint::{
    ConstraintConfig, MemoryEqualityConstraint, PointerRange, PointerRangeConstraints,
    StateEqualityConstraint,
};
use ::crackers::config::meta::{CrackersLogLevel, MetaConfig};
use ::crackers::config::sleigh::SleighConfig;
use ::crackers::config::specification::SpecificationConfig;
use ::crackers::config::synthesis::SynthesisConfig;
use ::crackers::gadget::library::builder::GadgetLibraryConfig;
use ::crackers::synthesis::builder::SynthesisSelectionStrategy;
use ::jingle::python::instruction::PythonInstruction;
use ::jingle::python::modeled_block::PythonModeledBlock;
use ::jingle::python::modeled_instruction::PythonModeledInstruction;
use ::jingle::python::resolved_varnode::PythonResolvedVarNode;
use ::jingle::python::sleigh_context::PythonLoadedSleighContext;
use ::jingle::python::state::PythonState;
use ::jingle::sleigh::PcodeOperation;
use pyo3::prelude::*;

#[pymodule]
#[pyo3(submodule)]
fn jingle(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<PythonResolvedVarNode>()?;
    m.add_class::<PcodeOperation>()?;
    m.add_class::<PythonInstruction>()?;
    m.add_class::<PythonLoadedSleighContext>()?;
    m.add_class::<PythonState>()?;
    m.add_class::<PythonModeledInstruction>()?;
    m.add_class::<PythonModeledBlock>()?;
    Ok(())
}

#[pymodule]
#[pyo3(submodule)]
fn crackers(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<PythonCrackersConfig>()?;
    m.add_class::<PythonDecisionResult>()?;
    m.add_class::<PythonSynthesisParams>()?;
    m.add_class::<PythonAssignmentModel>()?;
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

/// A Python module implemented in Rust.
#[pymodule]
fn _internal(m: &Bound<'_, PyModule>) -> PyResult<()> {
    let j = PyModule::new(m.py(), "jingle")?;
    jingle(&j)?;
    m.add_submodule(&j)?;
    let c = PyModule::new(m.py(), "crackers")?;
    crackers(&c)?;
    m.add_submodule(&c)?;
    Python::attach(|py| {
        py.import("sys")?
            .getattr("modules")?
            .set_item("_internal.jingle", j)?;
        py.import("sys")?
            .getattr("modules")?
            .set_item("_internal.crackers", c)
    })?;

    Ok(())
}
