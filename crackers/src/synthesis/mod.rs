use jingle::modeling::ModeledInstruction;
use std::cmp::Ordering;
use std::sync::Arc;
use tracing::{Level, event, instrument};
use z3::Context;

use crate::error::CrackersError;
use crate::error::CrackersError::EmptySpecification;
use crate::gadget::candidates::{CandidateBuilder, Candidates};
use crate::gadget::library::GadgetLibrary;
use crate::reference_program::ReferenceProgram;
use crate::synthesis::assignment_model::builder::AssignmentModelBuilder;
use crate::synthesis::builder::{
    StateConstraintGenerator, SynthesisParams, SynthesisSelectionStrategy,
    TransitionConstraintGenerator,
};
use crate::synthesis::pcode_theory::builder::PcodeTheoryBuilder;
use crate::synthesis::pcode_theory::theory_worker::TheoryWorker;
use crate::synthesis::selection_strategy::AssignmentResult::{Failure, Success};
use crate::synthesis::selection_strategy::OuterProblem::{OptimizeProb, SatProb};
use crate::synthesis::selection_strategy::optimization_problem::OptimizationProblem;
use crate::synthesis::selection_strategy::sat_problem::SatProblem;
use crate::synthesis::selection_strategy::{OuterProblem, SelectionFailure, SelectionStrategy};
use crate::synthesis::slot_assignments::SlotAssignments;

pub mod assignment_model;
pub mod builder;
mod combined;
pub(crate) mod partition_iterator;
pub mod pcode_theory;
pub mod selection_strategy;
pub mod slot_assignments;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
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
    AssignmentFound(AssignmentModelBuilder),
    Unsat(SelectionFailure),
}

pub struct AssignmentSynthesis {
    outer_problem: OuterProblem,
    library: Arc<GadgetLibrary>,
    candidates: Candidates,
    pointer_invariants: Vec<Arc<TransitionConstraintGenerator>>,
    preconditions: Vec<Arc<StateConstraintGenerator>>,
    postconditions: Vec<Arc<StateConstraintGenerator>>,
    candidates_per_slot: usize,
    instructions: ReferenceProgram,
    parallel: usize,
}

impl AssignmentSynthesis {
    pub fn new(builder: &SynthesisParams) -> Result<Self, CrackersError> {
        let instrs = &builder.reference_program;
        if instrs.is_empty() {
            return Err(EmptySpecification);
        }
        let arch_info = &builder.gadget_library.arch_info();
        let modeled_instrs: Vec<ModeledInstruction> = instrs
            .steps()
            .iter()
            .map(|i| i.model(arch_info).unwrap())
            .collect();

        let candidates = CandidateBuilder::default()
            .with_random_sample_size(builder.candidates_per_slot)
            .build(builder.gadget_library.get_random_candidates_for_trace(
                arch_info,
                modeled_instrs.as_slice(),
                builder.seed,
            ))?;
        let outer_problem = match builder.selection_strategy {
            SynthesisSelectionStrategy::SatStrategy => {
                SatProb(SatProblem::initialize(&candidates.candidates))
            }
            SynthesisSelectionStrategy::OptimizeStrategy => {
                OptimizeProb(OptimizationProblem::initialize(&candidates.candidates))
            }
        };
        Ok(AssignmentSynthesis {
            outer_problem,
            candidates,
            library: builder.gadget_library.clone(),
            pointer_invariants: builder.pointer_invariants.clone(),
            preconditions: builder.preconditions.clone(),
            postconditions: builder.postconditions.clone(),
            candidates_per_slot: builder.candidates_per_slot,
            instructions: builder.reference_program.clone(),
            parallel: builder.parallel,
        })
    }

    fn make_model_builder(&self, slot_assignments: SlotAssignments) -> AssignmentModelBuilder {
        AssignmentModelBuilder {
            templates: self.instructions.clone(),
            gadgets: slot_assignments.interpret_from_library(&self.candidates),
            preconditions: self.preconditions.clone(),
            postconditions: self.postconditions.clone(),
            pointer_invariants: self.pointer_invariants.clone(),
            arch_info: self.library.arch_info(),
        }
    }

