use jingle::modeling::State;
use jingle::sleigh::context::SleighContext;
use jingle::sleigh::Instruction;
use jingle::varnode::ResolvedVarnode;
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

pub type StateConstraintGenerator<'ctx> =
    dyn Fn(&'ctx Context, &State<'ctx>) -> Result<Bool<'ctx>, CrackersError> + 'ctx;
pub type PointerConstraintGenerator<'ctx> = dyn Fn(
        &'ctx Context,
        &ResolvedVarnode<'ctx>,
        &State<'ctx>,
    ) -> Result<Option<Bool<'ctx>>, CrackersError>
    + 'ctx;

pub struct SynthesisBuilder<'ctx> {
    pub(crate) selection_strategy: SynthesisSelectionStrategy,
    pub(crate) gadget_library_builder: GadgetLibraryBuilder,
    pub(crate) candidates_per_slot: usize,
    pub(crate) instructions: Vec<Instruction>,
    pub(crate) preconditions: Vec<Box<StateConstraintGenerator<'ctx>>>,
    pub(crate) postconditions: Vec<Box<StateConstraintGenerator<'ctx>>>,
    pub(crate) pointer_invariants: Vec<Box<PointerConstraintGenerator<'ctx>>>,
}

impl<'ctx> Default for SynthesisBuilder<'ctx> {
    fn default() -> Self {
        Self {
            selection_strategy: SynthesisSelectionStrategy::OptimizeStrategy,
            gadget_library_builder: GadgetLibraryBuilder::default(),
            candidates_per_slot: 50,
            instructions: vec![],
            preconditions: vec![],
            postconditions: vec![],
            pointer_invariants: vec![],
        }
    }
}

impl<'ctx> SynthesisBuilder<'ctx> {
    pub fn with_selection_strategy(mut self, strat: SynthesisSelectionStrategy) -> Self {
        self.selection_strategy = strat;
        self
    }

    pub fn with_gadget_library_builder(mut self, builder: GadgetLibraryBuilder) -> Self {
        self.gadget_library_builder = builder;
        self
    }

    pub fn candidates_per_slot(mut self, len: usize) -> Self {
        self.candidates_per_slot = len;
        self
    }
    pub fn specification<T: Iterator<Item = Instruction>>(mut self, iter: T) -> Self {
        self.instructions = iter.collect();
        self
    }

    pub fn with_precondition<F>(mut self, condition: F) -> Self
    where
        F: Fn(&'ctx Context, &State<'ctx>) -> Result<Bool<'ctx>, CrackersError>
            + Send
            + Sync
            + 'ctx,
    {
        self.preconditions.push(Box::new(condition));
        self
    }

    pub fn with_postcondition<F>(mut self, strat: F) -> Self
    where
        F: Fn(&'ctx Context, &State<'ctx>) -> Result<Bool<'ctx>, CrackersError>
            + Send
            + Sync
            + 'ctx,
    {
        self.postconditions.push(Box::new(strat));
        self
    }

    pub fn with_pointer_invariant<F>(mut self, strat: F) -> Self
    where
        F: Fn(
                &'ctx Context,
                &ResolvedVarnode<'ctx>,
                &State<'ctx>,
            ) -> Result<Option<Bool<'ctx>>, CrackersError>
            + 'ctx,
    {
        self.pointer_invariants.push(Box::new(strat));
        self
    }

    pub fn build(
        self,
        z3: &'ctx Context,
        gadget_source: &SleighContext,
    ) -> Result<AssignmentSynthesis<'ctx>, CrackersError> {
        let library = self.gadget_library_builder.build(gadget_source)?;

        let s = AssignmentSynthesis::new(z3, library, self)?;

        Ok(s)
    }
}
