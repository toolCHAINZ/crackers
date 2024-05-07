use std::slice;
use std::sync::Arc;

use jingle::JingleError;
use jingle::modeling::{ModeledBlock, ModeledInstruction, ModelingContext, State};
use jingle::sleigh::Instruction;
use jingle::varnode::ResolvedVarnode;
use tracing::{event, instrument, Level};
use z3::{Config, Context, Model, SatResult, Solver};
use z3::ast::{Ast, Bool};

use crate::error::CrackersError;
use crate::error::CrackersError::{EmptyAssignment, TheoryTimeout};
use crate::gadget::Gadget;
use crate::gadget::library::GadgetLibrary;
use crate::synthesis::builder::{PointerConstraintGenerator, StateConstraintGenerator};
use crate::synthesis::Decision;
use crate::synthesis::pcode_theory::theory_constraint::{
    ConjunctiveConstraint, gen_conflict_clauses, TheoryStage,
};
use crate::synthesis::slot_assignments::SlotAssignments;

pub mod builder;
mod theory_constraint;
pub mod theory_worker;

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

pub struct PcodeTheory<'ctx>
{
    z3: &'ctx Context,
    solver: Solver<'ctx>,
    templates: Vec<ModeledInstruction<'ctx>>,
    gadget_candidates: Vec<Vec<ModeledBlock<'ctx>>>,
    preconditions: Vec<Arc<StateConstraintGenerator>>,
    postconditions: Vec<Arc<StateConstraintGenerator>>,
    pointer_invariants: Vec<Arc<PointerConstraintGenerator>>,
}

