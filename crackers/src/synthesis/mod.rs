use std::cmp::Ordering;
use std::fs;
use std::io::Write;

use jingle::JingleError;
use jingle::modeling::{ModeledBlock, ModeledInstruction};
use jingle::sleigh::Instruction;
use tracing::{event, instrument, Level};
use z3::Context;

use crate::error::CrackersError;
use crate::error::CrackersError::ModelGenerationError;
use crate::gadget::GadgetLibrary;
use crate::synthesis::assignment_model::AssignmentModel;
use crate::synthesis::pcode_theory::{ConflictClause, PcodeTheory};
use crate::synthesis::selection_strategy::optimization_problem::OptimizationProblem;
use crate::synthesis::selection_strategy::SelectionStrategy;
use crate::synthesis::slot_assignments::SlotAssignments;

pub mod assignment_model;
mod pcode_theory;
pub mod selection_strategy;
pub mod slot_assignments;
mod builder;

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

#[derive(Debug)]
pub struct AssignmentSynthesis<'ctx, T: SelectionStrategy<'ctx>> {
    gadget_candidates: Vec<Vec<ModeledBlock<'ctx>>>,
    sat_problem: T,
    theory_problem: PcodeTheory<'ctx>,
}

impl<'ctx, T: SelectionStrategy<'ctx>> AssignmentSynthesis<'ctx, T> {
    #[instrument(skip_all)]
    pub fn new(
        z3: &'ctx Context,
        templates: Vec<Instruction>,
        library: GadgetLibrary,
    ) -> Result<Self, JingleError> {
        let mut modeled_templates = vec![];
        let mut gadget_candidates: Vec<Vec<ModeledBlock<'ctx>>> = vec![];
        for template in templates.iter() {
            modeled_templates
                .push(ModeledInstruction::new(template.clone(), &library, z3).unwrap());
            let candidates: Vec<ModeledBlock<'ctx>> = library
                .get_modeled_gadgets_for_instruction(z3, &template)
                // todo: just here to make testing faster. Remove this later
                .take(200)
                .collect();
            event!(
                Level::DEBUG,
                "Instruction {} has {} candidates",
                template.disassembly,
                candidates.len()
            );
            gadget_candidates.push(candidates);
        }
        let sat_problem = T::initialize(z3, &gadget_candidates);
        let theory_problem =
            PcodeTheory::new(z3, modeled_templates.as_slice(), &gadget_candidates)?;
        Ok(AssignmentSynthesis {
            gadget_candidates,
            sat_problem,
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
