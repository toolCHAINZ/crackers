use std::sync::Arc;

use derive_builder::Builder;
use jingle::modeling::{ModeledBlock, State};
use jingle::JingleContext;
#[cfg(feature = "pyo3")]
use pyo3::pyclass;
use serde::{Deserialize, Serialize};
use z3::ast::Bool;
use z3::Context;

use crate::error::CrackersError;
use crate::gadget::library::builder::GadgetLibraryConfig;
use crate::gadget::library::GadgetLibrary;
use crate::reference_program::ReferenceProgram;
use crate::synthesis::combined::CombinedAssignmentSynthesis;
use crate::synthesis::AssignmentSynthesis;

#[derive(Copy, Clone, Debug, Deserialize, Serialize)]
#[cfg_attr(feature = "pyo3", pyclass)]
pub enum SynthesisSelectionStrategy {
    #[serde(rename = "sat")]
    SatStrategy,
    #[serde(rename = "optimize")]
    OptimizeStrategy,
}

pub type StateConstraintGenerator = dyn Fn(&JingleContext, &State, u64) -> Result<Bool, CrackersError>
    + Send
    + Sync
    + 'static;
pub type TransitionConstraintGenerator = dyn Fn(&JingleContext, &ModeledBlock) -> Result<Option<Bool>, CrackersError>
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
    pub reference_program: ReferenceProgram,
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
    pub fn build_single(
        &self,
        z3: &Context,
    ) -> Result<AssignmentSynthesis, CrackersError> {
        let s = AssignmentSynthesis::new(z3, self)?;
        Ok(s)
    }

    pub fn build_combined(
        &self,
        z3: &Context,
    ) -> Result<CombinedAssignmentSynthesis, CrackersError> {
        Ok(CombinedAssignmentSynthesis {
            base_config: self.clone(),
            z3: z3.clone(),
        })
    }
}
