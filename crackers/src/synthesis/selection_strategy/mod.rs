use jingle::modeling::ModeledBlock;
use z3::ast::Bool;
use z3::Context;

use crate::synthesis::Decision;
use crate::synthesis::pcode_theory::ConflictClause;
use crate::synthesis::slot_assignments::SlotAssignments;

// mod optimization_problem;
pub mod optimization_problem;
pub mod sat_problem;

pub trait SelectionStrategy<'ctx> {
    fn initialize(z3: &'ctx Context, gadgets: &Vec<Vec<ModeledBlock<'ctx>>>) -> Self;
    fn derive_var_name(target_index: usize, gadget_index: usize) -> String {
        format!("i{}_g{}", target_index, gadget_index)
    }

    fn get_assignments(&self) -> Option<SlotAssignments>;

    fn get_decision_variable(&self, var: &Decision) -> &Bool<'ctx>;

    fn add_theory_clauses(&mut self, clauses: &[ConflictClause]);
}
