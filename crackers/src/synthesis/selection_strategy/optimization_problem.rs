use z3::ast::{Ast, Bool};
use z3::{Context, Optimize, SatResult};

use crate::error::CrackersError;
use crate::error::CrackersError::ModelGenerationError;
use crate::synthesis::Decision;
use crate::synthesis::pcode_theory::conflict_clause::ConflictClause;
use crate::synthesis::selection_strategy::AssignmentResult::{Failure, Success};
use crate::synthesis::selection_strategy::{
    AssignmentResult, InstrLen, SelectionFailure, SelectionStrategy,
};
use crate::synthesis::slot_assignments::SlotAssignments;

#[derive(Debug)]
pub struct OptimizationProblem<'ctx> {
    variables: Vec<Vec<Bool<'ctx>>>,
    z3: &'ctx Context,
    solver: Optimize<'ctx>,
    index_bools: Vec<Bool<'ctx>>,
}

impl<'ctx> OptimizationProblem<'ctx> {
    fn get_decision_variable(&self, var: &Decision) -> &Bool<'ctx> {
        &self.variables[var.index][var.choice]
    }

    fn get_unsat_reason(&self, core: Vec<Bool<'ctx>>) -> SelectionFailure {
        SelectionFailure {
            indices: self
                .index_bools
                .iter()
                .enumerate()
                .filter(|(_, t)| core.iter().any(|c| *c == **t))
                .map(|(i, _)| i)
                .collect(),
        }
    }
}

impl<'ctx> SelectionStrategy<'ctx> for OptimizationProblem<'ctx> {
    fn initialize<T: InstrLen>(z3: &'ctx Context, gadgets: &[Vec<T>]) -> Self {
        let mut prob = Self {
            variables: Default::default(),
            z3,
            solver: Optimize::new(z3),
            index_bools: Vec::with_capacity(gadgets.len()),
        };
        for (i, slot) in gadgets.iter().enumerate() {
            let mut vars = vec![];
            for (j, _) in slot.iter().enumerate() {
                let var = Bool::new_const(prob.z3, Self::derive_var_name(i, j));
                prob.solver
                    .assert_soft(&var.not(), gadgets[i][j].instr_len(), None);
                vars.push(var)
            }
            prob.variables.push(vars);
        }
        for (i, slot) in prob.variables.iter().enumerate() {
            let pbs: Vec<(&Bool<'ctx>, i32)> = slot.iter().map(|b| (b, 1)).collect();
            let b = Bool::fresh_const(z3, &format!("slot_{}", i));
            prob.index_bools.push(b.clone());
            prob.solver.assert_and_track(&Bool::pb_eq(z3, &pbs, 1), &b)
        }
        prob
    }
    fn get_assignments(&mut self) -> Result<AssignmentResult, CrackersError> {
        match self.solver.check(&[]) {
            SatResult::Unsat => Ok(Failure(self.get_unsat_reason(self.solver.get_unsat_core()))),
            SatResult::Unknown => {
                unreachable!("outer SAT solver timed out (this really shouldn't happen)!")
            }
            SatResult::Sat => {
                let model = self.solver.get_model().ok_or(ModelGenerationError)?;
                let assignment =
                    SlotAssignments::create_from_model(model, self.variables.as_slice())?;
                let decisions: Vec<&Bool<'ctx>> = assignment
                    .to_decisions()
                    .iter()
                    .map(|d| self.get_decision_variable(d))
                    .collect();
                self.solver.assert(&Bool::and(self.z3, &decisions).not());

                Ok(Success(assignment))
            }
        }
    }

    fn add_theory_clause(&mut self, clause: &ConflictClause) {
        let choices: Vec<&Bool> = clause
            .decisions()
            .iter()
            .map(|b| self.get_decision_variable(b))
            .collect();
        self.solver
            .assert(&Bool::and(self.z3, choices.as_slice()).not().simplify());
    }
}
