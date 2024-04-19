use z3::ast::Bool;

use crate::synthesis::Decision;
use crate::synthesis::pcode_theory::ConflictClause;
use crate::synthesis::slot_assignments::SlotAssignments;

// mod optimization_problem;
pub mod optimization_problem;
pub mod sat_problem;

pub trait SelectionStrategy {

    fn get_assignments(&self) -> Option<SlotAssignments>;

    fn get_decision_variable<'ctx>(&self, var: &Decision) -> &Bool<'ctx>;

    fn add_theory_clauses(&mut self, clauses: &[ConflictClause]);
}
