use std::ops::Deref;
use crate::gadget::Gadget;
use jingle::modeling::{ModeledBlock, ModeledInstruction};
use z3::ast::Bool;
use z3::Context;

use crate::synthesis::pcode_theory::ConflictClause;
use crate::synthesis::slot_assignments::SlotAssignments;
use crate::synthesis::Decision;
use crate::synthesis::selection_strategy::optimization_problem::OptimizationProblem;
use crate::synthesis::selection_strategy::sat_problem::SatProblem;

// mod optimization_problem;
pub mod optimization_problem;
pub mod sat_problem;

pub trait InstrLen {
    fn instr_len(&self) -> usize;
}

impl<'ctx> InstrLen for ModeledBlock<'ctx> {
    fn instr_len(&self) -> usize {
        self.instructions.len()
    }
}

impl InstrLen for Gadget {
    fn instr_len(&self) -> usize {
        self.instructions.len()
    }
}

impl<T: InstrLen> InstrLen for &T{
    fn instr_len(&self) -> usize {
        self.deref().instr_len()
    }
}
impl<'ctx> InstrLen for ModeledInstruction<'ctx> {
    fn instr_len(&self) -> usize {
        1
    }
}

pub trait SelectionStrategy<'ctx> {
    fn initialize<T: InstrLen>(z3: &'ctx Context, choices: &Vec<Vec<T>>) -> Self;

    fn get_assignments(&self, blacklist: &[&SlotAssignments]) -> Option<SlotAssignments>;

    fn add_theory_clauses(&mut self, clauses: &[ConflictClause]);

    fn derive_var_name(target_index: usize, gadget_index: usize) -> String {
        format!("i{}_g{}", target_index, gadget_index)
    }

}

pub enum OuterProblem<'ctx>{
    SatProb(SatProblem<'ctx>),
    OptimizeProb(OptimizationProblem<'ctx>)
}

impl<'ctx> OuterProblem<'ctx>{
    pub(crate) fn get_assignments(&self, blacklist: &[&SlotAssignments]) -> Option<SlotAssignments>{
        match self{
            OuterProblem::SatProb(s) => s.get_assignments(blacklist),
            OuterProblem::OptimizeProb(o) => o.get_assignments(blacklist)
        }
    }

    pub(crate) fn add_theory_clauses(&mut self, clauses: &[ConflictClause]){
        match self{
            OuterProblem::SatProb(s)=>s.add_theory_clauses(clauses),
            OuterProblem::OptimizeProb(o)=>o.add_theory_clauses(clauses)
        }
    }

}