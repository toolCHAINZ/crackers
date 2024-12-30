use std::sync::Arc;

use jingle::modeling::ModeledBlock;
use jingle::JingleContext;
use z3::Context;

use crate::error::CrackersError;
use crate::gadget::candidates::Candidates;
use crate::gadget::library::GadgetLibrary;
use crate::synthesis::builder::{StateConstraintGenerator, TransitionConstraintGenerator};
use crate::synthesis::pcode_theory::pcode_assignment::PcodeAssignment;
use crate::synthesis::pcode_theory::PcodeTheory;
use crate::synthesis::slot_assignments::SlotAssignments;

#[derive(Clone)]
pub struct PcodeTheoryBuilder<'lib> {
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
            library,
            candidates,
            preconditions: vec![],
            postconditions: vec![],
            pointer_invariants: vec![],
            candidates_per_slot: 200,
        }
    }
    pub fn build(self, z3: &Context) -> Result<PcodeTheory, CrackersError> {
        let jingle = JingleContext::new(z3, self.library);
        let gadget_candidates = self.candidates.model(&jingle)?;
        let t = PcodeTheory::new(
            jingle,
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
        let gadget_candidates: Vec<Vec<ModeledBlock<'ctx>>> = self.candidates.model(jingle)?;
        let selected_candidates: Vec<ModeledBlock<'ctx>> = slot_assignments
            .choices()
            .iter()
            .enumerate()
            .map(|(i, c)| gadget_candidates[i][*c].clone())
            .collect();
        Ok(PcodeAssignment::new(
            selected_candidates,
            self.preconditions.clone(),
            self.postconditions.clone(),
            self.pointer_invariants.clone(),
        ))
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

}
