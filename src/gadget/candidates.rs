use rand::{random, SeedableRng};
use rand::prelude::StdRng;
use rand::seq::SliceRandom;
use tracing::{event, Level};

use crate::gadget::Gadget;

#[derive(Debug, Default)]
pub struct CandidateBuilder {
    random_sample_size: Option<usize>,
    random_sample_seed: Option<usize>,
}

impl CandidateBuilder {
    pub fn with_random_sample_seed(mut self, seed: usize) -> Self {
        self.random_sample_seed = Some(seed);
        self
    }

    pub fn with_random_sample_size(mut self, size: usize) -> Self {
        self.random_sample_size = Some(size);
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

pub struct Candidates {
    candidates: Vec<Vec<Gadget>>,
}
