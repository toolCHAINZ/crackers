use jingle::modeling::State;
use jingle::sleigh::context::SleighContext;
use jingle::sleigh::Instruction;
use jingle::varnode::ResolvedIndirectVarNode;
use z3::ast::Bool;
use z3::Context;

use crate::error::CrackersError;
use crate::gadget::library::builder::GadgetLibraryBuilder;
use crate::synthesis::AssignmentSynthesis;

#[derive(Copy, Clone, Debug)]
pub enum SynthesisSelectionStrategy {
    SatStrategy,
    OptimizeStrategy,
}

pub type StateConstraintGenerator<'ctx> =
    dyn Fn(&'ctx Context, &State<'ctx>) -> Result<Bool<'ctx>, CrackersError> + 'ctx;
pub type PointerConstraintGenerator<'ctx> = dyn Fn(&'ctx Context, &ResolvedIndirectVarNode<'ctx>) -> Result<Option<Bool<'ctx>>, CrackersError>
    + 'ctx;

pub struct SynthesisBuilder<'ctx> {
    pub(crate) selection_strategy: SynthesisSelectionStrategy,
    pub(crate) max_gadget_length: usize,
    pub(crate) candidates_per_slot: usize,
    pub(crate) instructions: Box<dyn Iterator<Item = Instruction> + 'ctx>,
    pub(crate) preconditions: Vec<Box<StateConstraintGenerator<'ctx>>>,
    pub(crate) postconditions: Vec<Box<StateConstraintGenerator<'ctx>>>,
    pub(crate) pointer_invariants: Vec<Box<PointerConstraintGenerator<'ctx>>>,
}

impl<'ctx> Default for SynthesisBuilder<'ctx> {
    fn default() -> Self {
        Self {
            selection_strategy: SynthesisSelectionStrategy::OptimizeStrategy,
            max_gadget_length: 4,
            candidates_per_slot: 50,
            instructions: Box::new(vec![].into_iter()),
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

    pub fn max_gadget_length(mut self, len: usize) -> Self {
        self.max_gadget_length = len;
        self
    }

    pub fn candidates_per_slot(mut self, len: usize) -> Self {
        self.candidates_per_slot = len;
        self
    }
    pub fn specification<T: Iterator<Item = Instruction> + 'ctx>(mut self, iter: T) -> Self {
        self.instructions = Box::new(iter);
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
                &ResolvedIndirectVarNode<'ctx>,
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
        let lib_builder =
            GadgetLibraryBuilder::default().max_gadget_length(&self.max_gadget_length);
        let library = lib_builder.build(gadget_source)?;

        let s = AssignmentSynthesis::new(z3, library, self)?;

        Ok(s)
    }
}
