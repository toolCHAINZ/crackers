use jingle::modeling::ModeledInstruction;
use z3::{Context, Solver};

use crate::gadget::signature::OutputSignature;
use crate::gadget::Gadget;

pub struct TraceCandidateIterator<'ctx, T>
where
    T: Iterator<Item = Gadget>,
{
    z3: &'ctx Context,
    _solver: Solver<'ctx>,
    gadgets: T,
    trace: Vec<ModeledInstruction<'ctx>>,
}

impl<'ctx, T> TraceCandidateIterator<'ctx, T>
where
    T: Iterator<Item = Gadget>,
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
impl<'ctx, T> Iterator for TraceCandidateIterator<'ctx, T>
where
    T: Iterator<Item = Gadget>,
{
    type Item = Vec<Option<Gadget>>;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            let gadget = self.gadgets.next()?;
            let gadget_signature = OutputSignature::from(&gadget);
            let is_candidate: Vec<bool> = self
                .trace
                .iter()
                .map(|i| {
                    gadget_signature.covers(&OutputSignature::from(&i.instr))
                        && !i.instr.has_syscall()
                        || gadget.instructions.iter().any(|gi| gi.ops_equal(&i.instr))
                })
                .collect();
            if is_candidate.iter().any(|b| *b) {
                let model = gadget.model(self.z3);
                if model.is_ok() {
                    let result = is_candidate.iter().map(|c| match c {
                        false => None,
                        true => Some(gadget.clone()),
                    });
                    return Some(result.collect());
                } else {
                    continue;
                }
            } else {
                continue;
            }
        }
    }
}
