use std::collections::HashSet;
use jingle::modeling::{BranchConstraint, ModeledBlock, ModelingContext, State};
use jingle::modeling::BlockEndBehavior::Fallthrough;
use jingle::sleigh::{PcodeOperation, SpaceInfo, SpaceManager};
use jingle::varnode::ResolvedVarnode;
use z3::ast::Ast;
use z3::{Context, SatResult, Solver};
use crate::synthesis::greedy::GreedySynthesizerError;

#[derive(Debug)]
pub struct TraceModel<'ctx> {
    z3: &'ctx Context,
    pub solver: Solver<'ctx>,
    blocks: Vec<ModeledBlock<'ctx>>,
}

impl<'ctx> TraceModel<'ctx> {
    pub fn new(z3: &'ctx Context) -> Self {
        Self {
            z3,
            solver: Solver::new_for_logic(z3, "QF_AUFBV").unwrap(),
            blocks: vec![],
        }
    }
    pub fn push_for<'a: 'ctx, T: ModelingContext<'ctx>>(
        &mut self,
        block: &ModeledBlock<'ctx>,
        spec: &'a T,
    ) -> Result<(), GreedySynthesizerError> {
        self.solver.push();
        let block = block.clone();
        if !block
            .get_branch_constraint()
            .is_plausible_match(spec.get_branch_constraint())
        {
            return Err(
                GreedySynthesizerError::BlockChoiceError
            );
        } else if !matches!(spec.get_branch_constraint().last, Fallthrough(_)) {
            // TODO: we need to come up with constraints to make things like syscalls "unreachable"
            // by any other means
            let block_branch = block.get_branch_constraint().build_bv(spec)?;
            let spec_branch = spec.get_branch_constraint().build_bv(spec)?;
            self.solver.assert(&block_branch._eq(&spec_branch))
        }
        if let Some(last_block) = self.blocks.last() {
            self.solver.assert(&last_block.assert_concat(&block)?);
            self.solver
                .assert(&last_block.can_branch_to_address(block.get_address())?);
        }

        if matches!(
            self.solver.check_assumptions(&[block.refines(spec)?]),
            SatResult::Unsat
        ) {
            self.blocks.push(block);
            return Err(
                GreedySynthesizerError::BlockChoiceError,
            );
        }
        self.blocks.push(block);
        Ok(())
    }
    pub fn pop(&mut self) {
        self.solver.pop(1);
        self.blocks.pop();
    }
}

impl<'ctx> SpaceManager for TraceModel<'ctx> {
    fn get_space_info(&self, idx: usize) -> Option<&SpaceInfo> {
        self.blocks.first().and_then(|o| o.get_space_info(idx))
    }

    fn get_all_space_info(&self) -> &[SpaceInfo] {
        self.blocks
            .first()
            .map(|o| o.get_all_space_info())
            .unwrap_or(&[])
    }

    fn get_code_space_idx(&self) -> usize {
        self.blocks[0].get_code_space_idx()
    }
}

impl<'ctx> ModelingContext<'ctx> for TraceModel<'ctx> {
    fn get_z3(&self) -> &'ctx Context {
        self.z3
    }

    fn get_original_state(&self) -> &State<'ctx> {
        self.blocks.first().map(|b| b.get_original_state()).unwrap()
    }

    fn get_final_state(&self) -> &State<'ctx> {
        self.blocks.last().map(|b| b.get_final_state()).unwrap()
    }

    fn get_ops(&self) -> Vec<&PcodeOperation> {
        let mut ops = Vec::new();
        for block_ops in self.blocks.iter().map(|i| i.get_ops()) {
            for op in block_ops {
                ops.push(op)
            }
        }
        ops
    }

    fn get_inputs(&self) -> HashSet<ResolvedVarnode<'ctx>> {
        let mut varnode_set = HashSet::new();
        for x in &self.blocks {
            for x in x.get_inputs() {
                varnode_set.insert(x.clone());
            }
        }
        varnode_set
    }

    fn get_outputs(&self) -> HashSet<ResolvedVarnode<'ctx>> {
        let mut varnode_set = HashSet::new();
        for x in &self.blocks {
            for x in x.get_outputs() {
                varnode_set.insert(x.clone());
            }
        }
        varnode_set
    }

    fn get_branch_constraint(&self) -> &BranchConstraint {
        todo!()
    }
}
