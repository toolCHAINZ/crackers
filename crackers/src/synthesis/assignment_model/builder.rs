use crate::error::CrackersError;
use crate::gadget::Gadget;
use crate::reference_program::ReferenceProgram;
use crate::synthesis::assignment_model::AssignmentModel;
use crate::synthesis::builder::{StateConstraintGenerator, TransitionConstraintGenerator};
use crate::synthesis::pcode_theory::pcode_assignment::PcodeAssignment;
use jingle::modeling::{ModeledBlock, ModeledInstruction};
use jingle::sleigh::SleighArchInfo;
use std::borrow::Borrow;
use std::fmt::{Debug, Formatter};
use std::sync::Arc;
use z3::Solver;

#[derive(Clone)]
pub struct AssignmentModelBuilder {
    pub templates: ReferenceProgram,
    pub gadgets: Vec<Gadget>,
    pub preconditions: Vec<Arc<StateConstraintGenerator>>,
    pub postconditions: Vec<Arc<StateConstraintGenerator>>,
    pub pointer_invariants: Vec<Arc<TransitionConstraintGenerator>>,
    pub arch_info: SleighArchInfo,
}

impl Debug for AssignmentModelBuilder {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AssignmentModelBuilder")
            .field("templates", &self.templates)
            .field("gadgets", &self.gadgets)
            .field("arch_info", &self.arch_info)
            .finish()
    }
}

impl AssignmentModelBuilder {
    fn make_pcode_model<T: Borrow<SleighArchInfo>>(
        &self,
        jingle: T,
    ) -> Result<PcodeAssignment, CrackersError> {
        let jingle = jingle.borrow();
        let modeled_spec: Result<Vec<ModeledInstruction>, _> = self
            .templates
            .steps()
            .iter()
            .map(|i| i.model(jingle).map_err(CrackersError::from))
            .collect();
        let modeled_spec = modeled_spec?;
        let modeled_gadgets: Result<_, _> = self.gadgets.iter().map(|i| i.model(jingle)).collect();
        let modeled_gadgets = modeled_gadgets?;
        Ok(PcodeAssignment::new(
            self.templates.initial_memory().clone(),
            modeled_spec,
            modeled_gadgets,
            self.preconditions.clone(),
            self.postconditions.clone(),
            self.pointer_invariants.clone(),
        ))
    }
    pub fn build(&self) -> Result<AssignmentModel<ModeledBlock>, CrackersError> {
        // todo: remove this structure in jingle

        let pcode_model = self.make_pcode_model(&self.arch_info)?;
        let s = Solver::new();
        pcode_model.check(&self.arch_info, &s)
    }
}
