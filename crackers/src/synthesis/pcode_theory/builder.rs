use std::sync::Arc;

use jingle::JingleContext;
use jingle::modeling::{ModeledBlock, ModeledInstruction};
use z3::Context;

use crate::error::CrackersError;
use crate::gadget::candidates::Candidates;
use crate::gadget::library::GadgetLibrary;
use crate::reference_program::ReferenceProgram;
use crate::synthesis::builder::{StateConstraintGenerator, TransitionConstraintGenerator};
use crate::synthesis::pcode_theory::PcodeTheory;
use crate::synthesis::pcode_theory::pcode_assignment::PcodeAssignment;
use crate::synthesis::slot_assignments::SlotAssignments;

#[derive(Clone)]
pub struct PcodeTheoryBuilder<'lib> {
    reference_program: ReferenceProgram,
    library: &'lib GadgetLibrary,
    candidates: Candidates,
    preconditions: Vec<Arc<StateConstraintGenerator>>,
    postconditions: Vec<Arc<StateConstraintGenerator>>,
    pointer_invariants: Vec<Arc<TransitionConstraintGenerator>>,
    candidates_per_slot: usize,
}

impl<'lib> PcodeTheoryBuilder<'lib> {
    // todo: this is gross
    pub fn new(candidates: Candidates, library: &'lib GadgetLibrary) -> PcodeTheoryBuilder<'lib> {
        Self {
            reference_program: Default::default(),
            library,
            candidates,
            preconditions: vec![],
            postconditions: vec![],
            pointer_invariants: vec![],
            candidates_per_slot: 200,
        }
    }
    pub fn build(self, z3: &Context) -> Result<PcodeTheory<ModeledInstruction>, CrackersError> {
        let jingle = JingleContext::new(z3, self.library);
        let modeled_templates = self.model_instructions(&jingle)?;
        let gadget_candidates = self.candidates.model(&jingle)?;
        let t = PcodeTheory::new(
            jingle,
            modeled_templates,
            self.reference_program.initial_memory().clone(),
            gadget_candidates,
            self.preconditions,
            self.postconditions,
            self.pointer_invariants,
        )?;
        Ok(t)
    }

    pub fn build_assignment(
        &self,
        jingle: &JingleContext,
        slot_assignments: SlotAssignments,
    ) -> Result<PcodeAssignment, CrackersError> {
        let modeled_templates: Vec<ModeledInstruction> = self.model_instructions(jingle)?;
        let gadget_candidates: Vec<Vec<ModeledBlock>> = self.candidates.model(jingle)?;
        let selected_candidates: Vec<ModeledBlock> = slot_assignments
            .choices()
            .iter()
            .enumerate()
            .map(|(i, c)| gadget_candidates[i][*c].clone())
            .collect();
        Ok(PcodeAssignment::new(
            self.reference_program.initial_memory().clone(),
            modeled_templates,
            selected_candidates,
            self.preconditions.clone(),
            self.postconditions.clone(),
            self.pointer_invariants.clone(),
        ))
    }

    pub fn with_templates(mut self, templates: ReferenceProgram) -> Self {
        self.reference_program = templates;
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

    pub fn with_pointer_invariants(
        mut self,
        invariants: &[Arc<TransitionConstraintGenerator>],
    ) -> Self {
        self.pointer_invariants = invariants.to_vec();
        self
    }

    pub fn with_max_candidates(mut self, candidates: usize) -> Self {
        self.candidates_per_slot = candidates;
        self
    }

    fn model_instructions(
        &self,
        jingle: &JingleContext,
    ) -> Result<Vec<ModeledInstruction>, CrackersError> {
        let mut modeled_templates = vec![];
        for step in self.reference_program.steps() {
            modeled_templates.push(step.model(jingle)?);
        }
        Ok(modeled_templates)
    }
}
