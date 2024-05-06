use crate::error::CrackersError;
use crate::gadget::library::GadgetLibrary;
use crate::gadget::Gadget;
use crate::synthesis::builder::{PointerConstraintGenerator, StateConstraintGenerator};
use crate::synthesis::pcode_theory::PcodeTheory;
use jingle::modeling::{ModeledBlock, ModeledInstruction};
use jingle::sleigh::Instruction;
use tracing::{event, Level};
use z3::{Config, Context};

#[derive(Clone)]
pub struct PcodeTheoryBuilder<'lib> {
    templates: Vec<Instruction>,
    library: &'lib GadgetLibrary,
    gadget_candidates: Vec<Vec<Gadget>>,
    preconditions: Vec<&'static StateConstraintGenerator>,
    postconditions: Vec<&'static StateConstraintGenerator>,
    pointer_invariants: Vec<&'static PointerConstraintGenerator>,
    candidates_per_slot: usize,
}

impl<'lib> PcodeTheoryBuilder<'lib> {
    pub fn new(library: &'lib GadgetLibrary) -> PcodeTheoryBuilder<'lib> {
        Self {
            templates: Default::default(),
            library,
            gadget_candidates: vec![],
            preconditions: vec![],
            postconditions: vec![],
            pointer_invariants: vec![],
            candidates_per_slot: 200,
        }
    }
    pub fn build<'ctx>(self, z3: &'ctx Context) -> Result<PcodeTheory<'ctx>, CrackersError> {

        let t = PcodeTheory::new(z3,
            &self.templates, self.library, self.candidates_per_slot,
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

    pub fn with_preconditions(
        mut self,
        preconditions: &[&'static StateConstraintGenerator],
    ) -> Self {
        self.preconditions = preconditions.to_vec();
        self
    }

    pub fn with_postconditions(
        mut self,
        postconditions: &[&'static StateConstraintGenerator],
    ) -> Self {
        self.postconditions = postconditions.to_vec();
        self
    }

    pub fn with_pointer_invariants(
        mut self,
        invariants: &[&'static PointerConstraintGenerator],
    ) -> Self {
        self.pointer_invariants = invariants.to_vec();
        self
    }
}
