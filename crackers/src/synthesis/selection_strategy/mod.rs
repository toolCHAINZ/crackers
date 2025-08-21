use jingle::modeling::{ModeledBlock, ModeledInstruction};
#[cfg(feature = "pyo3")]
use pyo3::pyclass;

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

impl InstrLen for ModeledBlock {
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
impl InstrLen for ModeledInstruction {
    fn instr_len(&self) -> usize {
        1
    }
}

impl InstrLen for i32 {
    fn instr_len(&self) -> usize {
        *self as usize
    }
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AssignmentResult {
    Success(SlotAssignments),
    Failure(SelectionFailure),
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "pyo3", pyclass)]
pub struct SelectionFailure {
    pub indices: Vec<usize>,
}
pub trait SelectionStrategy {
    fn initialize<T: InstrLen>(choices: &[Vec<T>]) -> Self;

    fn get_assignments(&mut self) -> Result<AssignmentResult, CrackersError>;

    fn add_theory_clause(&mut self, clause: &ConflictClause);

    fn derive_var_name(target_index: usize, gadget_index: usize) -> String {
        format!("i{target_index}_g{gadget_index}")
    }
}

pub enum OuterProblem {
    SatProb(SatProblem),
    OptimizeProb(OptimizationProblem),
}

impl OuterProblem {
    pub(crate) fn get_assignments(&mut self) -> Result<AssignmentResult, CrackersError> {
        match self {
            OuterProblem::SatProb(s) => s.get_assignments(),
            OuterProblem::OptimizeProb(o) => o.get_assignments(),
        }
    }

    pub(crate) fn add_theory_clauses(&mut self, clauses: &ConflictClause) {
        match self {
            OuterProblem::SatProb(s) => s.add_theory_clause(clauses),
            OuterProblem::OptimizeProb(o) => o.add_theory_clause(clauses),
        }
    }
}
