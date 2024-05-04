use jingle::modeling::ModeledBlock;
use z3::ast::{Ast, Bool};
use z3::{Context, Optimize, SatResult};

use crate::synthesis::pcode_theory::ConflictClause;
use crate::synthesis::selection_strategy::SelectionStrategy;
use crate::synthesis::slot_assignments::SlotAssignments;
use crate::synthesis::Decision;

#[derive(Debug)]
pub struct OptimizationProblem<'ctx> {
    variables: Vec<Vec<Bool<'ctx>>>,
    z3: &'ctx Context,
    solver: Optimize<'ctx>,
}

impl<'ctx> OptimizationProblem<'ctx> {
    pub(crate) fn initialize(z3: &'ctx Context, gadgets: &Vec<Vec<ModeledBlock<'ctx>>>) -> Self {
        let mut prob = Self {
            variables: Default::default(),
            z3,
            solver: Optimize::new(z3),
        };
        for (i, slot) in gadgets.iter().enumerate() {
            let mut vars = vec![];
            for (j, _) in slot.iter().enumerate() {
                let var = Bool::new_const(prob.z3, Self::derive_var_name(i, j));
                prob.solver
                    .assert_soft(&var.not(), gadgets[i][j].instructions.len(), None);
                vars.push(var)
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
}

impl<'ctx> SelectionStrategy<'ctx> for OptimizationProblem<'ctx> {
    fn get_assignments(&self) -> Option<SlotAssignments> {
        match self.solver.check(&[]) {
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

    fn add_theory_clauses(&mut self, clauses: &[ConflictClause]) {
        for clause in clauses {
            match clause {
                ConflictClause::Unit(d) => {
                    let var = self.get_decision_variable(d);
                    self.solver.assert(&var.not());
                }
                ConflictClause::Conjunction(v) => {
                    let choices: Vec<&Bool<'ctx>> =
                        v.iter().map(|b| self.get_decision_variable(b)).collect();
                    self.solver
                        .assert(&Bool::and(self.z3, choices.as_slice()).not().simplify());
                }
            }
        }
    }
}
