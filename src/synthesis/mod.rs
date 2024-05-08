use std::cmp::Ordering;
use std::collections::HashMap;

use tracing::{event, instrument, Level};
use z3::{Config, Context};

use crate::error::CrackersError;
use crate::error::CrackersError::EmptySpecification;
use crate::gadget::Gadget;
use crate::gadget::library::GadgetLibrary;
use crate::synthesis::builder::{SynthesisBuilder, SynthesisSelectionStrategy};
use crate::synthesis::pcode_theory::builder::PcodeTheoryBuilder;
use crate::synthesis::pcode_theory::ConflictClause;
use crate::synthesis::pcode_theory::theory_worker::TheoryWorker;
use crate::synthesis::selection_strategy::{OuterProblem, SelectionStrategy};
use crate::synthesis::selection_strategy::optimization_problem::OptimizationProblem;
use crate::synthesis::selection_strategy::OuterProblem::{OptimizeProb, SatProb};
use crate::synthesis::selection_strategy::sat_problem::SatProblem;
use crate::synthesis::slot_assignments::SlotAssignments;

pub mod assignment_model;
pub mod builder;
mod pcode_theory;
pub mod selection_strategy;
pub mod slot_assignments;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Decision {
    pub index: usize,
    pub choice: usize,
}

impl PartialOrd for Decision {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        self.index.partial_cmp(&other.index)
    }
}

#[derive(Debug)]
pub enum DecisionResult {
    ConflictsFound(SlotAssignments, Vec<ConflictClause>),
    AssignmentFound(SlotAssignments),
    Unsat,
}

pub struct AssignmentSynthesis<'ctx> {
    outer_problem: OuterProblem<'ctx>,
    library: GadgetLibrary,
    builder: SynthesisBuilder,
}

impl<'ctx> AssignmentSynthesis<'ctx> {
    #[instrument(skip_all)]
    pub fn new(
        z3: &'ctx Context,
        library: GadgetLibrary,
        builder: SynthesisBuilder,
    ) -> Result<Self, CrackersError> {
        let instrs = &builder.instructions;
        if instrs.is_empty() {
            return Err(EmptySpecification);
        }

        let mut gadget_candidates: Vec<Vec<&Gadget>> = vec![];
        for template in instrs.iter() {
            let candidates: Vec<&Gadget> = library
                .get_gadgets_for_instruction(z3, template)?
                .take(builder.candidates_per_slot)
                .collect();
            event!(
                Level::DEBUG,
                "Instruction {} has {} candidates",
                template.disassembly,
                candidates.len()
            );
            gadget_candidates.push(candidates);
        }
        let outer_problem = match builder.selection_strategy {
            SynthesisSelectionStrategy::SatStrategy => {
                SatProb(SatProblem::initialize(z3, &gadget_candidates))
            }
            SynthesisSelectionStrategy::OptimizeStrategy => {
                OptimizeProb(OptimizationProblem::initialize(z3, &gadget_candidates))
            }
        };

        Ok(AssignmentSynthesis {
            outer_problem,
            library,
            builder,
        })
    }

    #[instrument(skip_all)]
    pub fn decide(&mut self) -> Result<DecisionResult, CrackersError> {
        let mut req_channels = vec![];
        let theory_builder = PcodeTheoryBuilder::new(&self.library)
            .with_pointer_invariants(&self.builder.pointer_invariants)
            .with_preconditions(&self.builder.preconditions)
            .with_postconditions(&self.builder.postconditions)
            .with_max_candidates(self.builder.candidates_per_slot)
            .with_templates(self.builder.instructions.clone().into_iter());

        let (resp_sender, resp_receiver) = std::sync::mpsc::channel();
        std::thread::scope(|s| {
            let mut workers = vec![];
            for idx in 0..self.builder.parallel {
                let t = theory_builder.clone();
                let r = resp_sender.clone();
                let (req_sender, req_receiver) = std::sync::mpsc::channel();
                req_channels.push(req_sender);
                workers.push(s.spawn(move || -> Result<(), CrackersError> {
                    let z3 = Context::new(&Config::new());
                    let worker = TheoryWorker::new(&z3, idx, r, req_receiver, t).unwrap();
                    event!(Level::TRACE, "Created worker {}", idx);
                    worker.run();
                    Ok(())
                }));
            }

            let mut blacklist = HashMap::new();
            for (i, x) in req_channels.iter().enumerate() {
                let active: Vec<&SlotAssignments> = blacklist.values().collect();
                event!(
                    Level::TRACE,
                    "Asking outer procedure for initial assignments"
                );
                let assignment = self.outer_problem.get_assignments(&active).unwrap();
                event!(Level::TRACE, "Sending {:?} to worker {}", &assignment, i);
                blacklist.insert(i, assignment.clone());
                x.send(assignment).unwrap();
            }
            event!(Level::TRACE, "Done sending initial jobs");

            for response in resp_receiver {
                event!(
                    Level::TRACE,
                    "Received response from worker {}",
                    response.idx
                );

                match response.theory_result {
                    Ok(r) => {
                        match r {
                            None => {
                                event!(
                                    Level::INFO,
                                    "Theory returned SAT for {:?}!",
                                    response.assignment
                                );

                                req_channels = vec![];

                                return Ok(DecisionResult::AssignmentFound(response.assignment));
                            }
                            Some(c) => {
                                event!(
                                    Level::INFO,
                                    "Worker {} found conflicts: {}",
                                    response.idx,
                                    response.assignment.display_conflict(&c)
                                );
                                self.outer_problem.add_theory_clauses(&c);
                                let active: Vec<&SlotAssignments> = blacklist.values().collect();
                                let new_assignment = self.outer_problem.get_assignments(&active);
                                match new_assignment {
                                    None => {
                                        event!(
                                            Level::ERROR,
                                            "Outer SAT returned UNSAT! No solution found! :("
                                        );
                                        // drop the senders
                                        req_channels = vec![];

                                        return Ok(DecisionResult::Unsat);
                                    }
                                    Some(a) => {
                                        blacklist.insert(response.idx, a.clone());
                                        req_channels[response.idx].send(a).unwrap();
                                    }
                                }
                            }
                        }
                    }
                    Err(e) => {
                        event!(
                            Level::ERROR,
                            "Worker {} returned error: {}",
                            response.idx,
                            e
                        );
                        std::process::exit(-1);
                    }
                }
            }
            unreachable!()
        })
    }
}
