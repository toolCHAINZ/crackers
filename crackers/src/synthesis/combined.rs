use jingle::sleigh::Instruction;
use tracing::{Level, event};
use z3::Context;

use crate::error::CrackersError;
use crate::synthesis::builder::SynthesisParams;
use crate::synthesis::partition_iterator::Partition;
use crate::synthesis::{AssignmentSynthesis, DecisionResult};

pub struct CombinedAssignmentSynthesis<'a> {
    pub(crate) base_config: SynthesisParams,
    pub(crate) z3: &'a Context,
}

impl<'a> CombinedAssignmentSynthesis<'a> {
    pub fn decide(&mut self) -> Result<DecisionResult, CrackersError> {
        let mut ordering: Vec<Vec<Instruction>> = self
            .base_config
            .instructions
            .partitions()
            .map(|part| {
                part.into_iter()
                    .map(|instrs| Instruction::try_from(instrs).unwrap())
                    .collect::<Vec<Instruction>>()
            })
            .collect();
        // let mut blacklist = HashSet::new();
        // todo: gross hack to avoid rewriting the partitioning algorithm to be breadth-first
        ordering.sort_by(|a, b| a.len().partial_cmp(&b.len()).unwrap());
        let iter = ordering.into_iter();
        let mut last: Option<_> = None;
        for instructions in iter {
            // todo: filter for instruction combinations that have already been ruled out?
            // if instructions.iter().any(|i| blacklist.contains(i)) {
            //     continue;
            // }
            let mut new_config = self.base_config.clone();
            new_config.instructions = instructions;
            let synth = AssignmentSynthesis::new(self.z3, &new_config);
            if let Ok(mut synth) = synth {
                // this one constructed, let's try it
                match synth.decide() {
                    Ok(result) => {
                        match result {
                            DecisionResult::AssignmentFound(a) => {
                                return Ok(DecisionResult::AssignmentFound(a));
                            }
                            DecisionResult::Unsat(e) => {
                                // todo: add in bad combos here
                                event!(Level::WARN, "{:?}", e);
                                // e.indexes.iter().for_each(|i|{blacklist.insert(new_config.instructions[*i].clone());});
                                last = Some(DecisionResult::Unsat(e))
                            }
                        }
                    }
                    Err(e) => {
                        event!(Level::ERROR, "{:?}", e)
                    }
                }
            } else {
                event!(Level::WARN, "Failed to find gadgets for partition")
            }
        }
        // Only an empty specification can possibly result in this being `None`
        last.ok_or(CrackersError::EmptySpecification)
    }

    // gross but I don't feel like rewriting this right now
    pub fn decide_single_threaded(&mut self) -> Result<DecisionResult, CrackersError> {
        let mut ordering: Vec<Vec<Instruction>> = self
            .base_config
            .instructions
            .partitions()
            .map(|part| {
                part.into_iter()
                    .map(|instrs| Instruction::try_from(instrs).unwrap())
                    .collect::<Vec<Instruction>>()
            })
            .collect();
        // let mut blacklist = HashSet::new();
        // todo: gross hack to avoid rewriting the partitioning algorithm to be breadth-first
        ordering.sort_by(|a, b| a.len().partial_cmp(&b.len()).unwrap());
        let iter = ordering.into_iter();
        let mut last: Option<_> = None;
        for instructions in iter {
            // todo: filter for instruction combinations that have already been ruled out?
            // if instructions.iter().any(|i| blacklist.contains(i)) {
            //     continue;
            // }
            let mut new_config = self.base_config.clone();
            new_config.instructions = instructions;
            let synth = AssignmentSynthesis::new(self.z3, &new_config);
            if let Ok(mut synth) = synth {
                // this one constructed, let's try it
                match synth.decide_single_threaded() {
                    Ok(result) => {
                        match result {
                            DecisionResult::AssignmentFound(a) => {
                                return Ok(DecisionResult::AssignmentFound(a));
                            }
                            DecisionResult::Unsat(e) => {
                                // todo: add in bad combos here
                                event!(Level::WARN, "{:?}", e);
                                // e.indexes.iter().for_each(|i|{blacklist.insert(new_config.instructions[*i].clone());});
                                last = Some(DecisionResult::Unsat(e))
                            }
                        }
                    }
                    Err(e) => {
                        event!(Level::ERROR, "{:?}", e)
                    }
                }
            } else {
                event!(Level::WARN, "Failed to find gadgets for partition")
            }
        }
        // Only an empty specification can possibly result in this being `None`
        last.ok_or(CrackersError::EmptySpecification)
    }
    pub fn new(z3: &'a Context, base_config: SynthesisParams) -> Self {
        Self { z3, base_config }
    }
}
