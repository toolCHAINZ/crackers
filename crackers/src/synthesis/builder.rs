use std::sync::Arc;

use derive_builder::Builder;
use jingle::modeling::{ModeledBlock, State};
#[cfg(feature = "pyo3")]
use pyo3::pyclass;
use serde::{Deserialize, Serialize};
use z3::ast::Bool;

use crate::error::CrackersError;
use crate::gadget::library::GadgetLibrary;
use crate::gadget::library::builder::GadgetLibraryConfig;
use crate::reference_program::ReferenceProgram;
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

pub type StateConstraintGenerator =
    dyn Fn(&State, u64) -> Result<Bool, CrackersError> + Send + Sync + 'static;
pub type TransitionConstraintGenerator =
    dyn Fn(&ModeledBlock) -> Result<Option<Bool>, CrackersError> + Send + Sync + 'static;

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
    pub fn build_single(&self) -> Result<AssignmentSynthesis, CrackersError> {
        let s = AssignmentSynthesis::new(self)?;
        Ok(s)
    }

    pub fn build_combined(&self) -> Result<CombinedAssignmentSynthesis, CrackersError> {
        Ok(CombinedAssignmentSynthesis {
            base_config: self.clone(),
        })
    }
}
