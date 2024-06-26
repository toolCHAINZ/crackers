use jingle::modeling::{ModeledBlock, ModeledInstruction};
use z3::Context;

use crate::error::CrackersError;
use crate::gadget::Gadget;
use crate::synthesis::pcode_theory::conflict_clause::ConflictClause;
use crate::synthesis::selection_strategy::optimization_problem::OptimizationProblem;
use crate::synthesis::selection_strategy::sat_problem::SatProblem;
use crate::synthesis::slot_assignments::SlotAssignments;

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

impl<T: InstrLen> InstrLen for &T {
    fn instr_len(&self) -> usize {
        (*self).instr_len()
    }
}
impl<'ctx> InstrLen for ModeledInstruction<'ctx> {
    fn instr_len(&self) -> usize {
        1
    }
}

#[derive(Clone, Debug)]
pub enum AssignmentResult{
    Success(SlotAssignments),
    Failure(SelectionFailure)
}
#[derive(Clone, Debug)]
pub struct SelectionFailure{
    indexes: Vec<usize>
}
pub trait SelectionStrategy<'ctx> {
    fn initialize<T: InstrLen>(z3: &'ctx Context, choices: &[Vec<T>]) -> Self;

    fn get_assignments(&mut self) -> Result<AssignmentResult, CrackersError>;

    fn add_theory_clauses(&mut self, clause: &ConflictClause);

    fn derive_var_name(target_index: usize, gadget_index: usize) -> String {
        format!("i{}_g{}", target_index, gadget_index)
    }
}

pub enum OuterProblem<'ctx> {
    SatProb(SatProblem<'ctx>),
    OptimizeProb(OptimizationProblem<'ctx>),
}

impl<'ctx> OuterProblem<'ctx> {
    pub(crate) fn get_assignments(&mut self) -> Result<AssignmentResult, CrackersError> {
        match self {
            OuterProblem::SatProb(s) => s.get_assignments(),
            OuterProblem::OptimizeProb(o) => o.get_assignments(),
        }
    }

    pub(crate) fn add_theory_clauses(&mut self, clauses: &ConflictClause) {
        match self {
            OuterProblem::SatProb(s) => s.add_theory_clauses(clauses),
            OuterProblem::OptimizeProb(o) => o.add_theory_clauses(clauses),
        }
    }
}
