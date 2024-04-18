use std::fmt::{Display, Formatter};

use jingle::modeling::{ModeledBlock, ModelingContext, State};
use jingle::sleigh::GeneralizedVarNode;
use jingle::varnode::ResolvedVarnode;
use z3::ast::BV;
use z3::Model;

use crate::synthesis::slot_assignments::SlotAssignments;

#[derive(Debug)]
pub struct AssignmentModel<'ctx> {
    assignments: SlotAssignments,
    model: Model<'ctx>,
    gadgets: Vec<ModeledBlock<'ctx>>,
}

impl<'ctx> AssignmentModel<'ctx> {
    pub fn new(
        assignments: SlotAssignments,
        model: Model<'ctx>,
        gadgets: Vec<ModeledBlock<'ctx>>,
    ) -> Self {
        Self {
            assignments,
            model,
            gadgets,
        }
    }

    pub fn model(&self) -> &Model<'ctx> {
        &self.model
    }

    pub fn get_assignments(&self) -> &SlotAssignments {
        &self.assignments
    }
    pub fn initial_state(&'ctx self) -> Option<&'ctx State<'ctx>> {
        self.gadgets.first().map(|f| f.get_original_state())
    }

    pub fn final_state(&'ctx self) -> Option<&'ctx State<'ctx>> {
        self.gadgets.last().map(|f| f.get_final_state())
    }

    pub fn read_original(&'ctx self, vn: GeneralizedVarNode) -> Option<BV<'ctx>> {
        self.initial_state().map(|f| f.read(vn).ok()).flatten()
    }

    pub fn read_output(&'ctx self, vn: GeneralizedVarNode) -> Option<BV<'ctx>> {
        self.final_state().map(|f| f.read(vn).ok()).flatten()
    }

    pub fn read_resolved(&'ctx self, vn: &ResolvedVarnode<'ctx>) -> Option<BV<'ctx>> {
        self.final_state()
            .map(|f| f.read_resolved(&vn).ok())
            .flatten()
    }
}

impl<'ctx> Display for AssignmentModel<'ctx> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "Gadgets:\n")?;
        for block in &self.gadgets {
            writeln!(f, "{}\n", block)?;
        }
        Ok(())
    }
}
