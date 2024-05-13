use z3::{Context, SatResult, Solver};
use z3::ast::{Ast, Bool};

use crate::synthesis::Decision;
use crate::synthesis::pcode_theory::conflict_clause::ConflictClause;
use crate::synthesis::selection_strategy::SelectionStrategy;
use crate::synthesis::slot_assignments::SlotAssignments;

#[derive(Debug, Clone)]
pub struct SatProblem<'ctx> {
    variables: Vec<Vec<Bool<'ctx>>>,
    z3: &'ctx Context,
    solver: Solver<'ctx>,
}

impl<'ctx> SatProblem<'ctx> {
    fn get_decision_variable(&self, var: &Decision) -> &Bool<'ctx> {
        &self.variables[var.index][var.choice]
    }
}

impl<'ctx> SelectionStrategy<'ctx> for SatProblem<'ctx> {
    fn initialize<T>(z3: &'ctx Context, gadgets: &[Vec<T>]) -> SatProblem<'ctx> {
        let mut prob = SatProblem {
            variables: Default::default(),
            z3,
            solver: Solver::new_for_logic(z3, "QF_FD").unwrap(),
        };
        for (i, slot) in gadgets.iter().enumerate() {
            let mut vars = vec![];
            for (j, _) in slot.iter().enumerate() {
                vars.push(Bool::new_const(prob.z3, SatProblem::derive_var_name(i, j)))
            }
            prob.variables.push(vars);
        }
        for slot in &prob.variables {
            let pbs: Vec<(&Bool<'ctx>, i32)> = slot.iter().map(|b| (b, 1)).collect();
            prob.solver.assert(&Bool::pb_eq(z3, &pbs, 1))
        }
        prob
    }

    fn get_assignments(&self, blacklist: &[&SlotAssignments]) -> Option<SlotAssignments> {
        let terms: Vec<Bool> = blacklist
            .iter()
            .map(|s| {
                let decisions: Vec<&Bool<'ctx>> = s
                    .to_decisions()
                    .iter()
                    .map(|d| self.get_decision_variable(d))
                    .collect();
                Bool::and(self.z3, &decisions).not()
            })
            .collect();
        self.solver.push();
        self.solver.assert(&Bool::and(self.z3, &terms));
        match self.solver.check() {
            SatResult::Unsat => None,
            SatResult::Unknown => {
                unreachable!("outer SAT solver timed out (this really shouldn't happen)!")
            }
            SatResult::Sat => {
                let model = self.solver.get_model()?;
                self.solver.pop(1);
                SlotAssignments::create_from_model(model, self.variables.as_slice())
            }
        }
    }

    fn add_theory_clauses(&mut self, clauses: &[ConflictClause]) {
        for clause in clauses {
            match clause {
                ConflictClause::Unit(d) => {
                    let var = self.get_decision_variable(d);
                    self.solver.assert(&var.not());
                }
                ConflictClause::Conjunction(v) => {
                    let choices: Vec<&Bool> =
                        v.iter().map(|b| self.get_decision_variable(b)).collect();
                    self.solver
                        .assert(&Bool::and(self.z3, choices.as_slice()).not().simplify());
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use z3::{Config, Context};

    use crate::synthesis::Decision;
    use crate::synthesis::pcode_theory::conflict_clause::ConflictClause;
    use crate::synthesis::selection_strategy::sat_problem::SatProblem;
    use crate::synthesis::selection_strategy::SelectionStrategy;

    #[test]
    fn test_assignment() {
        let z3 = Context::new(&Config::new());
        let thing = vec![vec![1, 2, 3], vec![2, 3, 4], vec![3, 4, 5]];
        let mut prob = SatProblem::initialize(&z3, &thing);
        let assignments = prob.get_assignments();
        // Verify that an unconstrained problem returns a model
        assert!(assignments.is_some());
        let a = assignments.unwrap();
        for (i, x) in a.choices().iter().enumerate() {
            // verify that all model outputs are sane
            assert!(x < &thing[i].len())
        }

        prob.add_theory_clauses(&[ConflictClause::Unit(Decision {
            index: 0,
            choice: 0,
        })]);
        let assignments2 = prob.get_assignments();
        // verify that adding a constraint still returns a model
        assert!(assignments2.is_some());
        let a2 = assignments2.unwrap();
        // verify that the new constraint has caused the model to change
        assert_ne!(a, a2);
        // now add clauses to make the problem UNSAT
        prob.add_theory_clauses(&[
            ConflictClause::Unit(Decision {
                index: 0,
                choice: 1,
            }),
            ConflictClause::Unit(Decision {
                index: 0,
                choice: 2,
            }),
        ]);
        let assignments3 = prob.get_assignments();
        // verify that we do not get a model back
        assert!(assignments3.is_none());
    }
}
