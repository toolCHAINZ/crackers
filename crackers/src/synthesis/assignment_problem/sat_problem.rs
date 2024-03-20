use z3::ast::Bool;
use z3::{Context, Model, SatResult, Solver};

pub struct SlotAssignments {
    choices: Vec<usize>,
}

impl SlotAssignments {
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
pub struct SatProblem<'ctx> {
    variables: Vec<Vec<Bool<'ctx>>>,
    z3: &'ctx Context,
    solver: Solver<'ctx>,
}

impl<'ctx> SatProblem<'ctx> {
    fn initialize<T>(z3: &'ctx Context, gadgets: &Vec<Vec<T>>) {
        let mut prob = SatProblem {
            variables: Default::default(),
            z3,
            solver: Solver::new(z3),
        };
        for (i, slot) in gadgets.iter().enumerate() {
            let mut vars = vec![];
            for (j, _) in slot.iter().enumerate() {
                vars.push(Bool::new_const(prob.z3, SatProblem::derive_var_name(i, j)))
            }
            prob.variables.push(vars);
        }
        for slot in prob.variables {
            let pbs: Vec<(&Bool<'ctx>, i32)> = slot.iter().map(|b| (b, 1)).collect();
            prob.solver.assert(&Bool::pb_eq(z3, &pbs, 1))
        }
    }
    fn derive_var_name(target_index: usize, gadget_index: usize) -> String {
        format!("i{}_g{}", target_index, gadget_index)
    }

    fn get_assignments(&self) -> Option<SlotAssignments> {
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
}
