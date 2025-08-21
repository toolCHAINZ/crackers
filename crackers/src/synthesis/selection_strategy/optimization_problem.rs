use z3::ast::{Ast, Bool};
use z3::{Optimize, SatResult};

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
pub struct OptimizationProblem {
    variables: Vec<Vec<Bool>>,
    solver: Optimize,
    index_bools: Vec<Bool>,
}

impl OptimizationProblem {
    fn get_decision_variable(&self, var: &Decision) -> &Bool {
        &self.variables[var.index][var.choice]
    }

    fn get_unsat_reason(&self, core: Vec<Bool>) -> SelectionFailure {
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

impl SelectionStrategy for OptimizationProblem {
    fn initialize<T: InstrLen>(gadgets: &[Vec<T>]) -> Self {
        let mut prob = Self {
            variables: Default::default(),
            solver: Optimize::new(),
            index_bools: Vec::with_capacity(gadgets.len()),
        };
        for (i, slot) in gadgets.iter().enumerate() {
            let mut vars = vec![];
            for (j, _) in slot.iter().enumerate() {
                let var = Bool::new_const(Self::derive_var_name(i, j));
                prob.solver
                    .assert_soft(&var.not(), gadgets[i][j].instr_len(), None);
                vars.push(var)
            }
            prob.variables.push(vars);
        }
        for (i, slot) in prob.variables.iter().enumerate() {
            let pbs: Vec<(&Bool, i32)> = slot.iter().map(|b| (b, 1)).collect();
            let b = Bool::fresh_const(&format!("slot_{i}"));
            prob.index_bools.push(b.clone());
            prob.solver.assert_and_track(&Bool::pb_eq(&pbs, 1), &b)
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
                let decisions: Vec<&Bool> = assignment
                    .to_decisions()
                    .iter()
                    .map(|d| self.get_decision_variable(d))
                    .collect();
                self.solver.assert(&Bool::and(&decisions).not());

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
            .assert(&Bool::and(choices.as_slice()).not().simplify());
    }
}
