use jingle::modeling::{ModeledBlock, ModeledInstruction};
use jingle::sleigh::Instruction;
use tracing::{event, instrument, Level};
use z3::{Context, Solver};

use crate::error::CrackersError;
use crate::gadget::GadgetLibrary;
use crate::synthesis::assignment_problem::pcode_theory::{ConflictClause, PcodeTheory};
use crate::synthesis::assignment_problem::sat_problem::{SatProblem, SlotAssignments};

mod pcode_theory;
mod sat_problem;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Decision {
    pub index: usize,
    pub choice: usize,
}

#[derive(Debug, Clone)]
pub enum DecisionResult {
    ConflictsFound(SlotAssignments, Vec<ConflictClause>),
    AssignmentFound(SlotAssignments),
    Unsat,
}

#[derive(Debug, Clone)]
pub struct AssignmentProblem<'ctx> {
    z3: &'ctx Context,
    solver: Solver<'ctx>,
    library: GadgetLibrary,
    templates: Vec<ModeledInstruction<'ctx>>,
    gadget_candidates: Vec<Vec<ModeledBlock<'ctx>>>,
    sat_problem: SatProblem<'ctx>,
    theory_problem: PcodeTheory<'ctx>,
}

impl<'ctx> AssignmentProblem<'ctx> {
    #[instrument(skip_all)]
    pub fn new(z3: &'ctx Context, templates: Vec<Instruction>, library: GadgetLibrary) -> Self {
        let mut modeled_templates = vec![];
        let mut gadget_candidates: Vec<Vec<ModeledBlock<'ctx>>> = vec![];
        for template in templates.iter() {
            modeled_templates
                .push(ModeledInstruction::new(template.clone(), &library, z3).unwrap());
            let candidates: Vec<ModeledBlock<'ctx>> = library
                .get_modeled_gadgets_for_instruction(z3, &template)
                // todo: just here to make testing faster. Remove this later
                .take(10)
                .collect();
            event!(
                Level::DEBUG,
                "Instruction {} has {} candidates",
                template.disassembly,
                candidates.len()
            );
            gadget_candidates.push(candidates);
        }
        let sat_problem = SatProblem::initialize(z3, &gadget_candidates);
        let theory_problem = PcodeTheory::new(z3, modeled_templates.as_slice(), &gadget_candidates);
        AssignmentProblem {
            z3,
            solver: Solver::new(z3),
            library,
            templates: modeled_templates,
            gadget_candidates,
            sat_problem,
            theory_problem,
        }
    }
    fn single_decision_iteration(&mut self) -> Result<DecisionResult, CrackersError> {
        let assignment = self.sat_problem.get_assignments();
        if let Some(a) = assignment {
            let conflicts = self.theory_problem.check_assignment(&a)?;
            if let Some(c) = conflicts {
                self.sat_problem.add_theory_clauses(&c);
                Ok(DecisionResult::ConflictsFound(a, c))
            } else {
                Ok(DecisionResult::AssignmentFound(a))
            }
        } else {
            Ok(DecisionResult::Unsat)
        }
    }

    #[instrument(skip_all)]
    pub fn decide(&mut self) -> Result<DecisionResult, CrackersError> {
        let mut keep_going = true;
        while keep_going {
            let res = self.single_decision_iteration()?;
            match res {
                DecisionResult::ConflictsFound(a, c) => {
                    event!(Level::DEBUG, "{:?} has {} conflicts", a, c.len());
                    continue;
                }
                DecisionResult::AssignmentFound(a) => {
                    event!(Level::DEBUG, "{:?} is feasible", a);
                    return Ok(DecisionResult::AssignmentFound(a));
                }
                DecisionResult::Unsat => {
                    event!(Level::DEBUG, "No assignment exists");
                    return Ok(DecisionResult::Unsat);
                }
            }
        }
        unreachable!()
    }
}
