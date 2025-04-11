use std::fmt::{Display, Formatter};

use colored::Colorize;

use crate::synthesis::pcode_theory::conflict_clause::ConflictClause;
use crate::synthesis::slot_assignments::SlotAssignments;

pub(crate) struct SlotAssignmentConflictDisplay<'a> {
    pub(crate) assignment: &'a SlotAssignments,
    pub(crate) conflict: &'a ConflictClause,
}

impl Display for SlotAssignmentConflictDisplay<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self.conflict.precondition {
            true => write!(f, "!")?,
            false => write!(f, " ")?,
        };
        write!(f, "[")?;
        let unit = self.conflict.decisions().len() == 1;
        for (i, assignment) in self.assignment.choices.iter().enumerate() {
            let token = if self.conflict.includes_index(i) {
                if unit {
                    format!("{:04}", assignment).red()
                } else {
                    format!("{:04}", assignment).yellow()
                }
            } else {
                format!("{:04}", assignment).normal()
            };
            if i > 0 {
                write!(f, ", {}", token)?;
            } else {
                write!(f, "{}", token)?;
            }
        }
        write!(f, "]")?;
        match self.conflict.postcondition {
            true => write!(f, "!")?,
            false => write!(f, " ")?,
        };
        Ok(())
    }
}
