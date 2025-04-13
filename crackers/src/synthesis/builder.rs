use std::sync::Arc;

use derive_builder::Builder;
use jingle::JingleContext;
use jingle::modeling::{ModeledBlock, State};
use jingle::sleigh::Instruction;
#[cfg(feature = "pyo3")]
use pyo3::pyclass;
use serde::{Deserialize, Serialize};
use z3::Context;
use z3::ast::Bool;

use crate::error::CrackersError;
use crate::gadget::library::GadgetLibrary;
use crate::gadget::library::builder::GadgetLibraryConfig;
use crate::synthesis::AssignmentSynthesis;
use crate::synthesis::combined::CombinedAssignmentSynthesis;

#[derive(Copy, Clone, Debug, Deserialize, Serialize)]
#[cfg_attr(feature = "pyo3", pyclass)]
pub enum SynthesisSelectionStrategy {
    #[serde(rename = "sat")]
    SatStrategy,
    #[serde(rename = "optimize")]
    OptimizeStrategy,
}

pub type StateConstraintGenerator = dyn for<'a> Fn(&JingleContext<'a>, &State<'a>, u64) -> Result<Bool<'a>, CrackersError>
    + Send
    + Sync
    + 'static;
pub type TransitionConstraintGenerator = dyn for<'a> Fn(&JingleContext<'a>, &ModeledBlock<'a>) -> Result<Option<Bool<'a>>, CrackersError>
    + Send
    + Sync
    + 'static;

#[derive(Clone, Debug)]
pub enum Library {
    Library(GadgetLibrary),
    Params(GadgetLibraryConfig),
}
#[derive(Clone, Builder)]
pub struct SynthesisParams {
    pub seed: i64,
    #[builder(default)]
    pub combine_instructions: bool,
    pub selection_strategy: SynthesisSelectionStrategy,
    #[builder(setter(custom))]
    pub gadget_library: Arc<GadgetLibrary>,
    pub candidates_per_slot: usize,
    pub parallel: usize,
    pub instructions: Vec<Instruction>,
    #[builder(default)]
    pub preconditions: Vec<Arc<StateConstraintGenerator>>,
    #[builder(default)]
    pub postconditions: Vec<Arc<StateConstraintGenerator>>,
    #[builder(default)]
    pub pointer_invariants: Vec<Arc<TransitionConstraintGenerator>>,
}

impl SynthesisParamsBuilder {
    pub fn gadget_library(&mut self, gadget_library: GadgetLibrary) -> &mut Self {
        self.gadget_library = Some(gadget_library.into());
        self
    }
}

impl SynthesisParams {
    pub fn build_single<'a>(
        &self,
        z3: &'a Context,
    ) -> Result<AssignmentSynthesis<'a>, CrackersError> {
        let s = AssignmentSynthesis::new(z3, self)?;
        Ok(s)
    }

    pub fn build_combined<'a>(
        &self,
        z3: &'a Context,
    ) -> Result<CombinedAssignmentSynthesis<'a>, CrackersError> {
        Ok(CombinedAssignmentSynthesis {
            base_config: self.clone(),
            z3,
        })
    }
}
