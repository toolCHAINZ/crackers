use z3::ast::{Ast, Bool};
use z3::{SatResult, Solver};

use crate::error::CrackersError;
use crate::error::CrackersError::ModelGenerationError;
use crate::synthesis::Decision;
use crate::synthesis::pcode_theory::conflict_clause::ConflictClause;
use crate::synthesis::selection_strategy::AssignmentResult::{Failure, Success};
use crate::synthesis::selection_strategy::{AssignmentResult, SelectionFailure, SelectionStrategy};
use crate::synthesis::slot_assignments::SlotAssignments;

#[derive(Debug, Clone)]
pub struct SatProblem {
    variables: Vec<Vec<Bool>>,
    solver: Solver,
    last_conflict: Option<ConflictClause>,
    last_assignment: Option<SlotAssignments>,
    index_bools: Vec<Bool>,
}

impl SatProblem {
    fn get_decision_variable(&self, var: &Decision) -> &Bool {
        &self.variables[var.index][var.choice]
    }

    fn get_last_conflict_refutation(&self) -> Option<Bool> {
        self.last_conflict.clone().map(|c| {
            let mut decisions: Vec<&Bool> = vec![];
            for x in c.decisions() {
                decisions.push(self.get_decision_variable(x));
            }
            if let Some(a) = &self.last_assignment {
                for x in c.decisions() {
                    decisions.push(self.get_decision_variable(&Decision {
                        index: x.index,
                        choice: a.choice(x.index),
                    }))
                }
            }
            Bool::or(&decisions).not()
        })
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
impl SelectionStrategy for SatProblem {
    fn initialize<T>(gadgets: &[Vec<T>]) -> SatProblem {
        let mut prob = SatProblem {
            variables: Default::default(),
            solver: Solver::new(),
            last_conflict: None,
            last_assignment: None,
            index_bools: Vec::with_capacity(gadgets.len()),
        };
        for (i, slot) in gadgets.iter().enumerate() {
            let mut vars = vec![];
            for (j, _) in slot.iter().enumerate() {
                vars.push(Bool::new_const(SatProblem::derive_var_name(i, j)))
            }
            prob.variables.push(vars);
        }
        for (i, slot) in prob.variables.iter().enumerate() {
            let pbs: Vec<(&Bool, i32)> = slot.iter().map(|b| (b, 1)).collect();
            let b = Bool::fresh_const(&format!("slot_{i}"));
            prob.index_bools.push(b.clone());
            prob.solver.assert_and_track(Bool::pb_eq(&pbs, 1), &b);
        }
        prob
    }

    fn get_assignments(&mut self) -> Result<AssignmentResult, CrackersError> {
        let sat_result = match self.get_last_conflict_refutation() {
            Some(c) => match self.solver.check_assumptions(&[c]) {
                SatResult::Sat => SatResult::Sat,
                _ => self.solver.check(),
            },
            _ => self.solver.check(),
        };
        match sat_result {
            SatResult::Unsat => Ok(Failure(self.get_unsat_reason(self.solver.get_unsat_core()))),
            SatResult::Unknown => {
                unreachable!("outer SAT solver timed out (this really shouldn't happen)!")
            }
            SatResult::Sat => {
                let model = self.solver.get_model().ok_or(ModelGenerationError)?;
                let assignment =
                    SlotAssignments::create_from_model(model, self.variables.as_slice())?;
                self.last_assignment = Some(assignment.clone());
                let decisions: Vec<&Bool> = assignment
                    .to_decisions()
                    .iter()
                    .map(|d| self.get_decision_variable(d))
                    .collect();
                self.solver.assert(Bool::and(&decisions).not());

                Ok(Success(assignment))
            }
        }
    }

    fn add_theory_clause(&mut self, clause: &ConflictClause) {
        self.last_conflict = Some(clause.clone());
        let choices: Vec<&Bool> = clause
            .decisions()
            .iter()
            .map(|b| self.get_decision_variable(b))
            .collect();
        self.solver
            .assert(Bool::and(choices.as_slice()).not().simplify());
    }
}

#[cfg(test)]
mod tests {
    use crate::synthesis::Decision;
    use crate::synthesis::pcode_theory::conflict_clause::ConflictClause;
    use crate::synthesis::selection_strategy::sat_problem::SatProblem;
    use crate::synthesis::selection_strategy::{AssignmentResult, SelectionStrategy};

    #[test]
    fn test_assignment() {
        let thing = vec![vec![1, 2, 3], vec![2, 3, 4], vec![3, 4, 5]];
        let mut prob = SatProblem::initialize(&thing);
        let assignments = prob.get_assignments();
        // Verify that an unconstrained problem returns a model
        assert!(assignments.is_ok());
        let a = assignments.unwrap();
        match &a {
            AssignmentResult::Success(a) => {
                for (i, x) in a.choices().iter().enumerate() {
                    // verify that all model outputs are sane
                    assert!(x < &thing[i].len())
                }
            }
            AssignmentResult::Failure(_) => {
                panic!()
            }
        }

        prob.add_theory_clause(&ConflictClause::from(Decision {
            index: 0,
            choice: 0,
        }));
        let assignments2 = prob.get_assignments();
        // verify that adding a constraint still returns a model
        assert!(assignments2.is_ok());
        let a2 = assignments2.unwrap();
        // verify that the new constraint has caused the model to change
        assert_ne!(a, a2);
        // now add clauses to make the problem UNSAT
        prob.add_theory_clause(&ConflictClause::from(Decision {
            index: 0,
            choice: 1,
        }));
        prob.add_theory_clause(&ConflictClause::from(Decision {
            index: 0,
            choice: 2,
        }));
        let assignments3 = prob.get_assignments();
        // verify that we do not get a model back
        assert!(matches!(assignments3, Ok(AssignmentResult::Failure(_))));
    }
}
