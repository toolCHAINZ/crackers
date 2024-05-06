use jingle::modeling::State;
use jingle::sleigh::context::SleighContext;
use jingle::sleigh::Instruction;
use jingle::varnode::ResolvedVarnode;
use serde::Deserialize;
use std::num::NonZeroUsize;
use z3::ast::Bool;
use z3::Context;

use crate::error::CrackersError;
use crate::gadget::library::builder::GadgetLibraryBuilder;
use crate::synthesis::AssignmentSynthesis;
use crate::synthesis::selection_strategy::SelectionStrategy;

#[derive(Copy, Clone, Debug, Deserialize)]
pub enum SynthesisSelectionStrategy {
    #[serde(rename = "sat")]
    SatStrategy,
    #[serde(rename = "optimize")]
    OptimizeStrategy,
}

pub type StateConstraintGenerator =
    dyn for<'a, 'b> Fn(&'a Context, &'b State<'a>) -> Result<Bool<'a>, CrackersError> + Send + Sync + 'static;
pub type PointerConstraintGenerator = dyn for<'a, 'b> Fn(
        &'a Context,
        &'b ResolvedVarnode<'a>,
        &'b State<'a>,
    ) -> Result<Option<Bool<'a>>, CrackersError>
    + Send
    + Sync + 'static;

pub struct SynthesisBuilder {
    pub(crate) selection_strategy: SynthesisSelectionStrategy,
    pub(crate) gadget_library_builder: GadgetLibraryBuilder,
    pub(crate) candidates_per_slot: usize,
    pub(crate) parallel: usize,
    pub(crate) instructions: Vec<Instruction>,
    pub(crate) preconditions: Vec<&'static StateConstraintGenerator>,
    pub(crate) postconditions: Vec<&'static StateConstraintGenerator>,
    pub(crate) pointer_invariants: Vec<&'static PointerConstraintGenerator>,
}

impl Default for SynthesisBuilder {
    fn default() -> Self {
        Self {
            selection_strategy: SynthesisSelectionStrategy::OptimizeStrategy,
            gadget_library_builder: GadgetLibraryBuilder::default(),
            candidates_per_slot: 50,
            parallel: 8,
            instructions: vec![],
            preconditions: vec![],
            postconditions: vec![],
            pointer_invariants: vec![],
        }
    }
}

impl SynthesisBuilder {
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

    pub fn parallel(mut self, len: usize) -> Self {
        self.parallel = len;
        self
    }
    pub fn specification<T: Iterator<Item = Instruction>>(mut self, iter: T) -> Self {
        self.instructions = iter.collect();
        self
    }

    pub fn with_precondition<F>(mut self, condition: &'static F) -> Self
    where
        F: for<'a, 'b> Fn(&'a Context, &'b State<'a>) -> Result<Bool<'a>, CrackersError>
            + Send
            + Sync + 'static,
    {
        self.preconditions.push(condition);
        self
    }

    pub fn with_postcondition<F>(mut self, strat: &'static F) -> Self
    where
        F: for<'a, 'b> Fn(&'a Context, &'b State<'a>) -> Result<Bool<'a>, CrackersError>
            + Send
            + Sync + 'static,
    {
        self.postconditions.push(strat);
        self
    }

    pub fn with_pointer_invariant<F>(mut self, strat: &'static F) -> Self
    where
        F: for<'a, 'b> Fn(
                &'a Context,
                &'b ResolvedVarnode<'a>,
                &'b State<'a>,
            ) -> Result<Option<Bool<'a>>, CrackersError>
            + Send
            + Sync + 'static,
    {
        self.pointer_invariants.push(strat);
        self
    }

    pub fn build<'a>(
        self,
        z3: &'a Context,
        gadget_source: &'a SleighContext,
    ) -> Result<AssignmentSynthesis<'a>, CrackersError> {
        let library = self.gadget_library_builder.build(gadget_source)?;

        let s = AssignmentSynthesis::new(z3, library, self)?;

        Ok(s)
    }
}
