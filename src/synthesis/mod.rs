use std::cmp::Ordering;
use std::io::Write;
use std::{fs, mem};

use jingle::modeling::{ModeledBlock, ModeledInstruction};
use jingle::sleigh::Instruction;
use jingle::JingleError;
use tracing::{event, instrument, Level};
use z3::Context;

use crate::error::CrackersError;
use crate::error::CrackersError::{EmptySpecification, ModelGenerationError};
use crate::gadget::library::builder::GadgetLibraryBuilder;
use crate::gadget::library::GadgetLibrary;
use crate::synthesis::assignment_model::AssignmentModel;
use crate::synthesis::builder::{SynthesisBuilder, SynthesisSelectionStrategy};
use crate::synthesis::pcode_theory::{ConflictClause, PcodeTheory};
use crate::synthesis::selection_strategy::optimization_problem::OptimizationProblem;
use crate::synthesis::selection_strategy::sat_problem::SatProblem;
use crate::synthesis::selection_strategy::{sat_problem, SelectionStrategy};
use crate::synthesis::slot_assignments::SlotAssignments;

pub mod assignment_model;
pub mod builder;
mod pcode_theory;
pub mod selection_strategy;
pub mod slot_assignments;

#[derive(Debug, Clone, PartialEq, Eq, Ord)]
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
pub enum DecisionResult<'ctx> {
    ConflictsFound(SlotAssignments, Vec<ConflictClause>),
    AssignmentFound(AssignmentModel<'ctx>),
    Unsat,
}

pub struct AssignmentSynthesis<'ctx> {
    gadget_candidates: Vec<Vec<ModeledBlock<'ctx>>>,
    sat_problem: Box<dyn SelectionStrategy<'ctx> + 'ctx>,
    theory_problem: PcodeTheory<'ctx>,
}

impl<'ctx> AssignmentSynthesis<'ctx> {
    #[instrument(skip_all)]
    pub fn new(
        z3: &'ctx Context,
        library: GadgetLibrary,
        builder: SynthesisBuilder<'ctx>,
    ) -> Result<Self, CrackersError> {
        let instrs: Vec<Instruction> = builder.instructions;
        if instrs.len() == 0 {
            return Err(EmptySpecification);
        }

        let mut modeled_templates = vec![];
        let mut gadget_candidates: Vec<Vec<ModeledBlock<'ctx>>> = vec![];
        for template in instrs.iter() {
            modeled_templates
                .push(ModeledInstruction::new(template.clone(), &library, z3).unwrap());
            let candidates: Vec<ModeledBlock<'ctx>> = library
                .get_modeled_gadgets_for_instruction(z3, &template)
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
        let outer: Box<dyn SelectionStrategy<'ctx>>;
        match builder.selection_strategy {
            SynthesisSelectionStrategy::SatStrategy => {
                outer = Box::new(SatProblem::initialize(z3, &gadget_candidates));
            }
            SynthesisSelectionStrategy::OptimizeStrategy => {
                outer = Box::new(OptimizationProblem::initialize(z3, &gadget_candidates));
            }
        };
        let theory_problem = PcodeTheory::new(
            z3,
            modeled_templates.as_slice(),
            &gadget_candidates,
            builder.preconditions,
            builder.postconditions,
            builder.pointer_invariants,
        )?;
        Ok(AssignmentSynthesis {
            gadget_candidates,
            sat_problem: outer,
            theory_problem,
        })
    }
    fn single_decision_iteration(&mut self) -> Result<DecisionResult<'ctx>, CrackersError> {
        event!(Level::TRACE, "checking SAT problem");
        let assignment = self.sat_problem.get_assignments();
        if let Some(a) = assignment {
            event!(Level::TRACE, "checking theory problem");

            let conflicts = self.theory_problem.check_assignment(&a);
            match conflicts {
                Ok(conflicts) => {
                    if let Some(c) = conflicts {
                        event!(Level::TRACE, "theory returned {} conjunctions", c.len());

                        self.sat_problem.add_theory_clauses(&c);
                        Ok(DecisionResult::ConflictsFound(a, c))
                    } else {
                        event!(Level::DEBUG, "theory returned SAT");
                        let model = self
                            .theory_problem
                            .get_model()
                            .ok_or(ModelGenerationError)?;
                        let gadgets = self.gadgets_for_assignment(&a);
                        Ok(DecisionResult::AssignmentFound(AssignmentModel::new(
                            a, model, gadgets,
                        )))
                    }
                }
                Err(err) => match err {
                    CrackersError::TheoryTimeout => {
                        event!(Level::WARN, "{:?} timed out", &a);
                        let c = a.as_conflict_clause();
                        self.sat_problem
                            .add_theory_clauses(&[a.as_conflict_clause()]);
                        let mut f =
                            fs::File::create(format!("dumps/gadgets_{:?}.txt", a.choices()))
                                .unwrap();
                        for b in self.gadgets_for_assignment(&a) {
                            f.write(format!("{}", b).as_ref())
                                .expect("TODO: panic message");
                        }
                        Ok(DecisionResult::ConflictsFound(a, vec![c]))
                    }
                    _ => return Err(err),
                },
            }
        } else {
            event!(Level::TRACE, "SAT problem returned UNSAT");

            Ok(DecisionResult::Unsat)
        }
    }

    fn gadgets_for_assignment(&self, a: &SlotAssignments) -> Vec<ModeledBlock<'ctx>> {
        let mut gadgets = Vec::with_capacity(a.choices().len());
        for (index, &choice) in a.choices().iter().enumerate() {
            gadgets.push(self.gadget_candidates[index][choice].clone());
        }
        gadgets
    }

    #[instrument(skip_all)]
    pub fn decide(&mut self) -> Result<DecisionResult, CrackersError> {
        loop {
            let res = self.single_decision_iteration()?;
            match res {
                DecisionResult::ConflictsFound(a, c) => {
                    event!(
                        Level::INFO,
                        "{} has conflicts",
                        a.display_conflict(c.as_slice())
                    );
                    continue;
                }
                DecisionResult::AssignmentFound(a) => {
                    event!(Level::INFO, "{:?} is feasible", a.get_assignments());
                    return Ok(DecisionResult::AssignmentFound(a));
                }
                DecisionResult::Unsat => {
                    event!(Level::WARN, "No assignment exists");
                    return Ok(DecisionResult::Unsat);
                }
            }
        }
    }
}
