use std::sync::Arc;

use jingle::modeling::{ModeledBlock, ModeledInstruction, ModelingContext, State};
use z3::{Context, SatResult, Solver};
use z3::ast::Bool;

use crate::error::CrackersError;
use crate::synthesis::assignment_model::AssignmentModel;
use crate::synthesis::builder::{PointerConstraintGenerator, StateConstraintGenerator};

pub struct PcodeAssignment<'ctx> {
    spec_trace: Vec<ModeledInstruction<'ctx>>,
    eval_trace: Vec<ModeledBlock<'ctx>>,
    preconditions: Vec<Arc<StateConstraintGenerator>>,
    postconditions: Vec<Arc<StateConstraintGenerator>>,
    pointer_invariants: Vec<Arc<PointerConstraintGenerator>>,
}

impl<'ctx> PcodeAssignment<'ctx> {
    pub fn new(
        spec_trace: Vec<ModeledInstruction<'ctx>>,
        eval_trace: Vec<ModeledBlock<'ctx>>,
        preconditions: Vec<Arc<StateConstraintGenerator>>,
        postconditions: Vec<Arc<StateConstraintGenerator>>,
        pointer_invariants: Vec<Arc<PointerConstraintGenerator>>,
    ) -> Self {
        Self {
            spec_trace,
            eval_trace,
            preconditions,
            postconditions,
            pointer_invariants,
        }
    }

    pub fn check(&self, z3: &'ctx Context, solver: &Solver<'ctx>) -> Result<AssignmentModel<'ctx, ModeledBlock<'ctx>>, CrackersError> {
        solver.assert(&assert_state_constraints(
            z3,
            &self.preconditions,
            self.eval_trace.as_slice().get_original_state(),
        )?);
        solver.assert(&assert_state_constraints(
            z3,
            &self.postconditions,
            self.eval_trace.as_slice().get_final_state(),
        )?);
        solver.assert(&assert_concat(z3, &self.spec_trace)?);
        solver.assert(&assert_concat(z3, &self.eval_trace)?);
        for x in self.eval_trace.windows(2) {
            solver.assert(&x[0].can_branch_to_address(x[1].get_address())?)
        }
        for (spec_inst, trace_inst) in self.spec_trace.iter().zip(&self.eval_trace) {
            solver.assert(&assert_compatible_semantics(
                z3,
                spec_inst,
                trace_inst,
                &self.pointer_invariants,
            )?);
        }
        match solver.check(){
            SatResult::Unsat | SatResult::Unknown=> {Err(CrackersError::ModelGenerationError)}
            SatResult::Sat => {
                let model =solver.get_model().ok_or(CrackersError::ModelGenerationError)?;
                Ok(AssignmentModel::generate(model,self.eval_trace.to_vec()))
            }
        }
    }
}
pub fn assert_concat<'ctx, T: ModelingContext<'ctx>>(
    z3: &'ctx Context,
    items: &[T],
) -> Result<Bool<'ctx>, CrackersError> {
    let mut bools = vec![];
    for x in items.windows(2) {
        bools.push(x[0].assert_concat(&x[1])?)
    }
    Ok(Bool::and(z3, &bools))
}

pub fn assert_compatible_semantics<'ctx, S: ModelingContext<'ctx>, T: ModelingContext<'ctx>>(
    z3: &'ctx Context,
    spec: &S,
    item: &T,
    invariants: &[Arc<PointerConstraintGenerator>],
) -> Result<Bool<'ctx>, CrackersError> {
    let mut bools = vec![];
    // First, all outputs of the item under test must be assignable to the same values
    // as in our specification computation
    bools.push(item.refines(spec)?);
    // Secondly, if the specification has some control flow behavior, the item must be able
    // to have the same control flow behavior
    if let Some(b) = spec.branch_comparison(item)? {
        bools.push(b)
    }
    // Thirdly, every input and output address must pass our pointer constraints
    for invariant in invariants.iter() {
        for x in item.get_inputs().union(&item.get_outputs()) {
            if let Ok(Some(b)) = invariant(z3, x, item.get_original_state()) {
                bools.push(b);
            }
        }
    }
    Ok(Bool::and(&z3, &bools))
}

pub fn assert_state_constraints<'ctx>(
    z3: &'ctx Context,
    constraints: &[Arc<StateConstraintGenerator>],
    state: &State<'ctx>,
) -> Result<Bool<'ctx>, CrackersError> {
    let mut bools = vec![];
    for x in constraints.iter() {
        let assertion = x(z3, state)?;
        bools.push(assertion);
    }
    Ok(Bool::and(z3, &bools))
}
