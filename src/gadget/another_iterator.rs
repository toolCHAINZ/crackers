use jingle::modeling::{ModeledInstruction, ModelingContext};
use z3::{Context, SatResult, Solver};

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
    check_model: bool,
}

impl<'ctx, 'a, T> TraceCandidateIterator<'ctx, 'a, T>
where
    T: Iterator<Item = &'a Gadget>,
{
    pub(crate) fn new(
        z3: &'ctx Context,
        gadgets: T,
        trace: Vec<ModeledInstruction<'ctx>>,
        check_model: bool,
    ) -> Self {
        let _solver = Solver::new(z3);
        Self {
            z3,
            _solver,
            gadgets,
            trace,
            check_model,
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
                        && !i.instr.has_syscall()
                        || gadget.instructions.iter().any(|gi| gi.ops_equal(&i.instr))
                })
                .collect();
            if is_candidate.iter().any(|b| *b) {
                let model = gadget.model(self.z3);
                if model.is_ok() {
                    if self.check_model {
                        is_candidate.iter().enumerate().for_each(|(i, c)| {
                            if *c {
                                let expr = model.as_ref().unwrap().reaches(&self.trace[i]).unwrap();
                                let expr2 = model
                                    .as_ref()
                                    .unwrap()
                                    .upholds_postcondition(&self.trace[i])
                                    .unwrap();
                                match self._solver.check_assumptions(&[expr, expr2]) {
                                    SatResult::Sat => next_entry[i].push(gadget),
                                    _ => {}
                                }
                            }
                        })
                    } else {
                        is_candidate.iter().enumerate().for_each(|(i, c)| {
                            if *c {
                                next_entry[i].push(gadget)
                            }
                        })
                    }
                }
                if next_entry.iter().all(|b| b.len() > 0) {
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
