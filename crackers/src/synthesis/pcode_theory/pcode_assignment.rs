use jingle::JingleContext;
use jingle::modeling::{ModeledBlock, ModeledInstruction, ModelingContext, State};
use std::sync::Arc;
use z3::ast::Bool;
use z3::{Context, SatResult, Solver};

use crate::error::CrackersError;
use crate::reference_program::MemoryValuation;
use crate::synthesis::assignment_model::AssignmentModel;
use crate::synthesis::builder::{StateConstraintGenerator, TransitionConstraintGenerator};

pub struct PcodeAssignment {
    initial_spec_memory: MemoryValuation,
    spec_trace: Vec<ModeledInstruction>,
    eval_trace: Vec<ModeledBlock>,
    preconditions: Vec<Arc<StateConstraintGenerator>>,
    postconditions: Vec<Arc<StateConstraintGenerator>>,
    pointer_invariants: Vec<Arc<TransitionConstraintGenerator>>,
}

impl PcodeAssignment {
    pub fn new(
        initial_spec_memory: MemoryValuation,
        spec_trace: Vec<ModeledInstruction>,
        eval_trace: Vec<ModeledBlock>,
        preconditions: Vec<Arc<StateConstraintGenerator>>,
        postconditions: Vec<Arc<StateConstraintGenerator>>,
        pointer_invariants: Vec<Arc<TransitionConstraintGenerator>>,
    ) -> Self {
        Self {
            initial_spec_memory,
            spec_trace,
            eval_trace,
            preconditions,
            postconditions,
            pointer_invariants,
        }
    }

    pub fn check(
        &self,
        jingle: &JingleContext,
        solver: &Solver,
    ) -> Result<AssignmentModel<ModeledBlock>, CrackersError> {
        let mem_cnstr = self.initial_spec_memory.to_constraint();
        solver.assert(&mem_cnstr(jingle, self.spec_trace[0].get_original_state())?);
        solver.assert(&assert_concat(jingle.ctx(), &self.spec_trace)?);
        solver.assert(&assert_concat(jingle.ctx(), &self.eval_trace)?);
        for x in self.eval_trace.windows(2) {
            solver.assert(&x[0].can_branch_to_address(x[1].get_address())?);
        }
        for (spec_inst, trace_inst) in self.spec_trace.iter().zip(&self.eval_trace) {
            solver.assert(&assert_compatible_semantics(
                jingle,
                spec_inst,
                trace_inst,
                &self.pointer_invariants,
            )?);
        }
        solver.assert(&assert_state_constraints(
            jingle,
            &self.preconditions,
            self.eval_trace.as_slice().get_original_state(),
            self.eval_trace[0].get_first_address(),
        )?);
        solver.assert(&assert_state_constraints(
            jingle,
            &self.postconditions,
            self.eval_trace.as_slice().get_final_state(),
            self.eval_trace.last().unwrap().get_last_address(),
        )?);
        match solver.check() {
            SatResult::Unsat | SatResult::Unknown => Err(CrackersError::ModelGenerationError),
            SatResult::Sat => {
                let model = solver
                    .get_model()
                    .ok_or(CrackersError::ModelGenerationError)?;
                Ok(AssignmentModel::new(
                    model,
                    self.eval_trace.to_vec(),
                    jingle.info.clone(),
                ))
            }
        }
    }
}
pub fn assert_concat<T: ModelingContext>(z3: &Context, items: &[T]) -> Result<Bool, CrackersError> {
    let mut bools = vec![];
    for x in items.windows(2) {
        bools.push(x[0].assert_concat(&x[1])?)
    }
    Ok(Bool::and(z3, &bools))
}

#[expect(deprecated)]
pub fn assert_compatible_semantics<S: ModelingContext>(
    jingle: &JingleContext,
    spec: &S,
    item: &ModeledBlock,
    invariants: &[Arc<TransitionConstraintGenerator>],
) -> Result<Bool, CrackersError> {
    let mut bools = vec![];
    // First, all outputs of the item under test must be assignable to the same values
    // as in our specification computation
    bools.push(item.upholds_postcondition(spec)?);
    // Secondly, if the specification has some control flow behavior, the item must be able
    // to have the same control flow behavior
    if let Some(b) = spec.branch_comparison(item)? {
        bools.push(b)
    }
    // Thirdly, every input and output address must pass our pointer constraints
    for invariant in invariants.iter() {
        let inv = invariant(jingle, item)?;
        if let Some(b) = inv {
            bools.push(b)
        }
    }
    Ok(Bool::and(jingle.ctx(), &bools))
}

pub fn assert_state_constraints(
    jingle: &JingleContext,
    constraints: &[Arc<StateConstraintGenerator>],
    state: &State,
    addr: u64,
) -> Result<Bool, CrackersError> {
    let mut bools = vec![];
    for x in constraints.iter() {
        let assertion = x(jingle, state, addr)?;
        bools.push(assertion);
    }
    Ok(Bool::and(jingle.ctx(), &bools))
}
