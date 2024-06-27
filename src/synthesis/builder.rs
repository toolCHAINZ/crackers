use std::sync::Arc;

use jingle::modeling::{ModeledBlock, State};
use jingle::sleigh::context::SleighContext;
use jingle::sleigh::Instruction;
use rand::random;
use serde::Deserialize;
use z3::ast::Bool;
use z3::Context;

use crate::error::CrackersError;
use crate::gadget::library::builder::GadgetLibraryBuilder;
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
pub struct SynthesisBuilder {
    pub(crate) seed: i64,
    pub(crate) selection_strategy: SynthesisSelectionStrategy,
    pub(crate) gadget_library_builder: GadgetLibraryBuilder,
    pub(crate) candidates_per_slot: usize,
    pub(crate) parallel: usize,
    pub(crate) instructions: Vec<Instruction>,
    pub(crate) preconditions: Vec<Arc<StateConstraintGenerator>>,
    pub(crate) postconditions: Vec<Arc<StateConstraintGenerator>>,
    pub(crate) pointer_invariants: Vec<Arc<TransitionConstraintGenerator>>,
}

impl Default for SynthesisBuilder {
    fn default() -> Self {
        Self {
            selection_strategy: SynthesisSelectionStrategy::OptimizeStrategy,
            gadget_library_builder: GadgetLibraryBuilder::default(),
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

impl SynthesisBuilder {
    pub fn with_selection_strategy(&mut self, strat: SynthesisSelectionStrategy) -> &mut Self {
        self.selection_strategy = strat;
        self
    }

    pub fn with_gadget_library_builder(&mut self, builder: GadgetLibraryBuilder) -> &mut Self {
        self.gadget_library_builder = builder;
        self
    }

    pub fn candidates_per_slot(&mut self, len: usize) -> &mut Self {
        self.candidates_per_slot = len;
        self
    }

    pub fn parallel(&mut self, len: usize) -> &mut Self {
        self.parallel = len;
        self
    }

    pub fn seed(&mut self, seed: i64) -> &mut Self {
        self.seed = seed;
        self
    }
    pub fn specification<T: Iterator<Item = Instruction>>(&mut self, iter: T) -> &mut Self {
        self.instructions = iter.collect();
        self
    }

    pub fn with_precondition(&mut self, condition: Arc<StateConstraintGenerator>) -> &mut Self {
        self.preconditions.push(condition);
        self
    }

    pub fn with_postcondition(&mut self, strat: Arc<StateConstraintGenerator>) -> &mut Self {
        self.postconditions.push(strat);
        self
    }

    pub fn with_pointer_invariant(
        &mut self,
        strat: Arc<TransitionConstraintGenerator>,
    ) -> &mut Self {
        self.pointer_invariants.push(strat);
        self
    }

    pub fn build<'a>(
        self,
        z3: &'a Context,
        gadget_source: &SleighContext,
    ) -> Result<AssignmentSynthesis<'a>, CrackersError> {
        let library = self.gadget_library_builder.build(gadget_source)?;

        let s = AssignmentSynthesis::new(z3, library, self)?;

        Ok(s)
    }
}
