use z3::ast::Bool;
use z3::{Context, Model, SatResult, Solver};

use crate::synthesis::assignment_problem::pcode_theory::ConflictClause;
use crate::synthesis::assignment_problem::Decision;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SlotAssignments {
    choices: Vec<usize>,
}

impl SlotAssignments {
    pub fn as_conflict_clause(&self) -> ConflictClause {
        ConflictClause::Conjunction(self.to_decisions())
    }
    pub fn to_decisions(&self) -> Vec<Decision> {
        let mut vec = Vec::with_capacity(self.choices.len());
        for (index, choice) in self.choices.iter().enumerate() {
            vec.push(Decision {
                index,
                choice: *choice,
            })
        }
        vec
    }
    pub fn choices(&self) -> &[usize] {
        self.choices.as_slice()
    }

    pub fn create_from_model<'ctx>(
        model: Model<'ctx>,
        variables: &[Vec<Bool<'ctx>>],
    ) -> Option<Self> {
        let mut choices = Vec::with_capacity(variables.len());
        for slot_choices in variables {
            let idx = slot_choices.iter().position(|v| {
                model
                    .eval(v, false)
                    .and_then(|b| b.as_bool())
                    .unwrap_or(false)
            });
            if let Some(idx) = idx {
                choices.push(idx);
            } else {
                return None;
            }
        }
        Some(Self { choices })
    }
}
#[derive(Debug, Clone)]
pub struct SatProblem<'ctx> {
    variables: Vec<Vec<Bool<'ctx>>>,
    z3: &'ctx Context,
    solver: Solver<'ctx>,
}

impl<'ctx> SatProblem<'ctx> {
    pub fn initialize<T>(z3: &'ctx Context, gadgets: &Vec<Vec<T>>) -> SatProblem<'ctx> {
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
    fn derive_var_name(target_index: usize, gadget_index: usize) -> String {
        format!("i{}_g{}", target_index, gadget_index)
    }

    pub fn get_assignments(&self) -> Option<SlotAssignments> {
        match self.solver.check() {
            SatResult::Unsat => None,
            SatResult::Unknown => {
                unreachable!("outer SAT solver timed out (this really shouldn't happen)!")
            }
            SatResult::Sat => {
                let model = self.solver.get_model()?;
                SlotAssignments::create_from_model(model, self.variables.as_slice())
            }
        }
    }

    fn get_decision_variable(&self, var: &Decision) -> &Bool<'ctx> {
        &self.variables[var.index][var.choice]
    }

    pub fn add_theory_clauses(&mut self, clauses: &[ConflictClause]) {
        let mut terms = Vec::new();
        for clause in clauses {
            match clause {
                ConflictClause::Unit(d) => {
                    let var = self.get_decision_variable(d);
                    terms.push(var.clone());
                }
                ConflictClause::Conjunction(v) => {
                    let choices: Vec<&Bool<'ctx>> =
                        v.iter().map(|b| self.get_decision_variable(b)).collect();
                    terms.push(Bool::and(self.z3, choices.as_slice()));
                }
            }
        }
        self.solver.assert(&Bool::or(self.z3, &terms).not());
    }
}

#[cfg(test)]
mod tests {
    use z3::{Config, Context};

    use crate::synthesis::assignment_problem::pcode_theory::ConflictClause;
    use crate::synthesis::assignment_problem::sat_problem::SatProblem;
    use crate::synthesis::assignment_problem::Decision;

    #[test]
    fn test_assignment() {
        let z3 = Context::new(&Config::new());
        let thing = vec![vec![1, 2, 3], vec![2, 3, 4], vec![3, 4, 5]];
        let mut prob = SatProblem::initialize(&z3, &thing);
        let assignments = prob.get_assignments();
        // Verify that an unconstrained problem returns a model
        assert!(assignments.is_some());
        let a = assignments.unwrap();
        for (i, x) in a.choices.iter().enumerate() {
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
