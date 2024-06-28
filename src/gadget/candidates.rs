use jingle::modeling::ModeledBlock;
use rand::prelude::StdRng;
use rand::seq::SliceRandom;
use rand::SeedableRng;
use tracing::{event, Level};
use z3::Context;

use crate::error::CrackersError;
use crate::error::CrackersError::UnsimulatedOperation;
use crate::gadget::Gadget;

#[derive(Clone, Debug, Default)]
pub struct CandidateBuilder {
    random_sample_size: usize,
    random_sample_seed: i64,
}

impl CandidateBuilder {
    pub fn with_random_sample_seed(mut self, seed: i64) -> Self {
        self.random_sample_seed = seed;
        self
    }

    pub fn with_random_sample_size(mut self, size: usize) -> Self {
        self.random_sample_size = size;
        self
    }

    pub fn build<T: Iterator<Item = Vec<Option<Gadget>>>>(
        &self,
        iter: T,
    ) -> Result<Candidates, CrackersError> {
        let mut candidates = vec![];
        for x in iter {
            if candidates.is_empty() {
                for _ in &x {
                    candidates.push(vec![])
                }
            }
            for (i, opt_gadget) in x.into_iter().enumerate() {
                if let Some(g) = opt_gadget {
                    candidates[i].push(g)
                }
            }
        }
        let seed = self.random_sample_seed as u64;
        let mut rng = StdRng::seed_from_u64(seed);
        event!(Level::INFO, "Using seed: {}", seed);
        candidates = candidates
            .into_iter()
            .map(|c| {
                c.choose_multiple(&mut rng, self.random_sample_size)
                    .cloned()
                    .collect()
            })
            .collect();

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