impl<'ctx> PcodeTheory<'ctx>
{
    pub fn new(
        z3: &'ctx Context,
        templates: &Vec<Instruction>,
        library: &GadgetLibrary,
        candidates_per_slot: usize,
        preconditions: Vec<Arc<StateConstraintGenerator>>,
        postconditions: Vec<Arc<StateConstraintGenerator>>,
        pointer_invariants: Vec<Arc<PointerConstraintGenerator>>,
    ) -> Result<Self, CrackersError> {
        let mut modeled_templates = vec![];
        let mut gadget_candidates: Vec<Vec<ModeledBlock<'ctx>>> = vec![];
        for template in templates.iter() {
            modeled_templates.push(ModeledInstruction::new(template.clone(), library, &z3)?);
            let candidates: Vec<ModeledBlock<'ctx>> = library
                .get_gadgets_for_instruction(&z3, &template)?
                .take(candidates_per_slot)
                .map(|g| {
                    ModeledBlock::read(&z3, library, g.instructions.clone().into_iter()).unwrap()
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
        let solver = Solver::new_for_logic(&z3, "QF_ABV").unwrap();
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
        self.solver.reset();
        for instruction in self.templates.windows(2) {
            self.solver
                .assert(&instruction[0].assert_concat(&instruction[1])?);
        }
        let mut assertions = Vec::new();
        event!(Level::TRACE, "Evaluating memory and branching");
        let mem_and_branch_conflicts =
            self.eval_memory_conflict_and_branching(&mut assertions, slot_assignments)?;
        if mem_and_branch_conflicts.is_some() {
            event!(Level::DEBUG, "memory and branching returned conflicts");
            return Ok(mem_and_branch_conflicts);
        }
        event!(Level::TRACE, "Evaluating combined semantics");
        self.assert_preconditions(slot_assignments)?;
        self.assert_postconditions(slot_assignments)?;
        let combined_conflicts = self.eval_combined_semantics(&mut assertions, slot_assignments)?;
        if combined_conflicts.is_some() {
            event!(Level::DEBUG, "combined semantics returned conflicts");

            return Ok(combined_conflicts);
        }
        Ok(None)
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
        for x in &self.preconditions {
            let assertion = x(&self.z3, first_gadget.get_original_state())?;
            self.solver.assert(&assertion);
        }

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
        for x in &self.postconditions {
            let assertion = x(&self.z3, last_gadget.get_final_state())?;
            self.solver.assert(&assertion);
        }

        Ok(())
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
            let mut bools = vec![];
            for x in gadget.get_inputs().union(&gadget.get_outputs()) {
                for invariant in &self.pointer_invariants {
                    if let Ok(Some(b)) = invariant(&self.z3, x, gadget.get_original_state()) {
                        bools.push(b);
                    }
                }
            }
            if index == 0 {
                for x in &self.preconditions {
                    let assertion = x(&self.z3, gadget.get_original_state())?;
                    self.solver.assert(&assertion);
                }
            }

            if index == slot_assignments.choices().len() - 1 {
                for x in &self.postconditions {
                    let assertion = x(&self.z3, gadget.get_final_state())?;
                    self.solver.assert(&assertion);
                }

                if let Some(comp) = spec.branch_comparison(gadget)? {
                    bools.push(comp);
                }
                bools.push(gadget.refines(spec)?);
                let refines = Bool::fresh_const(&self.z3, "combine");
                self.solver
                    .assert_and_track(&Bool::and(&self.z3, &bools), &refines);
                assertions.push(ConjunctiveConstraint::new(
                    &[Decision { index, choice }],
                    refines,
                    TheoryStage::CombinedSemantics,
                ));
            }
        }
        self.collect_conflicts(assertions, slot_assignments)
    }

    #[instrument(skip_all)]
    fn eval_unit_semantics(
        &self,
        assertions: &mut Vec<ConjunctiveConstraint<'ctx>>,
        slot_assignments: &SlotAssignments,
    ) -> Result<Option<Vec<ConflictClause>>, CrackersError> {
        for (index, &choice) in slot_assignments.choices().iter().enumerate() {
            let gadget = &self.gadget_candidates[index][choice].fresh()?;
            let spec = &self.templates[index];
            let mut bools = vec![];
            let refines = Bool::fresh_const(&self.z3, "unit");
            if index == 0 {
                for x in &self.preconditions {
                    bools.push(x(&self.z3, gadget.get_original_state())?.simplify());
                }
            }
            if index == slot_assignments.choices().len() - 1 {
                for x in &self.postconditions {
                    bools.push(x(&self.z3, gadget.get_final_state())?.simplify());
                }
            }
            bools.push(gadget.refines(spec)?.simplify());
            if let Some(comp) = spec.branch_comparison(gadget)? {
                bools.push(comp.simplify());
            }
            let condition = Bool::and(&self.z3, &bools);
            self.solver.assert_and_track(&condition, &refines);
            assertions.push(ConjunctiveConstraint::new(
                &[Decision { index, choice }],
                refines,
                TheoryStage::UnitSemantics,
            ));
        }
        // these assertions are used as a pre-filtering step before evaluating a gadget in context
        // so we do not need to keep them around after this check.
        self.collect_conflicts(assertions, slot_assignments)
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
            let concat_var = Bool::fresh_const(&self.z3, &"concat");
            self.solver
                .assert_and_track(&block1.assert_concat(block2)?, &concat_var);
            assertions.push(ConjunctiveConstraint::new(
                &[Decision {
                    index,
                    choice: w[0],
                }],
                concat_var,
                TheoryStage::Consistency,
            ));
            let branch_var = Bool::fresh_const(&self.z3, &"branch");
            self.solver.assert_and_track(
                &block1.can_branch_to_address(block2.get_address())?,
                &branch_var,
            );
            assertions.push(ConjunctiveConstraint::new(
                &[Decision {
                    index,
                    choice: w[0],
                }],
                branch_var,
                TheoryStage::Branch,
            ))
        }
        self.collect_conflicts(assertions, slot_assignments)
    }

    fn collect_conflicts(
        &self,
        assertions: &mut Vec<ConjunctiveConstraint<'ctx>>,
        assignments: &SlotAssignments,
    ) -> Result<Option<Vec<ConflictClause>>, CrackersError> {
        let mut constraints = Vec::new();
        match self.solver.check() {
            SatResult::Unsat => {
                let unsat_core = self.solver.get_unsat_core();
                for b in &unsat_core {
                    if let Some(m) = assertions.iter().find(|p| p.get_bool().eq(&b)) {
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
                if clauses.len() == 0 {
                    return Ok(Some(vec![assignments.as_conflict_clause()]));
                }
                Ok(Some(clauses))
            }
            SatResult::Unknown => Err(TheoryTimeout),
            SatResult::Sat => Ok(None),
        }
    }
    pub fn get_model(&self) -> Option<Model<'ctx>> {
        self.solver.get_model()
    }
}
