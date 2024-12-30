use std::sync::Arc;

use jingle::modeling::{ModeledInstruction, ModelingContext};
use jingle::sleigh::{Instruction, OpCode};
use jingle::JingleContext;
use tracing::trace;
use z3::ast::{Ast, Bool};
use z3::Solver;

use crate::gadget::signature::GadgetSignature;
use crate::gadget::Gadget;
use crate::synthesis::builder::StateConstraintGenerator;

pub struct TraceCandidateIterator<'ctx, 'a, T>
where
    T: Iterator<Item = &'a Gadget>,
{
    jingle: JingleContext<'ctx>,
    _solver: Solver<'ctx>,
    gadgets: T,
    step_length: usize,
    postconditions: Vec<Arc<StateConstraintGenerator>>,
}

impl<'ctx, 'a, T> TraceCandidateIterator<'ctx, 'a, T>
where
    T: Iterator<Item = &'a Gadget>,
{
    pub(crate) fn new(
        jingle: &JingleContext<'ctx>,
        gadgets: T,
        step_length: usize,
        postconditions: Vec<Arc<StateConstraintGenerator>>,
    ) -> Self {
        let _solver = Solver::new(jingle.z3);
        Self {
            jingle: jingle.clone(),
            _solver,
            gadgets,
            step_length,
            postconditions,
        }
    }
}
impl<'a, T> Iterator for TraceCandidateIterator<'_, 'a, T>
where
    T: Iterator<Item = &'a Gadget>,
{
    type Item = Vec<Option<&'a Gadget>>;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            let gadget = self.gadgets.next()?;
            trace!("Evaluating gadget at {:x}", gadget.address());
            let model = gadget.model(&self.jingle);
            if let Ok(model) = &model {
                for predicate in &self.postconditions {
                    let f = predicate(&self.jingle, model.get_final_state(), 0);
                    let i = predicate(&self.jingle, model.get_original_state(), 0);
                    if let Ok(f) = f{
                        if let Ok(i) = i {
                            let f = f.simplify();
                            //
                            if f.is_const() && f.as_bool().unwrap() || !f.is_const(){
                                let i = i.simplify();
                                if !f._eq(&i).simplify().is_const(){
                                    return Some(vec![Some(gadget); self.step_length]);
                                }
                            }
                        }
                    }
                }
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
