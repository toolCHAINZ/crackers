use std::fmt::{Display, Formatter};

use colored::Colorize;

use crate::synthesis::assignment_problem::pcode_theory::ConflictClause;
use crate::synthesis::assignment_problem::slot_assignments::SlotAssignments;

pub(crate) struct SlotAssignmentConflictDisplay<'a> {
    pub(crate) assignment: &'a SlotAssignments,
    pub(crate) conflicts: &'a [ConflictClause],
}

impl<'a> Display for SlotAssignmentConflictDisplay<'a> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "[")?;
        for (i, assignment) in self.assignment.choices.iter().enumerate() {
            let token = if self.conflicts.iter().any(|c| match c {
                ConflictClause::Unit(c) => c.index == i,
                ConflictClause::Conjunction(_) => false,
            }) {
                format!("{}", assignment).red()
            } else if self.conflicts.iter().any(|c| match c {
                ConflictClause::Unit(_) => false,
                ConflictClause::Conjunction(cc) => cc.iter().any(|d| d.index == i),
            }) {
                format!("{}", assignment).yellow()
            } else {
                format!("{}", assignment).normal()
            };
            if i > 0 {
                write!(f, ", {}", token)?;
            } else {
                write!(f, "{}", token)?;
            }
        }
        write!(f, "]")
    }
}
