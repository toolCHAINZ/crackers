use jingle::modeling::{ModeledBlock, ModelingContext};
use z3::{Context, SatResult, Solver};

use crate::gadget::Gadget;
use crate::gadget::signature::OutputSignature;

pub struct TraceCandidateIterator<'ctx, T>
where
    T: Iterator<Item = Gadget>,
{
    z3: &'ctx Context,
    solver: Solver<'ctx>,
    gadgets: T,
    trace: Vec<ModeledBlock<'ctx>>,
}

impl<'ctx, T> TraceCandidateIterator<'ctx, T>
where
    T: Iterator<Item = Gadget>,
{
    pub(crate) fn new(z3: &'ctx Context, gadgets: T, trace: Vec<ModeledBlock<'ctx>>) -> Self {
        let solver = Solver::new(z3);
        Self {
            z3,
            solver,
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
                .map(|i| OutputSignature::from(i).covers(&gadget_signature))
                .collect();
            if is_candidate.iter().any(|b| *b) {
                let model = gadget.model(self.z3);
                if let Ok(model) = model {
                    let result = is_candidate.iter().enumerate().map(|(i, c)| match c {
                        false => None,
                        true => {
                            let t = &self.trace[i];
                            let check = model.upholds_postcondition(t).ok()?;
                            match self.solver.check_assumptions(&[check]) {
                                SatResult::Sat => Some(gadget.clone()),
                                _ => None,
                            }
                        }
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
