use std::cmp::Ordering;
use std::collections::HashMap;

use jingle::modeling::{ModeledBlock, ModelingContext};
use tracing::{event, instrument, Level};
use z3::{Config, Context, Solver};

use pcode_theory::conflict_clause::ConflictClause;

use crate::error::CrackersError;
use crate::error::CrackersError::EmptySpecification;
use crate::gadget::Gadget;
use crate::gadget::library::GadgetLibrary;
use crate::synthesis::assignment_model::AssignmentModel;
use crate::synthesis::builder::{SynthesisBuilder, SynthesisSelectionStrategy};
use crate::synthesis::pcode_theory::builder::PcodeTheoryBuilder;
use crate::synthesis::pcode_theory::pcode_assignment::PcodeAssignment;
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
pub enum DecisionResult<'ctx, T: ModelingContext<'ctx>> {
    ConflictsFound(SlotAssignments, Vec<ConflictClause>),
    AssignmentFound(AssignmentModel<'ctx, T>),
    Unsat,
}

pub struct AssignmentSynthesis<'ctx> {
    z3: &'ctx Context,
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
            z3,
            outer_problem,
            library,
            builder,
        })
    }

    #[instrument(skip_all)]
    pub fn decide(&mut self) -> Result<DecisionResult<'ctx, ModeledBlock<'ctx>>, CrackersError> {
        let mut req_channels = vec![];
        let theory_builder = PcodeTheoryBuilder::new(&self.library)
            .with_pointer_invariants(&self.builder.pointer_invariants)
            .with_preconditions(&self.builder.preconditions)
            .with_postconditions(&self.builder.postconditions)
            .with_max_candidates(self.builder.candidates_per_slot)
            .with_templates(self.builder.instructions.clone().into_iter());

        let (resp_sender, resp_receiver) = std::sync::mpsc::channel();
        std::thread::scope(|s| {
            for idx in 0..self.builder.parallel {
                let t = theory_builder.clone();
                let r = resp_sender.clone();
                let (req_sender, req_receiver) = std::sync::mpsc::channel();
                req_channels.push(req_sender);
                s.spawn(move || -> Result<(), CrackersError> {
                    let z3 = Context::new(&Config::new());
                    let worker = TheoryWorker::new(&z3, idx, r, req_receiver, t).unwrap();
                    event!(Level::TRACE, "Created worker {}", idx);
                    worker.run();
                    std::mem::drop(worker);
                    Ok(())
                });
            }
            std::mem::drop(resp_sender);
            for (i, x) in req_channels.iter().enumerate() {
                event!(
                    Level::TRACE,
                    "Asking outer procedure for initial assignments"
                );
                if let Some(assignment) = self.outer_problem.get_assignments() {
                    event!(Level::TRACE, "Sending {:?} to worker {}", &assignment, i);
                    x.send(assignment).unwrap();
                }
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
                                dbg!("huh");
                                req_channels.clear();
                                let t = theory_builder.clone();
                                let a :PcodeAssignment<'ctx> = t.build_assignment(self.z3, response.assignment)?;
                                dbg!("huh2");
                                let solver = Solver::new(self.z3);
                                let model = a.check(self.z3, &solver)?;
                                dbg!("here");
                                return Ok(DecisionResult::AssignmentFound(model));
                            }
                            Some(c) => {
                                event!(
                                    Level::INFO,
                                    "Worker {} found conflicts: {}",
                                    response.idx,
                                    response.assignment.display_conflict(&c)
                                );
                                self.outer_problem.add_theory_clauses(&c);
                                let new_assignment = self.outer_problem.get_assignments();
                                match new_assignment {
                                    None => {
                                        // drop the senders
                                        req_channels.clear();
                                    }
                                    Some(a) => {
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
            event!(
                Level::ERROR,
                "Outer SAT returned UNSAT! No solution found! :("
            );
            return Ok(DecisionResult::Unsat);
        })
    }
}
