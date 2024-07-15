use jingle::modeling::ModeledBlock;
use z3::Context;

use crate::error::CrackersError;
use crate::error::CrackersError::UnsimulatedOperation;
use crate::gadget::Gadget;

#[derive(Clone, Debug, Default)]
pub struct CandidateBuilder {
    random_sample_size: usize,
}

impl CandidateBuilder {
    pub fn with_random_sample_size(mut self, size: usize) -> Self {
        self.random_sample_size = size;
        self
    }

    pub fn build<'a, T: Iterator<Item = Vec<&'a Gadget>>>(
        &self,
        iter: T,
    ) -> Result<Candidates, CrackersError> {
        let candidates: Vec<Vec<&Gadget>> = iter
            .map(|v| v.iter().map(|vv| vec![*vv]).collect())
            .take(self.random_sample_size)
            .reduce(|mut acc: Vec<Vec<&Gadget>>, value| {
                for (i, vector) in acc.iter_mut().enumerate() {
                    vector.push(value[i][0]);
                }
                acc
            })
            .unwrap();
        let candidates: Vec<Vec<Gadget>> = candidates
            .into_iter()
            .map(|g| g.into_iter().cloned().collect()).collect();
        if let Some((index, _)) = candidates.iter().enumerate().find(|(_, f)| f.is_empty()) {
            Err(UnsimulatedOperation { index })
        } else {
            Ok(Candidates { candidates })
        }
    }
}

#[derive(Clone)]
pub struct Candidates {
    pub candidates: Vec<Vec<Gadget>>,
}

impl Candidates {
    pub fn model<'ctx>(
        &self,
        z3: &'ctx Context,
    ) -> Result<Vec<Vec<ModeledBlock<'ctx>>>, CrackersError> {
        let mut result = vec![];
        for x in &self.candidates {
            let mut v = vec![];
            for g in x {
                v.push(g.model(z3)?);
            }
            result.push(v)
        }
        Ok(result)
    }
}
