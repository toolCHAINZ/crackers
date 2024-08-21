use jingle::modeling::{ModeledInstruction, ModelingContext};
use jingle::sleigh::{Instruction, OpCode};
use z3::ast::Ast;
use z3::{Context, Solver};

use crate::gadget::signature::OutputSignature;
use crate::gadget::Gadget;

pub struct TraceCandidateIterator<'ctx, 'a, T>
where
    T: Iterator<Item = &'a Gadget>,
{
    z3: &'ctx Context,
    _solver: Solver<'ctx>,
    gadgets: T,
    trace: Vec<ModeledInstruction<'ctx>>,
}

impl<'ctx, 'a, T> TraceCandidateIterator<'ctx, 'a, T>
where
    T: Iterator<Item = &'a Gadget>,
{
    pub(crate) fn new(z3: &'ctx Context, gadgets: T, trace: Vec<ModeledInstruction<'ctx>>) -> Self {
        let _solver = Solver::new(z3);
        Self {
            z3,
            _solver,
            gadgets,
            trace,
        }
    }
}
impl<'ctx, 'a, T> Iterator for TraceCandidateIterator<'ctx, 'a, T>
where
    T: Iterator<Item = &'a Gadget>,
{
    type Item = Vec<&'a Gadget>;

    fn next(&mut self) -> Option<Self::Item> {
        let mut next_entry = vec![vec![]; self.trace.len()];
        loop {
            let gadget = self.gadgets.next()?;
            let gadget_signature = OutputSignature::from(gadget);
            let is_candidate: Vec<bool> = self
                .trace
                .iter()
                .map(|i| {
                    gadget_signature.covers(&OutputSignature::from(&i.instr))
                        && has_compatible_control_flow(&i.instr, gadget)
                })
                .collect();
            if is_candidate.iter().any(|b| *b) {
                let model = gadget.model(self.z3);
                if let Ok(model) = &model {
                    is_candidate.iter().enumerate().for_each(|(i, c)| {
                        if *c {
                            let expr = model
                                .upholds_postcondition(&self.trace[i])
                                .unwrap()
                                .simplify();
                            if !expr.is_const() || expr.as_bool().unwrap() {
                                next_entry[i].push(gadget)
                            }
                        }
                    })
                }
                if next_entry.iter().all(|b| !b.is_empty()) {
                    let new: Vec<&Gadget> =
                        next_entry.iter_mut().map(|b| b.pop().unwrap()).collect();
                    return Some(new);
                } else {
                    continue;
                }
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
