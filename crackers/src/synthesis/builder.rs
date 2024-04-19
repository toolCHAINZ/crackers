use jingle::modeling::State;
use jingle::varnode::ResolvedIndirectVarNode;
use z3::ast::Bool;
use z3::Context;

use crate::error::CrackersError;

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
    selection_strategy: SynthesisSelectionStrategy,
    max_gadget_length: usize,
    max_gadgets_per_slot: usize,
    preconditions: Vec<Box<StateConstraintGenerator<'ctx>>>,
    postconditions: Vec<Box<StateConstraintGenerator<'ctx>>>,
    pointer_invariants: Vec<Box<PointerConstraintGenerator<'ctx>>>,
}

impl<'ctx> Default for SynthesisBuilder<'ctx> {
    fn default() -> Self {
        Self {
            selection_strategy: SynthesisSelectionStrategy::OptimizeStrategy,
            max_gadget_length: 4,
            max_gadgets_per_slot: 50,
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

    pub fn with_precondition<F>(mut self, condition: F) -> Self
    where
        F: Fn(&Context, &State<'ctx>) -> Result<Bool<'ctx>, CrackersError> + 'ctx,
    {
        self.preconditions.push(Box::new(condition));
        self
    }

    pub fn with_postcondition<F>(mut self, strat: F) -> Self
        where
            F: Fn(&Context, &State<'ctx>) -> Result<Bool<'ctx>, CrackersError> + 'ctx,
    {
        self.postconditions.push(Box::new(strat));
        self
    }
}
