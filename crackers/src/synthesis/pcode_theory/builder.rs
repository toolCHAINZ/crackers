use std::sync::Arc;

use jingle::JingleContext;
use jingle::modeling::{ModeledBlock, ModeledInstruction};
use jingle::sleigh::Instruction;
use z3::Context;

use crate::error::CrackersError;
use crate::gadget::candidates::Candidates;
use crate::gadget::library::GadgetLibrary;
use crate::synthesis::builder::{StateConstraintGenerator, TransitionConstraintGenerator};
use crate::synthesis::pcode_theory::PcodeTheory;
use crate::synthesis::pcode_theory::pcode_assignment::PcodeAssignment;
use crate::synthesis::slot_assignments::SlotAssignments;

#[derive(Clone)]
pub struct PcodeTheoryBuilder<'lib> {
    templates: Vec<Instruction>,
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
            templates: Default::default(),
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
            gadget_candidates,
            self.preconditions,
            self.postconditions,
            self.pointer_invariants,
        )?;
        Ok(t)
    }

    pub fn build_assignment<'ctx>(
        &self,
        jingle: &JingleContext<'ctx>,
        slot_assignments: SlotAssignments,
    ) -> Result<PcodeAssignment<'ctx>, CrackersError> {
        let modeled_templates: Vec<ModeledInstruction<'ctx>> = self.model_instructions(jingle)?;
        let gadget_candidates: Vec<Vec<ModeledBlock<'ctx>>> = self.candidates.model(jingle)?;
        let selected_candidates: Vec<ModeledBlock<'ctx>> = slot_assignments
            .choices()
            .iter()
            .enumerate()
            .map(|(i, c)| gadget_candidates[i][*c].clone())
            .collect();
        Ok(PcodeAssignment::new(
            modeled_templates,
            selected_candidates,
            self.preconditions.clone(),
            self.postconditions.clone(),
            self.pointer_invariants.clone(),
        ))
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

    fn model_instructions<'ctx>(
        &self,
        jingle: &JingleContext<'ctx>,
    ) -> Result<Vec<ModeledInstruction<'ctx>>, CrackersError> {
        let mut modeled_templates = vec![];
        for template in &self.templates {
            modeled_templates.push(ModeledInstruction::new(template.clone(), jingle)?);
        }
        Ok(modeled_templates)
    }
}
