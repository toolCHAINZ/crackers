use jingle::modeling::{ModeledBlock, ModeledInstruction};
use jingle::sleigh::Instruction;
use z3::{Context, Solver};

use crate::gadget::GadgetLibrary;

mod sat_problem;

#[derive(Debug, Clone)]
pub struct Decision {
    pub index: usize,
    pub choice: usize,
}

#[derive(Debug, Clone)]
pub struct AssignmentProblem<'ctx> {
    z3: &'ctx Context,
    solver: Solver<'ctx>,
    decisions: Vec<Decision>,
    templates: Vec<ModeledInstruction<'ctx>>,
    library: GadgetLibrary,
    gadget_candidates: Vec<Vec<ModeledBlock<'ctx>>>,
}

impl<'ctx> AssignmentProblem<'ctx> {
    pub fn new(z3: &'ctx Context, templates: Vec<Instruction>, library: GadgetLibrary) -> Self {
        let mut prob = AssignmentProblem {
            z3,
            solver: Solver::new(z3),
            library,
            decisions: Default::default(),
            templates: Default::default(),
            gadget_candidates: Default::default(),
        };
        for template in templates.iter() {
            prob.templates
                .push(ModeledInstruction::new(template.clone(), &prob.library, z3).unwrap());
            prob.gadget_candidates.push(
                prob.library
                    .get_modeled_gadgets_for_instruction(z3, &template)
                    .collect(),
            );
        }
        prob
    }

    pub fn decision_level(&self) -> usize {
        self.decisions.len()
    }
}
