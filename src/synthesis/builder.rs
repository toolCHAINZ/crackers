use std::sync::Arc;

use derive_builder::Builder;
use jingle::modeling::{ModeledBlock, State};
use jingle::sleigh::Instruction;
use serde::{Deserialize, Serialize};
use z3::ast::Bool;
use z3::Context;

use crate::error::CrackersError;
use crate::gadget::library::builder::GadgetLibraryParams;
use crate::gadget::library::GadgetLibrary;
use crate::synthesis::combined::CombinedAssignmentSynthesis;
use crate::synthesis::AssignmentSynthesis;

#[derive(Copy, Clone, Debug, Deserialize, Serialize)]
pub enum SynthesisSelectionStrategy {
    #[serde(rename = "sat")]
    SatStrategy,
    #[serde(rename = "optimize")]
    OptimizeStrategy,
}

pub type StateConstraintGenerator = dyn for<'a, 'b> Fn(&'a Context, &'b State<'a>, u64) -> Result<Bool<'a>, CrackersError>
    + Send
    + Sync
    + 'static;
pub type TransitionConstraintGenerator = dyn for<'a, 'b> Fn(&'a Context, &'b ModeledBlock<'a>) -> Result<Option<Bool<'a>>, CrackersError>
    + Send
    + Sync
    + 'static;

#[derive(Clone, Debug)]
pub enum Library {
    Library(GadgetLibrary),
    Params(GadgetLibraryParams),
}
#[derive(Clone, Builder)]
pub struct SynthesisParams {
    pub seed: i64,
    pub selection_strategy: SynthesisSelectionStrategy,
    #[builder(setter(custom))]
    pub gadget_library: Arc<GadgetLibrary>,
    pub candidates_per_slot: usize,
    pub parallel: usize,
    pub instructions: Vec<Instruction>,
    pub preconditions: Vec<Arc<StateConstraintGenerator>>,
    pub postconditions: Vec<Arc<StateConstraintGenerator>>,
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
