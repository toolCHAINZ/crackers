use std::sync::Arc;

use jingle::sleigh::Instruction;
use z3::Context;

use crate::error::CrackersError;
use crate::gadget::library::GadgetLibrary;
use crate::synthesis::builder::{PointerConstraintGenerator, StateConstraintGenerator};
use crate::synthesis::pcode_theory::PcodeTheory;

#[derive(Clone)]
pub struct PcodeTheoryBuilder<'lib>
{
    templates: Vec<Instruction>,
    library: &'lib GadgetLibrary,
    preconditions: Vec<Arc<StateConstraintGenerator>>,
    postconditions: Vec<Arc<StateConstraintGenerator>>,
    pointer_invariants: Vec<Arc<PointerConstraintGenerator>>,
    candidates_per_slot: usize,
}

impl<'lib> PcodeTheoryBuilder<'lib>
{
    pub fn new(library: &'lib GadgetLibrary) -> PcodeTheoryBuilder<'lib> {
        Self {
            templates: Default::default(),
            library,
            preconditions: vec![],
            postconditions: vec![],
            pointer_invariants: vec![],
            candidates_per_slot: 200,
        }
    }
    pub fn build<'ctx>(self, z3: &'ctx Context) -> Result<PcodeTheory<'ctx>, CrackersError> {
        let t = PcodeTheory::new(
            z3,
            &self.templates,
            self.library,
            self.candidates_per_slot,
            self.preconditions,
            self.postconditions,
            self.pointer_invariants,
        )?;
        Ok(t)
    }

    pub fn with_templates<T: Iterator<Item = Instruction>>(mut self, templates: T) -> Self {
        self.templates = templates.collect();
        self
    }

    pub fn with_preconditions(mut self, preconditions: &[Arc<StateConstraintGenerator>]) -> Self {
        self.preconditions = preconditions.to_vec();
        self
    }

    pub fn with_postconditions(mut self, postconditions: &[Arc<StateConstraintGenerator>]) -> Self {
        self.postconditions = postconditions.to_vec();
        self
    }

    pub fn with_pointer_invariants(mut self, invariants: &[Arc<PointerConstraintGenerator>]) -> Self {
        self.pointer_invariants = invariants.to_vec();
        self
    }

    pub fn with_max_candidates(mut self, candidates: usize) -> Self{
        self.candidates_per_slot = candidates;
        self
    }
}
