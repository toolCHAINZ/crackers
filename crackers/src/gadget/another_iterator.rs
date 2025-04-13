use jingle::JingleContext;
use jingle::modeling::{ModeledInstruction, ModelingContext};
use jingle::sleigh::{Instruction, OpCode};
use tracing::trace;
use z3::Solver;
use z3::ast::Ast;

use crate::gadget::Gadget;
use crate::gadget::signature::GadgetSignature;

pub struct TraceCandidateIterator<'ctx, 'a, T>
where
    T: Iterator<Item = &'a Gadget>,
{
    jingle: JingleContext<'ctx>,
    _solver: Solver<'ctx>,
    gadgets: T,
    trace: Vec<ModeledInstruction<'ctx>>,
}

impl<'ctx, 'a, T> TraceCandidateIterator<'ctx, 'a, T>
where
    T: Iterator<Item = &'a Gadget>,
{
    pub(crate) fn new(
        jingle: &JingleContext<'ctx>,
        gadgets: T,
        trace: Vec<ModeledInstruction<'ctx>>,
    ) -> Self {
        let _solver = Solver::new(jingle.z3);
        Self {
            jingle: jingle.clone(),
            _solver,
            gadgets,
            trace,
        }
    }
}
impl<'a, T> Iterator for TraceCandidateIterator<'_, 'a, T>
where
    T: Iterator<Item = &'a Gadget>,
{
    type Item = Vec<Option<&'a Gadget>>;

    fn next(&mut self) -> Option<Self::Item> {
        let mut next_entry = vec![None; self.trace.len()];
        loop {
            let gadget = self.gadgets.next()?;
            let gadget_signature = GadgetSignature::from(gadget);
            trace!("Evaluating gadget at {:x}", gadget.address());
            let is_candidate: Vec<bool> = self
                .trace
                .iter()
                .map(|i| {
                    trace!(
                        "Checking {} signature vs gadget {}",
                        i.instr.disassembly, gadget
                    );

                    gadget_signature.covers(&GadgetSignature::from_instr(&i.instr, i))
                        && has_compatible_control_flow(&i.instr, gadget)
                })
                .collect();
            if is_candidate.iter().any(|b| *b) {
                let model = gadget.model(&self.jingle);
                if let Ok(model) = &model {
                    is_candidate.iter().enumerate().for_each(|(i, c)| {
                        if *c {
                            let expr = model
                                .upholds_postcondition(&self.trace[i])
                                .unwrap()
                                .simplify();
                            if !expr.is_const() || expr.as_bool().unwrap() {
                                next_entry[i] = Some(gadget)
                            }
                        }
                    })
                } else {
                    trace!("Could not model gadget: \n{}", gadget)
                }
                return Some(next_entry);
            } else {
                continue;
            }
        }
    }
}

fn has_compatible_control_flow(i: &Instruction, gadget: &Gadget) -> bool {
    if i.has_syscall() {
        gadget.instructions.iter().any(|gi| gi.ops_equal(i))
    } else {
        gadget.ops().any(|o| is_controllable_jump(o.opcode()))
    }
}

fn is_controllable_jump(op: OpCode) -> bool {
    matches!(
        op,
        OpCode::CPUI_BRANCHIND | OpCode::CPUI_CALLIND | OpCode::CPUI_RETURN
    )
}
