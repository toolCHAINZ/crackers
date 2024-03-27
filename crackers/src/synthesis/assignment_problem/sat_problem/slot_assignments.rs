use z3::Model;
use z3::ast::Bool;
use crate::synthesis::assignment_problem::Decision;
use crate::synthesis::assignment_problem::pcode_theory::ConflictClause;

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
