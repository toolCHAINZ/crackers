mod config;
mod decision;
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
use ::jingle::python::jingle_context::PythonJingleContext;
use ::jingle::python::modeled_block::PythonModeledBlock;
use ::jingle::python::modeled_instruction::PythonModeledInstruction;
use ::jingle::python::sleigh_context::LoadedSleighContextWrapper;
use ::jingle::python::state::PythonState;
use ::jingle::sleigh::{IndirectVarNode, PcodeOperation, VarNode};
use pyo3::prelude::*;

#[pymodule]
#[pyo3(submodule)]
fn jingle(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<VarNode>()?;
    m.add_class::<IndirectVarNode>()?;
    m.add_class::<PcodeOperation>()?;
    m.add_class::<PythonInstruction>()?;
    m.add_class::<LoadedSleighContextWrapper>()?;
    m.add_class::<PythonJingleContext>()?;
    m.add_class::<PythonState>()?;
    m.add_class::<PythonModeledInstruction>()?;
    m.add_class::<PythonModeledBlock>()?;
    Ok(())
}
/// A Python module implemented in Rust.
#[pymodule]
fn crackers(m: &Bound<'_, PyModule>) -> PyResult<()> {
    let j = PyModule::new(m.py(), "jingle")?;
    jingle(&j)?;
    m.add_submodule(&j)?;
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
