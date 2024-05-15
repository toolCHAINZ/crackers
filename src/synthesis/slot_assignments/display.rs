use std::fmt::{Display, Formatter};

use colored::Colorize;

use crate::synthesis::pcode_theory::conflict_clause::ConflictClause;
use crate::synthesis::slot_assignments::SlotAssignments;

pub(crate) struct SlotAssignmentConflictDisplay<'a> {
    pub(crate) assignment: &'a SlotAssignments,
    pub(crate) conflict: &'a ConflictClause,
}

impl<'a> Display for SlotAssignmentConflictDisplay<'a> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "[")?;
        let unit = match self.conflict{
            ConflictClause::Unit(_) => true,
            ConflictClause::Conjunction(c)=>c.len() == 1
        };
        for (i, assignment) in self.assignment.choices.iter().enumerate() {
            let token = if self.conflict.includes_index(i){
                if unit {
                    format!("{:04}", assignment).red()
                }else{
                    format!("{:04}", assignment).yellow()
                }
            }else{
                format!("{:04}", assignment).normal()
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
