use std::sync::Arc;

use jingle::JingleContext;
use jingle::modeling::{ModeledBlock, ModelingContext, State};
use tracing::{Level, event};
use z3::ast::Bool;
use z3::{SatResult, Solver};

use conflict_clause::ConflictClause;

use crate::error::CrackersError;
use crate::error::CrackersError::TheoryTimeout;
use crate::reference_program::MemoryValuation;
use crate::synthesis::Decision;
use crate::synthesis::builder::{StateConstraintGenerator, TransitionConstraintGenerator};
use crate::synthesis::pcode_theory::pcode_assignment::{
    assert_compatible_semantics, assert_concat, assert_state_constraints,
};
use crate::synthesis::pcode_theory::theory_constraint::{
    ConjunctiveConstraint, TheoryStage, gen_conflict_clauses,
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
    initial_memory: MemoryValuation,
    gadget_candidates: Vec<Vec<ModeledBlock<'ctx>>>,
    preconditions: Vec<Arc<StateConstraintGenerator>>,
    postconditions: Vec<Arc<StateConstraintGenerator>>,
    pointer_invariants: Vec<Arc<TransitionConstraintGenerator>>,
}

impl<'ctx, S: ModelingContext<'ctx>> PcodeTheory<'ctx, S> {
    pub fn new(
        j: JingleContext<'ctx>,
        templates: Vec<S>,
        initial_memory: MemoryValuation,
        gadget_candidates: Vec<Vec<ModeledBlock<'ctx>>>,
        preconditions: Vec<Arc<StateConstraintGenerator>>,
        postconditions: Vec<Arc<StateConstraintGenerator>>,
        pointer_invariants: Vec<Arc<TransitionConstraintGenerator>>,
    ) -> Result<Self, CrackersError> {
        let solver = Solver::new_for_logic(j.z3, "QF_ABV").unwrap();
        Ok(Self {
            j,
            solver,
            templates,
            initial_memory,
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
        let final_state = self.j.fresh_state();
        self.solver
            .assert(&assert_concat(self.j.z3, &self.templates)?);
        let mut assertions: Vec<ConjunctiveConstraint> = Vec::new();
        let mem_cnstr = self.initial_memory.to_constraint();
        self.solver
            .assert(&mem_cnstr(&self.j, self.templates[0].get_original_state())?);
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
        if let Some(((index, g), choice)) = gadgets
            .iter()
            .enumerate()
            .next_back()
            .zip(slot_assignments.choices().last())
        {
            let concat = Bool::fresh_const(self.j.z3, "m");
            self.solver
                .assert_and_track(&g.get_final_state()._eq(&final_state)?, &concat);
            assertions.push(ConjunctiveConstraint::new(
                &[Decision {
                    index,
                    choice: *choice,
                }],
                concat,
                TheoryStage::Consistency,
            ))
        }
        for (index, (spec, g)) in self.templates.iter().zip(&gadgets).enumerate() {
            let sem = Bool::fresh_const(self.j.z3, "c");
            self.solver.assert_and_track(
                &assert_compatible_semantics(&self.j, spec, g, &self.pointer_invariants)?,
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
        let first_addr = gadgets[0].get_address();
        let last_addr = gadgets[gadgets.len() - 1].get_address();
        let pre = self.assert_preconditions(gadgets[0].get_original_state(), first_addr)?;
        let post = self.assert_postconditions(&final_state, last_addr)?;
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
            TheoryStage::Postcondition,
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
        state: &State<'ctx>,
        addr: u64,
    ) -> Result<Bool<'ctx>, CrackersError> {
        assert_state_constraints(&self.j, &self.preconditions, state, addr)
    }

    fn assert_postconditions(
        &self,
        state: &State<'ctx>,
        addr: u64,
    ) -> Result<Bool<'ctx>, CrackersError> {
        assert_state_constraints(&self.j, &self.postconditions, state, addr)
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
