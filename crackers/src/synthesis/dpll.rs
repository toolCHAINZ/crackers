use jingle::modeling::ModeledInstruction;
use z3::{Context, Solver};

use crate::gadget::Gadget;

#[derive(Debug, Clone)]
struct Decision {
    index: usize,
    choice: usize,
}

#[derive(Debug, Clone)]
struct AssignmentProblem<'ctx> {
    z3: &'ctx Context,
    solver: Solver<'ctx>,
    decisions: Vec<Decision>,
    gadgets: Vec<Gadget>,
    templates: Vec<ModeledInstruction<'ctx>>,
}

impl<'ctx> AssignmentProblem<'ctx> {
    pub fn new(z3: &'ctx Context) -> Self {
        let solver = Solver::new(z3);
        todo!()
    }

    pub fn decision_level(&self) -> usize {
        self.decisions.len()
    }
}
