use jingle::modeling::{ModeledBlock, ModeledInstruction, ModelingContext};
use tracing::{event, Level};
use z3::{Context, SatResult, Solver};
use z3::ast::{Ast, Bool};

use crate::error::CrackersError;
use crate::error::CrackersError::TheoryTimeout;
use crate::synthesis::assignment_problem::Decision;
use crate::synthesis::assignment_problem::pcode_theory::pairwise::PairwiseConstraint;
use crate::synthesis::assignment_problem::sat_problem::SlotAssignments;

mod pairwise;

#[derive(Debug, Clone)]
pub enum ConflictClause {
    Unit(Decision),
    Conjunction(Vec<Decision>),
}
#[derive(Debug, Clone)]
pub struct PcodeTheory<'ctx> {
    z3: &'ctx Context,
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
        let solver = Solver::new(z3);
        Self {
            z3,
            solver,
            templates: templates.to_vec(),
            gadget_candidates: gadget_candidates.to_vec(),
        }
    }
    pub fn check_assignment(
        &self,
        slot_assignments: &SlotAssignments,
    ) -> Result<Option<Vec<ConflictClause>>, CrackersError> {
        let mut conflicts: Vec<ConflictClause> = Vec::new();
        let mut assertions: Vec<PairwiseConstraint> = Vec::new();
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
        }
        self.solver.push();
        for (index, w) in slot_assignments.choices().windows(2).enumerate() {
            let block1 = &self.gadget_candidates[index][w[0]];
            let block2 = &self.gadget_candidates[index + 1][w[1]];
            let concat_var = Bool::fresh_const(self.z3, &"concat_");
            self.solver
                .assert_and_track(&block1.assert_concat(block2)?, &concat_var);
            assertions.push(PairwiseConstraint::new(
                Decision {
                    index,
                    choice: w[0],
                },
                Decision {
                    index: index + 1,
                    choice: w[1],
                },
                concat_var,
            ));
            // todo: do this with the [BranchConstraint] object
            let branch_var = Bool::fresh_const(self.z3, &"branch_");
            self.solver.assert_and_track(
                &block1.can_branch_to_address(block2.get_address())?,
                &branch_var,
            );
            assertions.push(PairwiseConstraint::new(
                Decision {
                    index,
                    choice: w[0],
                },
                Decision {
                    index: index + 1,
                    choice: w[1],
                },
                branch_var,
            ))
        }
        if conflicts.len() == 0 {
            match self.solver.check() {
                SatResult::Unsat => {
                    let unsat_core = self.solver.get_unsat_core();
                    for b in unsat_core {
                        if let Some(m) = assertions.iter().find(|p| p.get_bool().eq(&b)) {
                            conflicts.push(m.gen_conflict_clause())
                        }
                    }
                }
                SatResult::Unknown => return Err(TheoryTimeout),
                SatResult::Sat => return Ok(None),
            }
            event!(Level::DEBUG, "about to check {:?}", conflicts);
        }
        self.solver.pop(1);
        Ok(Some(conflicts))
    }
}
