use jingle::JingleError;
use jingle::modeling::ModeledBlock;
use rand::{random, SeedableRng};
use rand::prelude::StdRng;
use rand::seq::SliceRandom;
use tracing::{event, Level};
use z3::Context;

use crate::error::CrackersError;
use crate::gadget::Gadget;

#[derive(Clone, Debug, Default)]
pub struct CandidateBuilder {
    random_sample_size: Option<usize>,
    random_sample_seed: Option<i64>,
}

impl CandidateBuilder {
    pub fn with_random_sample_seed(mut self, seed: Option<i64>) -> Self {
        self.random_sample_seed = seed;
        self
    }

    pub fn with_random_sample_size(mut self, size: Option<usize>) -> Self {
        self.random_sample_size = size;
        self
    }

    pub fn build<T: Iterator<Item = Vec<Option<Gadget>>>>(&self, iter: T) -> Candidates {
        let mut candidates = vec![];
        for x in iter {
            if candidates.len() == 0 {
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
        if let Some(s) = self.random_sample_size {
            let seed = self.random_sample_seed.unwrap_or(random());
            let mut rng = StdRng::seed_from_u64(seed as u64);
            event!(Level::INFO, "Using seed: {}", seed);
             candidates = candidates
                .into_iter()
                .map(|c| c.choose_multiple(&mut rng, s).cloned().collect())
                .collect();
        }
        Candidates { candidates }
    }
}

#[derive(Clone)]
pub struct Candidates {
    pub candidates: Vec<Vec<Gadget>>,
}

impl Candidates{
    pub fn model<'ctx>(&self, z3: &'ctx Context) -> Result<Vec<Vec<ModeledBlock<'ctx>>>, CrackersError>{
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