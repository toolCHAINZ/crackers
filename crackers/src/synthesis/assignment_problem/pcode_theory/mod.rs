use jingle::JingleError;
use jingle::modeling::{ModeledBlock, ModeledInstruction, ModelingContext};
use tracing::{event, instrument, Level};
use z3::{Context, Model, SatResult, Solver};
use z3::ast::Bool;

use crate::error::CrackersError;
use crate::error::CrackersError::TheoryTimeout;
use crate::synthesis::assignment_problem::Decision;
use crate::synthesis::assignment_problem::pcode_theory::pairwise::{
    ConjunctiveConstraint, TheoryStage,
};
use crate::synthesis::assignment_problem::sat_problem::SlotAssignments;

mod pairwise;

#[derive(Debug, Clone)]
pub enum ConflictClause {
    Unit(Decision),
    Conjunction(Vec<Decision>),
}

impl ConflictClause {
    pub fn combine(clauses: &[ConflictClause]) -> Self {
        let mut result = vec![];
        for x in clauses {
            match x {
                ConflictClause::Conjunction(v) => result.extend(v.clone()),
                ConflictClause::Unit(d) => result.push(d.clone()),
            }
        }
        ConflictClause::Conjunction(result)
    }
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
    ) -> Result<Self, JingleError> {
        let solver = Solver::new_for_logic(z3, "QF_AUFBV").unwrap();
        for instruction in templates.windows(2) {
            instruction[0].assert_concat(&instruction[1])?;
        }
        solver.push();
        Ok(Self {
            z3,
            solver,
            templates: templates.to_vec(),
            gadget_candidates: gadget_candidates.to_vec(),
        })
    }
    pub fn check_assignment(
        &self,
        slot_assignments: &SlotAssignments,
    ) -> Result<Option<Vec<ConflictClause>>, CrackersError> {
        event!(Level::TRACE, "Resetting solver");
        self.solver.pop(1);
        self.solver.push();
        let mut assertions = Vec::new();
        event!(Level::TRACE, "Evaluating unit semantics");
        let unit_conflicts = self.eval_unit_semantics(&mut assertions, slot_assignments)?;
        if unit_conflicts.is_some() {
            event!(Level::DEBUG, "Unit semantics returned conflicts");
            return Ok(unit_conflicts);
        }
        event!(Level::TRACE, "Evaluating memory and branching");
        let mem_and_branch_conflicts =
            self.eval_memory_conflict_and_branching(&mut assertions, slot_assignments)?;
        if mem_and_branch_conflicts.is_some() {
            event!(Level::DEBUG, "memory and branching returned conflicts");
            return Ok(mem_and_branch_conflicts);
        }
        event!(Level::TRACE, "Evaluating combined semantics");
        let combined_conflicts = self.eval_combined_semantics(&mut assertions, slot_assignments)?;
        if combined_conflicts.is_some() {
            event!(Level::DEBUG, "combined semantics returned conflicts");

            return Ok(combined_conflicts);
        }
        Ok(None)
    }
    #[instrument(skip_all)]
    fn eval_combined_semantics(
        &self,
        assertions: &mut Vec<ConjunctiveConstraint<'ctx>>,
        slot_assignments: &SlotAssignments,
    ) -> Result<Option<Vec<ConflictClause>>, CrackersError> {
        for (index, &choice) in slot_assignments.choices().iter().enumerate() {
            let gadget = &self.gadget_candidates[index][choice];
            let spec = &self.templates[index];
            let refines = Bool::fresh_const(self.z3, "refines");
            self.solver
                .assert_and_track(&gadget.refines(spec)?, &refines);
            assertions.push(ConjunctiveConstraint::new(
                &[Decision { index, choice }],
                refines,
                TheoryStage::CombinedSemantics,
            ))
        }
        self.collect_conflicts(assertions)
    }

    #[instrument(skip_all)]

    fn eval_unit_semantics(
        &self,
        assertions: &mut Vec<ConjunctiveConstraint<'ctx>>,
        slot_assignments: &SlotAssignments,
    ) -> Result<Option<Vec<ConflictClause>>, CrackersError> {
        for (index, &choice) in slot_assignments.choices().iter().enumerate() {
            let gadget = &self.gadget_candidates[index][choice];
            let spec = &self.templates[index];
            let refines = Bool::fresh_const(self.z3, "refines");
            self.solver
                .assert_and_track(&gadget.fresh()?.reaches(&spec.fresh()?)?, &refines);
            assertions.push(ConjunctiveConstraint::new(
                &[Decision { index, choice }],
                refines,
                TheoryStage::UnitSemantics,
            ))
        }
        self.collect_conflicts(assertions)
    }

    fn collect_conflicts(
        &self,
        assertions: &mut Vec<ConjunctiveConstraint<'ctx>>,
    ) -> Result<Option<Vec<ConflictClause>>, CrackersError> {
        let mut constraints = Vec::new();
        match self.solver.check() {
            SatResult::Unsat => {
                let unsat_core = self.solver.get_unsat_core();
                for b in unsat_core {
                    if let Some(m) = assertions.iter().find(|p| p.get_bool().eq(&b)) {
                        event!(Level::TRACE, "{:?}: {:?}", b, m.decisions);
                        constraints.push(m)
                    } else {
                        event!(Level::WARN, "Unsat Core returned unrecognized variable");
                    }
                }
                let mut unit_conflicts = vec![];
                let mut combined_conflicts = vec![];
                for x in constraints {
                    match x.get_type() {
                        TheoryStage::UnitSemantics => unit_conflicts.push(x.gen_conflict_clause()),
                        _ => combined_conflicts.push(x.gen_conflict_clause()),
                    }
                }
                if combined_conflicts.len() > 0 {
                    Ok(Some(vec![ConflictClause::combine(&combined_conflicts)]))
                } else if unit_conflicts.len() > 0 {
                    Ok(Some(unit_conflicts))
                } else {
                    Ok(None)
                }
            }
            SatResult::Unknown => Err(TheoryTimeout),
            SatResult::Sat => Ok(None),
        }
    }

    #[instrument(skip_all)]

    fn eval_memory_conflict_and_branching(
        &self,
        assertions: &mut Vec<ConjunctiveConstraint<'ctx>>,
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
                TheoryStage::Consistency,
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
                TheoryStage::Consistency,
            ))
        }
        self.collect_conflicts(assertions)
    }

    pub fn get_model(&self) -> Option<Model<'ctx>> {
        self.solver.get_model()
    }
}
