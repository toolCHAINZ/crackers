use jingle::modeling::{ModeledBlock, ModeledInstruction, ModelingContext};
use tracing::{event, Level};
use z3::{Context, SatResult, Solver};
use z3::ast::{Ast, Bool};

use crate::error::CrackersError;
use crate::error::CrackersError::TheoryTimeout;
use crate::synthesis::assignment_problem::Decision;
use crate::synthesis::assignment_problem::pcode_theory::pairwise::ConjunctiveConstraint;
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
        self.solver.reset();
        let unit_conflicts = self.eval_unit_semantics(slot_assignments)?;
        if unit_conflicts.is_some() {
            return Ok(unit_conflicts);
        }
        let mem_and_branch_conflicts = self.eval_memory_conflict_and_branching(slot_assignments)?;
        if mem_and_branch_conflicts.is_some() {
            return Ok(mem_and_branch_conflicts);
        }
        let combined_semantics_conflicts = self.eval_combined_semantics(slot_assignments)?;
        if combined_semantics_conflicts.is_some() {
            return Ok(combined_semantics_conflicts);
        }
        Ok(None)
    }

    fn eval_combined_semantics(
        &self,
        slot_assignments: &SlotAssignments,
    ) -> Result<Option<Vec<ConflictClause>>, CrackersError> {
        let mut assertions = Vec::new();
        let mut conflicts = Vec::with_capacity(slot_assignments.choices().len());
        let mut gadgets: Vec<ModeledBlock<'ctx>> =
            Vec::with_capacity(slot_assignments.choices().len());
        for (i, x) in slot_assignments.choices().iter().enumerate() {
            gadgets.push(self.gadget_candidates[i][*x].clone());
        }
        let template_block = ModeledBlock::try_from(self.templates.as_slice())?;
        let sem_bool = Bool::fresh_const(self.z3, "sem");
        self.solver
            .assert_and_track(&gadgets.as_slice().reaches(&template_block)?, &sem_bool);
        assertions.push(ConjunctiveConstraint::new(
            &slot_assignments.to_decisions(),
            sem_bool,
        ));
        match self.solver.check() {
            SatResult::Unsat => {
                let unsat_core = self.solver.get_unsat_core();
                for b in unsat_core {
                    if let Some(m) = assertions.iter().find(|p| p.get_bool().eq(&b)) {
                        event!(Level::DEBUG, "{:?}: {:?}", b, m.decisions);
                        conflicts.push(m.gen_conflict_clause())
                    }
                }
            }
            SatResult::Unknown => return Err(TheoryTimeout),
            SatResult::Sat => return Ok(None),
        }

        Ok(Some(conflicts))
    }

    fn eval_unit_semantics(
        &self,
        slot_assignments: &SlotAssignments,
    ) -> Result<Option<Vec<ConflictClause>>, CrackersError> {
        let mut conflicts = Vec::with_capacity(slot_assignments.choices().len());
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
        if conflicts.len() > 0 {
            Ok(Some(conflicts))
        } else {
            Ok(None)
        }
    }

    fn eval_memory_conflict_and_branching(
        &self,
        slot_assignments: &SlotAssignments,
    ) -> Result<Option<Vec<ConflictClause>>, CrackersError> {
        let mut conflicts = Vec::with_capacity(slot_assignments.choices().len());
        let mut assertions = Vec::new();
        for (index, w) in slot_assignments.choices().windows(2).enumerate() {
            let block1 = &self.gadget_candidates[index][w[0]];
            let block2 = &self.gadget_candidates[index + 1][w[1]];
            let concat_var = Bool::fresh_const(self.z3, &"concat");
            self.solver
                .assert_and_track(&block1.assert_concat(block2)?, &concat_var);
            assertions.push(ConjunctiveConstraint::new(
                &[
                    Decision {
                        index,
                        choice: w[0],
                    },
                    Decision {
                        index: index + 1,
                        choice: w[1],
                    },
                ],
                concat_var,
            ));
            // todo: do this with the [BranchConstraint] object
            let branch_var = Bool::fresh_const(self.z3, &"branch");
            self.solver.assert_and_track(
                &block1.can_branch_to_address(block2.get_address())?,
                &branch_var,
            );
            assertions.push(ConjunctiveConstraint::new(
                &[
                    Decision {
                        index,
                        choice: w[0],
                    },
                    Decision {
                        index: index + 1,
                        choice: w[1],
                    },
                ],
                branch_var,
            ))
        }
        match self.solver.check() {
            SatResult::Unsat => {
                let unsat_core = self.solver.get_unsat_core();
                for b in unsat_core {
                    if let Some(m) = assertions.iter().find(|p| p.get_bool().eq(&b)) {
                        event!(Level::DEBUG, "{:?}: {:?}", b, m.decisions);
                        conflicts.push(m.gen_conflict_clause())
                    }
                }
                Ok(Some(conflicts))
            }
            SatResult::Unknown => Err(TheoryTimeout),
            SatResult::Sat => Ok(None),
        }
    }


}
