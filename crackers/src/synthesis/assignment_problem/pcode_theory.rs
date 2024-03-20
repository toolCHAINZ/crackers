use jingle::modeling::{ModeledBlock, ModeledInstruction, ModelingContext};
use z3::{Context, SatResult, Solver};
use z3::ast::Bool;

use crate::error::CrackersError;
use crate::error::CrackersError::TheoryTimeout;
use crate::synthesis::assignment_problem::Decision;
use crate::synthesis::assignment_problem::sat_problem::SlotAssignments;

pub enum ConflictClause {
    Unit(Decision),
    Conjunction(Vec<Decision>),
}
pub struct PcodeTheory<'ctx> {
    solver: Solver<'ctx>,
    templates: Vec<ModeledInstruction<'ctx>>,
    gadget_candidates: Vec<Vec<ModeledBlock<'ctx>>>,
}

impl<'ctx> PcodeTheory<'ctx> {
    pub fn new(
        z3: &'ctx Context,
        templates: &[ModeledInstruction<'ctx>],
        gadget_candidates: &[Vec<ModeledBlock<'ctx>>],
    ) -> Self {
        Self {
            solver: Solver::new(z3),
            templates: templates.to_vec(),
            gadget_candidates: gadget_candidates.to_vec(),
        }
    }
    pub fn check_assignment(
        &self,
        slot_assignments: SlotAssignments,
    ) -> Result<Option<Vec<ConflictClause>>, CrackersError> {
        let mut conflicts: Vec<ConflictClause> = Vec::new();
        let mut assertions: Vec<Bool> = Vec::new();
        // first check all the individual choices that we've made. Many of these, especially at first,
        // will produce conflict clauses, so it's good to get rid of these quickly

        // todo: At some point maybe cache which ones we've already verified. Maybe do that for all
        // sub-problems below a given size?
        for (index, choice) in slot_assignments.choices().iter().enumerate() {
            let gadget = &self.gadget_candidates[index][choice.clone()];
            let spec = &self.templates[index];
            match self.solver.check_assumptions(&[gadget.reaches(spec)?]) {
                SatResult::Unsat => conflicts.push(ConflictClause::Unit(Decision {
                    index,
                    choice: choice.clone(),
                })),
                SatResult::Unknown => return Err(TheoryTimeout),
                SatResult::Sat => {}
            }
            if index > 1 {
                assertions
                    .push(self.gadget_candidates[index][choice.clone() - 1].assert_concat(gadget)?);
                // todo: do this with the [BranchConstraint] object
                assertions.push(
                    self.gadget_candidates[index][choice.clone() - 1]
                        .can_branch_to_address(gadget.get_address())?,
                );
            }
        }
        if conflicts.len() == 0 {
            match self.solver.check_assumptions(&assertions.as_slice()) {
                SatResult::Unsat => {
                    let unsat_core = self.solver.get_unsat_core();
                    panic!("{:?}", unsat_core);
                }
                SatResult::Unknown => return Err(TheoryTimeout),
                SatResult::Sat => return Ok(None),
            }
        }
        Ok(Some(conflicts))
    }
}
