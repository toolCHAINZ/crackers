use std::sync::Arc;

use jingle::JingleContext;
use jingle::modeling::{ModeledBlock, ModelingContext, };
use tracing::{event, Level};
use z3::{Context, SatResult, Solver};
use z3::ast::Bool;

use conflict_clause::ConflictClause;

use crate::error::CrackersError;
use crate::error::CrackersError::{EmptyAssignment, TheoryTimeout};
use crate::synthesis::builder::{StateConstraintGenerator, TransitionConstraintGenerator};
use crate::synthesis::Decision;
use crate::synthesis::pcode_theory::pcode_assignment::{
    assert_compatible_semantics, assert_concat, assert_state_constraints,
};
use crate::synthesis::pcode_theory::theory_constraint::{
    ConjunctiveConstraint, gen_conflict_clauses, TheoryStage,
};
use crate::synthesis::slot_assignments::SlotAssignments;

pub mod builder;
pub mod conflict_clause;
pub mod pcode_assignment;
mod theory_constraint;
pub mod theory_worker;

pub struct PcodeTheory<'ctx, S: ModelingContext<'ctx>> {
    j: JingleContext<'ctx>,
    solver: Solver<'ctx>,
    templates: Vec<S>,
    gadget_candidates: Vec<Vec<ModeledBlock<'ctx>>>,
    preconditions: Vec<Arc<StateConstraintGenerator>>,
    postconditions: Vec<Arc<StateConstraintGenerator>>,
    pointer_invariants: Vec<Arc<TransitionConstraintGenerator>>,
}

impl<'ctx, S: ModelingContext<'ctx>> PcodeTheory<'ctx, S> {
    pub fn new(
        j: JingleContext<'ctx>,
        templates: Vec<S>,
        gadget_candidates: Vec<Vec<ModeledBlock<'ctx>>>,
        preconditions: Vec<Arc<StateConstraintGenerator>>,
        postconditions: Vec<Arc<StateConstraintGenerator>>,
        pointer_invariants: Vec<Arc<TransitionConstraintGenerator>>,
    ) -> Result<Self, CrackersError> {
        let solver = Solver::new_for_logic(&j.z3, "QF_ABV").unwrap();
        Ok(Self {
            j,
            solver,
            templates,
            gadget_candidates,
            preconditions,
            postconditions,
            pointer_invariants,
        })
    }
    pub fn check_assignment(
        &self,
        slot_assignments: &SlotAssignments,
    ) -> Result<Option<ConflictClause>, CrackersError> {
        event!(Level::TRACE, "Resetting solver");
        let gadgets: Vec<ModeledBlock<'ctx>> = slot_assignments
            .choices()
            .iter()
            .enumerate()
            .map(|(i, c)| self.gadget_candidates[i][*c].clone())
            .collect();

        self.solver.reset();
        event!(Level::TRACE, "Evaluating combined semantics");
        self.solver
            .assert(&assert_concat(self.j.z3, &self.templates)?);

        let mut assertions: Vec<ConjunctiveConstraint> = Vec::new();
        for (index, x) in gadgets.windows(2).enumerate() {
            let branch = Bool::fresh_const(self.j.z3, "b");
            let concat = Bool::fresh_const(self.j.z3, "m");
            self.solver
                .assert_and_track(&x[0].assert_concat(&x[1])?, &concat);

            self.solver
                .assert_and_track(&x[0].can_branch_to_address(x[1].get_address())?, &branch);
            assertions.push(ConjunctiveConstraint::new(
                &[Decision {
                    index,
                    choice: slot_assignments.choice(index),
                }],
                branch,
                TheoryStage::Branch,
            ));
            assertions.push(ConjunctiveConstraint::new(
                &[Decision {
                    index,
                    choice: slot_assignments.choice(index),
                }],
                concat,
                TheoryStage::Consistency,
            ))
        }
        for (index, (spec, g)) in self.templates.iter().zip(&gadgets).enumerate() {
            let sem = Bool::fresh_const(self.j.z3, "c");
            self.solver.assert_and_track(
                &assert_compatible_semantics(self.j.z3, spec, &g, &self.pointer_invariants)?,
                &sem,
            );
            assertions.push(ConjunctiveConstraint::new(
                &[Decision {
                    index,
                    choice: slot_assignments.choice(index),
                }],
                sem,
                TheoryStage::CombinedSemantics,
            ))
        }

        let pre = self.assert_preconditions(slot_assignments)?;
        let post = self.assert_postconditions(slot_assignments)?;
        let pre_bool = Bool::fresh_const(self.j.z3, "pre");
        let post_bool = Bool::fresh_const(self.j.z3, "post");
        self.solver.assert_and_track(&pre, &pre_bool);
        self.solver.assert_and_track(&post, &post_bool);
        assertions.push(ConjunctiveConstraint::new(
            &[],
            pre_bool,
            TheoryStage::Precondition,
        ));
        assertions.push(ConjunctiveConstraint::new(
            &[],
            post_bool,
            TheoryStage::Precondition,
        ));
        event!(Level::TRACE, "Evaluating chain:");
        for x in &gadgets {
            for i in &x.instructions {
                event!(Level::TRACE, "{}", &i.disassembly)
            }
        }
        self.collect_conflicts(&assertions, slot_assignments)
    }

    fn assert_preconditions(
        &self,
        slot_assignments: &SlotAssignments,
    ) -> Result<Bool<'ctx>, CrackersError> {
        let first_gadget = &self
            .gadget_candidates
            .first()
            .map(|f| &f[slot_assignments.choice(0)])
            .ok_or(EmptyAssignment)?;
        assert_state_constraints(
            self.j.z3,
            &self.preconditions,
            &first_gadget.get_original_state(),
        )
    }

    fn assert_postconditions(
        &self,
        slot_assignments: &SlotAssignments,
    ) -> Result<Bool<'ctx>, CrackersError> {
        let last_gadget = &self
            .gadget_candidates
            .last()
            .map(|f| &f[slot_assignments.choice(self.gadget_candidates.len() - 1)])
            .ok_or(EmptyAssignment)?;
        assert_state_constraints(
            self.j.z3,
            &self.postconditions,
            &last_gadget.get_final_state(),
        )
    }

    fn collect_conflicts(
        &self,
        assertions: &[ConjunctiveConstraint<'ctx>],
        assignments: &SlotAssignments,
    ) -> Result<Option<ConflictClause>, CrackersError> {
        let mut constraints = Vec::new();
        match self.solver.check() {
            SatResult::Unsat => {
                let unsat_core = self.solver.get_unsat_core();
                event!(Level::DEBUG, "Unsat core: {:?}", unsat_core);
                for b in &unsat_core {
                    if let Some(m) = assertions.iter().find(|p| p.get_bool().eq(b)) {
                        event!(Level::DEBUG, "{:?}: {:?}", b, m.decisions);
                        constraints.push(m)
                    } else {
                        event!(
                            Level::WARN,
                            "Unsat Core returned unrecognized variable: {:?}",
                            &unsat_core
                        );
                    }
                }
                let clauses = gen_conflict_clauses(constraints.as_slice());
                Ok(Some(clauses.unwrap_or(assignments.as_conflict_clause())))
            }
            SatResult::Unknown => Err(TheoryTimeout),
            SatResult::Sat => Ok(None),
        }
    }
}
