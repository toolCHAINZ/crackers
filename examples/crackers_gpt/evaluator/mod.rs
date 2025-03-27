use crackers::assert_concat;
use crackers::synthesis::assignment_model::AssignmentModel;
use crackers::synthesis::builder::SynthesisParams;
use crackers::synthesis::selection_strategy::SelectionFailure;
use crackers::synthesis::DecisionResult;
use jingle::modeling::{ModeledBlock, ModeledInstruction, ModelingContext};
use jingle::sleigh::{ArchInfoProvider, Instruction};
use jingle::JingleContext;
use z3::ast::{Ast, BV};
use z3::{Config, Context, SatResult, Solver};

use crate::config::CrackersGptConfig;

pub struct ExecveEvaluator {
    config: SynthesisParams,
    z3: Context,
}

#[derive(Debug)]
pub enum ExecveEvaluation<'ctx> {
    /// Something is wrong with the spec itself
    SpecError(Vec<String>),
    ImplementationError(SelectionFailure),
    Success(AssignmentModel<'ctx, ModeledBlock<'ctx>>),
}

pub fn summarize_implementation_error(spec: &[Instruction], e: SelectionFailure) -> Vec<String> {
    let mut res = vec![];
    for x in e.indexes {
        res.push(format!(
            "Ordering error: do not perform {} at offset {:x} in the trace",
            spec[x].disassembly, x
        ));
    }
    res
}

impl<'ctx> From<DecisionResult<'ctx, ModeledBlock<'ctx>>> for ExecveEvaluation<'ctx> {
    fn from(value: DecisionResult<'ctx, ModeledBlock<'ctx>>) -> Self {
        match value {
            DecisionResult::AssignmentFound(a) => ExecveEvaluation::Success(a),
            DecisionResult::Unsat(d) => ExecveEvaluation::ImplementationError(d),
        }
    }
}
impl ExecveEvaluator {
    pub fn new(config: CrackersGptConfig) -> anyhow::Result<Self> {
        let synth_params = config.resolve(vec![])?;
        Ok(Self {
            config: synth_params,
            z3: Context::new(&Config::new()),
        })
    }

    pub fn eval(&self, spec: &[Instruction]) -> anyhow::Result<ExecveEvaluation> {
        let spec_eval = self.eval_spec_model(spec)?;
        if let Some(a) = spec_eval {
            return Ok(ExecveEvaluation::SpecError(a));
        }
        // todo: gross hack to avoid dealing with some liftimes. I can live with one huge memcpy
        let mut c = self.config.clone();
        c.instructions = spec.to_vec();
        let mut problem = c.build_combined(&self.z3)?;
        let p: ExecveEvaluation = problem.decide()?.into();
        Ok(p)
    }

    pub fn eval_spec_model(&self, spec: &[Instruction]) -> anyhow::Result<Option<Vec<String>>> {
        let solver = Solver::new(&self.z3);
        let jingle = JingleContext::new(&self.z3, self.config.gadget_library.as_ref());
        let mut model = vec![];
        for x in spec {
            model.push(ModeledInstruction::new(x.clone(), &jingle)?);
        }
        solver.assert(&assert_concat(&self.z3, &model)?.simplify());
        // verify basic requirements of execve
        let rsi = model.as_slice().get_final_state().read(
            self.config
                .gadget_library
                .get_register("RSI")
                .unwrap()
                .into(),
        )?;
        rsi.simplify();
        let rax = model.as_slice().get_final_state().read(
            self.config
                .gadget_library
                .get_register("RAX")
                .unwrap()
                .into(),
        )?;
        rax.simplify();

        let rdx = model.as_slice().get_final_state().read(
            self.config
                .gadget_library
                .get_register("RDX")
                .unwrap()
                .into(),
        )?;
        rdx.simplify();

        let rsi_eq = rsi._eq(&BV::from_u64(&self.z3, 0, rsi.get_size()));
        let rax_eq = rax._eq(&BV::from_u64(&self.z3, 0x3b, rsi.get_size()));
        let rdx_eq = rdx._eq(&BV::from_u64(&self.z3, 0, rsi.get_size()));
        // todo: check string pointer
        // todo: check invariants
        match solver.check_assumptions(&[rsi_eq.clone(), rax_eq.clone(), rdx_eq.clone()]) {
            SatResult::Unsat => {
                let core = solver.get_unsat_core();
                let mut res = vec![];
                if core.contains(&rsi_eq) {
                    res.push("Assertion failed: 'rsi' == 0".to_string());
                }
                if core.contains(&rax_eq) {
                    res.push("Assertion failed: 'rax' == 0x3b".to_string());
                }
                if core.contains(&rdx_eq) {
                    res.push("Assertion failed: 'rdx' == 0".to_string());
                }
                Ok(Some(res))
            }
            SatResult::Unknown => {
                unreachable!()
            }
            SatResult::Sat => Ok(None),
        }
    }
}
