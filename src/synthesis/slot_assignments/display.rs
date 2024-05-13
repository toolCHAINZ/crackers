use std::fmt::{Display, Formatter};

use colored::Colorize;

use crate::synthesis::pcode_theory::conflict_clause::ConflictClause;
use crate::synthesis::slot_assignments::SlotAssignments;

pub(crate) struct SlotAssignmentConflictDisplay<'a> {
    pub(crate) assignment: &'a SlotAssignments,
    pub(crate) conflicts: &'a [ConflictClause],
}

impl<'a> Display for SlotAssignmentConflictDisplay<'a> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "[")?;
        let num_conflicts = self
            .conflicts
            .iter()
            .map(|c| c.decisions().len())
            .reduce(|a, b| a + b)
            .unwrap_or(1);
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
                if num_conflicts > 1 {
                    format!("{}", assignment).yellow()
                } else {
                    format!("{}", assignment).red()
                }
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
