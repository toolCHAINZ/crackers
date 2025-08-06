use std::fmt::{Display, Formatter};

use z3::Model;
use z3::ast::Bool;

use crate::error::CrackersError;
use crate::error::CrackersError::ModelParsingError;
use crate::gadget::Gadget;
use crate::gadget::candidates::Candidates;
use crate::synthesis::Decision;
use crate::synthesis::pcode_theory::conflict_clause::ConflictClause;
use crate::synthesis::slot_assignments::display::SlotAssignmentConflictDisplay;

mod display;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SlotAssignments {
    choices: Vec<usize>,
}

impl SlotAssignments {
    pub fn as_conflict_clause(&self) -> ConflictClause {
        ConflictClause::from(self.to_decisions().iter())
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

    pub fn choice(&self, idx: usize) -> usize {
        self.choices[idx]
    }
    pub fn choices(&self) -> &[usize] {
        self.choices.as_slice()
    }

    pub fn create_from_model(model: Model, variables: &[Vec<Bool>]) -> Result<Self, CrackersError> {
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
                return Err(ModelParsingError);
            }
        }
        Ok(Self { choices })
    }

    pub(crate) fn display_conflict<'a>(
        &'a self,
        conflicts: &'a ConflictClause,
    ) -> SlotAssignmentConflictDisplay<'a> {
        SlotAssignmentConflictDisplay {
            assignment: self,
            conflict: conflicts,
        }
    }

    pub fn interpret_from_library(&self, candidates: &Candidates) -> Vec<Gadget> {
        self.choices()
            .iter()
            .copied()
            .enumerate()
            .map(|(index, selection)| candidates.candidates[index][selection].clone())
            .collect()
    }
}

impl Display for SlotAssignments {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "[")?;
        for (i, assignment) in self.choices.iter().enumerate() {
            let token = format!("{assignment:04}");
            if i > 0 {
                write!(f, ", {token}")?;
            } else {
                write!(f, "{token}")?;
            }
        }
        write!(f, "]")
    }
}
