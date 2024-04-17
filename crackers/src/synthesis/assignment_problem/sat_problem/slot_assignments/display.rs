use std::fmt::Display;
use crate::synthesis::assignment_problem::pcode_theory::ConflictClause;
use crate::synthesis::assignment_problem::sat_problem::slot_assignments::SlotAssignments;

struct SlotAssignmentConflictDisplay<'a>{
    assignment: &'a SlotAssignments,
    conflicts: &'a[ConflictClause]
}

/*impl<'a> Display for SlotAssignmentConflictDisplay<'a>{

}*/