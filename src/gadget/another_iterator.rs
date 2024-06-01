use jingle::modeling::{ModeledBlock, ModelingContext};
use z3::{Context, SatResult, Solver};

use crate::gadget::Gadget;
use crate::gadget::signature::OutputSignature;

pub struct TraceCandidateIterator<'a, 'ctx> {
    z3: &'ctx Context,
    solver: Solver<'ctx>,
    gadgets: Box<dyn Iterator<Item = &'a Gadget>>,
    trace: Vec<ModeledBlock<'ctx>>,
}

impl<'a, 'ctx> Iterator for TraceCandidateIterator<'a, 'ctx> {
    type Item = Vec<Option<&'a Gadget>>;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            let gadget = self.gadgets.next()?;
            let gadget_signature = OutputSignature::from(gadget);
            let is_candidate: Vec<bool> = self
                .trace
                .iter()
                .map(|i| OutputSignature::from(i).covers(&gadget_signature))
                .collect();
            if is_candidate.iter().any(|b| *b) {
                let model = gadget.model(self.z3);
                if let Ok(model) = model{
                    let result = is_candidate.iter().enumerate().map(|(i,c)| match c{
                        false => None,
                        true => {
                            let t = &self.trace[i];
                            let check = model.upholds_postcondition(t).ok()?;
                            match self.solver.check_assumptions(&[check]){
                                SatResult::Sat => Some(gadget),
                                _ => None
                            }
                        }
                    });
                    return Some(result.collect())
                }else{
                    continue;
                }
            }else{
                continue;
            }
        }
    }
}