    fn make_pcode_theory_builder(&self) -> PcodeTheoryBuilder<'_> {
        PcodeTheoryBuilder::new(self.candidates.clone(), &self.library)
            .with_pointer_invariants(&self.pointer_invariants)
            .with_preconditions(&self.preconditions)
            .with_postconditions(&self.postconditions)
            .with_max_candidates(self.candidates_per_slot)
            .with_templates(self.instructions.clone())
    }

    pub fn decide_single_threaded(&mut self) -> Result<DecisionResult, CrackersError> {
        let theory_builder = self.make_pcode_theory_builder();
        let theory = theory_builder.build()?;
        loop {
            let assignment = self.outer_problem.get_assignments()?;
            match assignment {
                Success(a) => {
                    let theory_result = theory.check_assignment(&a)?;
                    match theory_result {
                        None => {
                            // success
                            return Ok(DecisionResult::AssignmentFound(self.make_model_builder(a)));
                        }
                        Some(conflict) => {
                            self.outer_problem.add_theory_clauses(&conflict);
                        }
                    }
                }
                Failure(d) => return Ok(DecisionResult::Unsat(d)),
            }
        }
    }
    #[instrument(skip_all)]
    pub fn decide(&mut self) -> Result<DecisionResult, CrackersError> {
        let mut req_channels = vec![];
        let mut kill_senders = vec![];
        let library = self.library.clone();
        let theory_builder = PcodeTheoryBuilder::new(self.candidates.clone(), &library)
            .with_pointer_invariants(&self.pointer_invariants)
            .with_preconditions(&self.preconditions)
            .with_postconditions(&self.postconditions)
            .with_max_candidates(self.candidates_per_slot)
            .with_templates(self.instructions.clone());
        let (resp_sender, resp_receiver) = std::sync::mpsc::channel();
        std::thread::scope(|s| {
            for idx in 0..self.parallel {
                let t = theory_builder.clone();
                let r = resp_sender.clone();
                let (req_sender, req_receiver) = std::sync::mpsc::channel();
                let (kill_sender, kill_receiver) = std::sync::mpsc::channel();
                kill_senders.push(kill_sender);
                req_channels.push(req_sender);
                s.spawn(move || {
                    let z3 = Context::thread_local();
                    std::thread::scope(|inner| {
                        let handle = z3.handle();
                        inner.spawn(move || {
                            for _ in kill_receiver {
                                handle.interrupt();
                            }
                        });
                        let worker = TheoryWorker::new(idx, r, req_receiver, t).unwrap();
                        event!(Level::TRACE, "Created worker {}", idx);
                        worker.run();
                        drop(worker);
                    });
                });
            }
            drop(resp_sender);
            for (i, x) in req_channels.iter().enumerate() {
                event!(
                    Level::TRACE,
                    "Asking outer procedure for initial assignments"
                );
                if let Ok(assignment) = self.outer_problem.get_assignments() {
                    match assignment {
                        Success(assignment) => {
                            event!(Level::TRACE, "Sending {:?} to worker {}", &assignment, i);
                            x.send(assignment).unwrap();
                        }
                        Failure(a) => {
                            req_channels.clear();
                            for x in &kill_senders {
                                x.send(()).unwrap();
                            }
                            kill_senders.clear();
                            return Ok(DecisionResult::Unsat(a));
                        }
                    }
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
                                req_channels.clear();
                                for x in &kill_senders {
                                    x.send(()).unwrap();
                                }
                                kill_senders.clear();
                                return Ok(DecisionResult::AssignmentFound(
                                    self.make_model_builder(response.assignment),
                                ));
                            }
                            Some(c) => {
                                event!(
                                    Level::TRACE,
                                    "Worker {} found conflicts: {}",
                                    response.idx,
                                    response.assignment.display_conflict(&c)
                                );
                                self.outer_problem.add_theory_clauses(&c);
                                let new_assignment = self.outer_problem.get_assignments()?;
                                match new_assignment {
                                    Failure(a) => {
                                        // drop the senders
                                        req_channels.clear();
                                        for x in &kill_senders {
                                            x.send(()).unwrap();
                                        }
                                        kill_senders.clear();
                                        return Ok(DecisionResult::Unsat(a));
                                    }
                                    Success(a) => {
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
            unreachable!()
        })
    }
}
