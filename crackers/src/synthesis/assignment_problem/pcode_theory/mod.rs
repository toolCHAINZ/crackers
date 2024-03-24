use jingle::modeling::{ModeledBlock, ModeledInstruction, ModelingContext};
use jingle::varnode::ResolvedVarnode;
use tracing::{event, instrument, Level};
use z3::{Context, Model, SatResult, Solver};
use z3::ast::{Ast, Bool, BV};

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
        let solver = Solver::new_for_logic(z3, "QF_AUFBV").unwrap();
        Self {
            z3,
            solver,
            templates: templates.to_vec(),
            gadget_candidates: gadget_candidates.to_vec(),
        }
    }
    #[instrument(skip_all)]
    pub fn check_assignment(
        &self,
        slot_assignments: &SlotAssignments,
    ) -> Result<Option<Vec<ConflictClause>>, CrackersError> {
        event!(Level::TRACE, "Resetting solver");
        self.solver.reset();
        let mut assertions = Vec::new();
        event!(Level::TRACE, "Evaluating unit semantics");
        let unit_conflicts = self.eval_unit_semantics(&mut assertions, slot_assignments)?;
        if unit_conflicts.is_some() {
            event!(Level::TRACE, "Unit semantics returned conflicts");
            return Ok(unit_conflicts);
        }
        event!(Level::TRACE, "Evaluating combined semantics");
        let mem_and_branch_conflicts = self.eval_memory_conflict_and_branching(&mut assertions, slot_assignments)?;
        if mem_and_branch_conflicts.is_some() {
            event!(Level::TRACE, "combined semantics returned conflicts");

            return Ok(mem_and_branch_conflicts);
        }
        event!(Level::TRACE, "Evaluating combined semantics2");
        let mem_and_branch_conflicts = self.eval_combined_semantics(&mut assertions, slot_assignments)?;
        if mem_and_branch_conflicts.is_some() {
            event!(Level::TRACE, "combined semantics returned conflicts");

            return Ok(mem_and_branch_conflicts);
        }
        Ok(None)
    }

    fn eval_combined_semantics(
        &self,
        assertions: &mut Vec<ConjunctiveConstraint<'ctx>>,
        slot_assignments: &SlotAssignments,
    ) -> Result<Option<Vec<ConflictClause>>, CrackersError> {
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
        self.collect_conflicts(assertions)
    }

    fn eval_unit_semantics(
        &self, assertions: &mut Vec<ConjunctiveConstraint<'ctx>>,
        slot_assignments: &SlotAssignments,
    ) -> Result<Option<Vec<ConflictClause>>, CrackersError> {
        for (index, &choice) in slot_assignments.choices().iter().enumerate() {
            let gadget = &self.gadget_candidates[index][choice];
            let spec = &self.templates[index];
            let refines = Bool::fresh_const(self.z3, "refines");
            self.solver.assert_and_track(&gadget.refines(spec)?, &refines);
            assertions.push(ConjunctiveConstraint::new(&[Decision { index, choice }], refines))
        }
        self.collect_conflicts(assertions)
    }

    fn collect_conflicts(&self, assertions: &mut Vec<ConjunctiveConstraint<'ctx>>) -> Result<Option<Vec<ConflictClause>>, CrackersError> {
        let mut conflicts = Vec::new();
        match self.solver.check() {
            SatResult::Unsat => {
                let unsat_core = self.solver.get_unsat_core();
                for b in unsat_core {
                    if let Some(m) = assertions.iter().find(|p| p.get_bool().eq(&b)) {
                        event!(Level::TRACE, "{:?}: {:?}", b, m.decisions);
                        conflicts.push(m)
                    } else {
                        event!(Level::TRACE, "MISSED");
                    }
                }
                let count_unit = conflicts.iter().map(|f| {
                    match f.is_unit() {
                        false => 0,
                        true => 1
                    }
                }).reduce(|a, b| a + b).unwrap_or(0);

                if count_unit >= 2 {
                    let conflicts: Vec<ConflictClause> = conflicts.iter().filter(|p| p.is_unit()).map(|f| f.gen_conflict_clause()).collect();
                    Ok(Some(conflicts))
                } else {
                    let conflicts: Vec<ConflictClause> = conflicts.iter().map(|f| f.gen_conflict_clause()).collect();
                    Ok(Some(conflicts))
                }
            }
            SatResult::Unknown => Err(TheoryTimeout),
            SatResult::Sat => Ok(None)
        }
    }

    fn eval_memory_conflict_and_branching(
        &self, assertions: &mut Vec<ConjunctiveConstraint<'ctx>>,
        slot_assignments: &SlotAssignments,
    ) -> Result<Option<Vec<ConflictClause>>, CrackersError> {
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
        self.collect_conflicts(assertions)
    }

    pub fn get_model(&self) -> Option<Model<'ctx>> {
        self.solver.get_model()
    }
}
