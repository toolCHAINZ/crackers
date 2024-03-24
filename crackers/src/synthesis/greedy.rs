use jingle::modeling::{ModeledInstruction, ModelingContext};
use jingle::JingleError;
use thiserror::Error;
use z3::{Context, SatResult};

use crate::gadget::GadgetLibrary;
use crate::synthesis::greedy::GreedySynthesizerError::NoSolution;
use crate::trace::TraceModel;

#[derive(Debug, Error)]
pub enum GreedySynthesizerError {
    #[error("Z3 translation error")]
    Modeling(#[from] JingleError),
    #[error("z3 timed out when checking a branch target")]
    BranchTargetCheckTimeout,
    #[error("I don't remember what this one was. TBD")]
    BlockChoiceError,
    #[error("the outer problem returned UNSAT")]
    NoSolution,
}

pub struct GreedySynthesizer<'ctx> {
    z3: &'ctx Context,
    specification: Vec<ModeledInstruction<'ctx>>,
    candidate_blocks: GadgetLibrary,
}

impl<'ctx> GreedySynthesizer<'ctx> {
    pub fn new(
        z3: &'ctx Context,
        specification: Vec<ModeledInstruction<'ctx>>,
        library: GadgetLibrary,
    ) -> Self {
        Self {
            z3,
            specification,
            candidate_blocks: library,
        }
    }

    pub fn decide(&self) -> Result<TraceModel, GreedySynthesizerError> {
        let mut trace = TraceModel::new(self.z3);
        for instrs in self.specification.as_slice().windows(2) {
            let first = &instrs[0];
            let second = &instrs[1];
            trace.solver.assert(&first.assert_concat(second)?)
        }
        self.decide_internal(&mut trace, 0).map(|_| trace)
    }
    fn decide_internal<'a: 'ctx>(
        &'a self,
        trace: &mut TraceModel<'ctx>,
        depth: usize,
    ) -> Result<(), GreedySynthesizerError> {
        if depth == self.specification.len() {
            return Ok(());
        }
        let instr = &self.specification[depth];
        for _ in 0..depth {
            print!(" ");
        }
        println!(
            "{} {}",
            depth,
            instr.instr.display(instr.get_final_state()).unwrap()
        );

        for block in self
            .candidate_blocks
            .get_modeled_gadgets_for_instruction(self.z3, &instr.instr)
        {
            let can_substitute = trace
                .solver
                .check_assumptions(&[block.isolated()?.reaches(&instr.isolated()?)?]);
            if matches!(can_substitute, SatResult::Sat) {
                let block = block.isolated()?;
                if matches!(trace.push_for(&block, instr), Ok(()))
                    && matches!(trace.solver.check(), SatResult::Sat)
                {
                    for (i, x) in block.instructions.iter().enumerate() {
                        for _ in 0..depth {
                            print!(" ");
                        }
                        if i == 0 {
                            println!(" -> {}", x.disassembly);
                        } else {
                            println!("    {}", x.disassembly);
                        }
                    }
                    if let Ok(r) = self.decide_internal(trace, depth + 1) {
                        return Ok(r);
                    }
                }
                trace.pop();
            }
        }
        Err(NoSolution)
    }
}
