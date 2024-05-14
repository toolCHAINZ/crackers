use std::slice;
use std::sync::Arc;

use jingle::modeling::{ModeledBlock, ModeledInstruction, ModelingContext, State};
use jingle::sleigh::Instruction;
use tracing::{event, instrument, Level};
use z3::{Context, Model, SatResult, Solver};
use z3::ast::{Ast, Bool};

use conflict_clause::ConflictClause;

use crate::error::CrackersError;
use crate::error::CrackersError::{EmptyAssignment, TheoryTimeout};
use crate::gadget::Gadget;
use crate::gadget::library::GadgetLibrary;
use crate::synthesis::builder::{PointerConstraintGenerator, StateConstraintGenerator};
use crate::synthesis::Decision;
use crate::synthesis::pcode_theory::pcode_assignment::{
    assert_compatible_semantics, assert_concat, assert_state_constraints, PcodeAssignment,
};
use crate::synthesis::pcode_theory::theory_constraint::{
    ConjunctiveConstraint, gen_conflict_clauses, TheoryStage,
};
use crate::synthesis::slot_assignments::SlotAssignments;

pub mod builder;
pub mod conflict_clause;
mod pcode_assignment;
mod theory_constraint;
pub mod theory_worker;

pub struct PcodeTheory<'ctx> {
    z3: &'ctx Context,
    solver: Solver<'ctx>,
    templates: Vec<ModeledInstruction<'ctx>>,
    gadget_candidates: Vec<Vec<ModeledBlock<'ctx>>>,
    preconditions: Vec<Arc<StateConstraintGenerator>>,
    postconditions: Vec<Arc<StateConstraintGenerator>>,
    pointer_invariants: Vec<Arc<PointerConstraintGenerator>>,
}

impl<'ctx> PcodeTheory<'ctx> {
    pub fn new(
        z3: &'ctx Context,
        templates: &[Instruction],
        library: &GadgetLibrary,
        candidates_per_slot: usize,
        preconditions: Vec<Arc<StateConstraintGenerator>>,
        postconditions: Vec<Arc<StateConstraintGenerator>>,
        pointer_invariants: Vec<Arc<PointerConstraintGenerator>>,
    ) -> Result<Self, CrackersError> {
        let mut modeled_templates = vec![];
        let mut gadget_candidates: Vec<Vec<ModeledBlock<'ctx>>> = vec![];
        for template in templates.iter() {
            modeled_templates.push(ModeledInstruction::new(template.clone(), library, z3)?);
            let candidates: Vec<ModeledBlock<'ctx>> = library
                .get_gadgets_for_instruction(z3, template)?
                .take(candidates_per_slot)
                .map(|g| {
                    ModeledBlock::read(z3, library, g.instructions.clone().into_iter()).unwrap()
                })
                .collect();
            event!(
                Level::DEBUG,
                "Instruction {} has {} candidates",
                template.disassembly,
                candidates.len()
            );
            gadget_candidates.push(candidates);
        }
        let solver = Solver::new_for_logic(z3, "QF_ABV").unwrap();
        Ok(Self {
            z3,
            solver,
            templates: modeled_templates,
            gadget_candidates,
            preconditions,
            postconditions,
            pointer_invariants,
        })
    }
    pub fn check_assignment(
        &self,
        slot_assignments: &SlotAssignments,
    ) -> Result<Option<Vec<ConflictClause>>, CrackersError> {
        event!(Level::TRACE, "Resetting solver");
        let gadgets: Vec<ModeledBlock> = slot_assignments
            .choices()
            .iter()
            .enumerate()
            .map(|(i, c)| self.gadget_candidates[i][*c].clone())
            .collect();

        self.solver.reset();
        event!(Level::TRACE, "Evaluating combined semantics");
        self.assert_preconditions(slot_assignments)?;
        self.assert_postconditions(slot_assignments)?;
        self.solver
            .assert(&assert_concat(self.z3, &self.templates)?);

        let mut assertions: Vec<ConjunctiveConstraint> = Vec::new();
        for (index, x) in gadgets.windows(2).enumerate() {
            let branch = Bool::fresh_const(self.z3, "b");
            let concat = Bool::fresh_const(self.z3, "m");
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
        for (index, (spec, g)) in self.templates.iter().zip(gadgets).enumerate() {
            let sem = Bool::fresh_const(self.z3, "c");
            self.solver.assert_and_track(
                &assert_compatible_semantics(self.z3, spec, &g, &self.pointer_invariants)?,
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

        self.collect_conflicts(&assertions, slot_assignments)
    }

    fn assert_preconditions(
        &self,
        slot_assignments: &SlotAssignments,
    ) -> Result<(), CrackersError> {
        let first_gadget = &self
            .gadget_candidates
            .first()
            .map(|f| &f[slot_assignments.choice(0)])
            .ok_or(EmptyAssignment)?;
        self.solver.assert_and_track(&assert_state_constraints(
            &self.z3,
            &self.preconditions,
            &first_gadget.get_original_state(),
        )?, &Bool::fresh_const(&self.z3, "pre"));
        Ok(())
    }

    fn assert_postconditions(
        &self,
        slot_assignments: &SlotAssignments,
    ) -> Result<(), CrackersError> {
        let last_gadget = &self
            .gadget_candidates
            .last()
            .map(|f| &f[slot_assignments.choice(self.gadget_candidates.len() - 1)])
            .ok_or(EmptyAssignment)?;
        self.solver.assert_and_track(&assert_state_constraints(
            &self.z3,
            &self.postconditions,
            &last_gadget.get_final_state(),
        )?, &Bool::fresh_const(&self.z3, "post"));
        Ok(())
    }

    fn collect_conflicts(
        &self,
        assertions: &[ConjunctiveConstraint<'ctx>],
        assignments: &SlotAssignments,
    ) -> Result<Option<Vec<ConflictClause>>, CrackersError> {
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
                if clauses.is_empty() {
                    return Ok(Some(vec![assignments.as_conflict_clause()]));
                }
                Ok(Some(clauses))
            }
            SatResult::Unknown => Err(TheoryTimeout),
            SatResult::Sat => Ok(None),
        }
    }
    
}
