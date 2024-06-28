use derive_builder::Builder;
use std::sync::Arc;

use jingle::modeling::{ModeledBlock, State};
use jingle::sleigh::context::SleighContext;
use jingle::sleigh::Instruction;
use rand::random;
use serde::Deserialize;
use z3::ast::Bool;
use z3::Context;

use crate::error::CrackersError;
use crate::gadget::library::builder::GadgetLibraryParams;
use crate::gadget::library::GadgetLibrary;
use crate::synthesis::AssignmentSynthesis;

#[derive(Copy, Clone, Debug, Deserialize)]
pub enum SynthesisSelectionStrategy {
    #[serde(rename = "sat")]
    SatStrategy,
    #[serde(rename = "optimize")]
    OptimizeStrategy,
}

pub type StateConstraintGenerator = dyn for<'a, 'b> Fn(&'a Context, &'b State<'a>) -> Result<Bool<'a>, CrackersError>
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
#[builder(default)]
pub struct SynthesisParams {
    pub(crate) seed: i64,
    pub(crate) selection_strategy: SynthesisSelectionStrategy,
    pub(crate) gadget_library_builder: Library,
    pub(crate) candidates_per_slot: usize,
    pub(crate) parallel: usize,
    pub(crate) instructions: Vec<Instruction>,
    pub(crate) preconditions: Vec<Arc<StateConstraintGenerator>>,
    pub(crate) postconditions: Vec<Arc<StateConstraintGenerator>>,
    pub(crate) pointer_invariants: Vec<Arc<TransitionConstraintGenerator>>,
}

impl Default for SynthesisParams {
    fn default() -> Self {
        Self {
            selection_strategy: SynthesisSelectionStrategy::OptimizeStrategy,
            gadget_library_builder: Library::Params(GadgetLibraryParams::default()),
            candidates_per_slot: 50,
            parallel: 4,
            instructions: vec![],
            preconditions: vec![],
            postconditions: vec![],
            pointer_invariants: vec![],
            seed: random(),
        }
    }
}

impl SynthesisParams {
    pub fn build<'a>(
        self,
        z3: &'a Context,
        gadget_source: &SleighContext,
    ) -> Result<AssignmentSynthesis<'a>, CrackersError> {
        let library = match &self.gadget_library_builder {
            Library::Library(l) => l.clone(),
            Library::Params(p) => p.build(gadget_source)?,
        };
        let s = AssignmentSynthesis::new(z3, library, self)?;

        Ok(s)
    }
}
