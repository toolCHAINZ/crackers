use std::slice;

use jingle::JingleError;
use jingle::modeling::{ModeledBlock, ModeledInstruction, ModelingContext};
use jingle::sleigh::{create_varnode, varnode};
use tracing::{event, instrument, Level};
use z3::{Context, Model, SatResult, Solver};
use z3::ast::{Ast, Bool, BV};

use crate::error::CrackersError;
use crate::error::CrackersError::TheoryTimeout;
use crate::synthesis::Decision;
use crate::synthesis::pcode_theory::theory_constraint::{
    ConjunctiveConstraint, gen_conflict_clauses, TheoryStage,
};
use crate::synthesis::slot_assignments::SlotAssignments;

mod theory_constraint;

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

    pub fn decisions(&self) -> &[Decision] {
        match self {
            ConflictClause::Unit(decision) => slice::from_ref(decision),
            ConflictClause::Conjunction(d) => d.as_slice(),
        }
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
        let solver = Solver::new_for_logic(z3, "QF_ABV").unwrap();
        for instruction in templates.windows(2) {
            solver.assert(&instruction[0].assert_concat(&instruction[1])?);
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
        let first_gadget = &self.gadget_candidates[0][slot_assignments.choices()[0]];
        let sp = first_gadget
            .get_original_state()
            .read_varnode(&varnode!(first_gadget, "register"[0x20]:8).unwrap())?;
        self.solver
            .assert(&sp._eq(&BV::from_u64(self.z3, 0xDEAD_BEEF_DEAD_BEEF, 64)));
        event!(Level::TRACE, "Evaluating unit semantics");
        let unit_conflicts = self.eval_unit_semantics(&mut assertions, slot_assignments)?;
        if unit_conflicts.is_some() {
            event!(Level::DEBUG, "Unit semantics returned conflicts");
            return Ok(unit_conflicts);
        }
        event!(Level::TRACE, "Evaluating branch destination semantics");
        let branch_semantics_conflicts =
            self.eval_branching_semantics(&mut assertions, slot_assignments)?;
        if branch_semantics_conflicts.is_some() {
            event!(Level::DEBUG, "Branch semantics returned conflicts");
            return Ok(branch_semantics_conflicts);
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
            let refines = Bool::fresh_const(self.z3, "combine");
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
        self.solver.push();
        for (index, &choice) in slot_assignments.choices().iter().enumerate() {
            let gadget = &self.gadget_candidates[index][choice];
            let spec = &self.templates[index];
            let refines = Bool::fresh_const(self.z3, "unit");
            self.solver
                .assert_and_track(&gadget.fresh()?.refines(spec)?, &refines);
            assertions.push(ConjunctiveConstraint::new(
                &[Decision { index, choice }],
                refines,
                TheoryStage::UnitSemantics,
            ))
        }
        // these assertions are used as a pre-filtering step before evaluating a gadget in context
        // so we do not need to keep them around after this check.
        let c = self.collect_conflicts(assertions);
        self.solver.pop(1);
        c
    }

    #[instrument(skip_all)]
    fn eval_branching_semantics(
        &self,
        assertions: &mut Vec<ConjunctiveConstraint<'ctx>>,
        slot_assignments: &SlotAssignments,
    ) -> Result<Option<Vec<ConflictClause>>, CrackersError> {
        for (index, &choice) in slot_assignments.choices().iter().enumerate() {
            let gadget = &self.gadget_candidates[index][choice];
            let spec = &self.templates[index];
            if spec.get_branch_constraint().has_branch() {
                let branch = Bool::fresh_const(self.z3, "branch_dest");
                let branch_meta = Bool::fresh_const(self.z3, "branch_meta");
                let spec_branch_dest = spec.get_branch_constraint().build_bv(spec)?;
                let gadget_branch_dest = gadget.get_branch_constraint().build_bv(gadget)?;

                let spec_branch_meta = spec.get_branch_constraint().build_bv(spec)?;
                let gadget_branch_meta = gadget.get_branch_constraint().build_bv(gadget)?;
                self.solver
                    .assert_and_track(&spec_branch_dest._eq(&gadget_branch_dest), &branch);
                self.solver
                    .assert_and_track(&spec_branch_meta._eq(&gadget_branch_meta), &branch_meta);
                assertions.push(ConjunctiveConstraint::new(
                    &[Decision { index, choice }],
                    branch,
                    TheoryStage::UnitSemantics,
                ))
            }
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
                Ok(Some(gen_conflict_clauses(constraints.as_slice())))
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
                TheoryStage::Branch,
            ))
        }
        self.collect_conflicts(assertions)
    }

    pub fn get_model(&self) -> Option<Model<'ctx>> {
        self.solver.get_model()
    }
}
