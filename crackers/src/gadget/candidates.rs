use crate::error::CrackersError;
use crate::error::CrackersError::UnsimulatedOperation;
use crate::gadget::Gadget;
use jingle::modeling::ModeledBlock;
use jingle::sleigh::SleighArchInfo;
use std::borrow::Borrow;

#[derive(Clone, Debug, Default)]
pub struct CandidateBuilder {
    random_sample_size: usize,
}

impl CandidateBuilder {
    pub fn with_random_sample_size(mut self, size: usize) -> Self {
        self.random_sample_size = size;
        self
    }

    pub fn build<'a, T: Iterator<Item = Vec<Option<&'a Gadget>>>>(
        &self,
        iter: T,
    ) -> Result<Candidates, CrackersError> {
        let mut candidates: Vec<Vec<Gadget>> = vec![];
        for gc in iter {
            // todo: this feels ugly but I just need something that works for now
            if gc.len() != candidates.len() {
                candidates = gc.iter().map(|_| vec![]).collect();
            }
            gc.iter()
                .enumerate()
                .filter_map(|(i, g)| g.map(|g| (i, g.clone())))
                .for_each(|(i, g)| {
                    if candidates[i].len() < self.random_sample_size {
                        candidates[i].push(g)
                    }
                });
            if !candidates.iter().any(|g| g.len() < self.random_sample_size) {
                break;
            }
        }
        // We never found ANY candidates for ANYTHING
        if candidates.is_empty() {
            Err(UnsimulatedOperation { index: 0 })
        }
        // We never found candidates for something
        else if let Some((index, _)) = candidates.iter().enumerate().find(|(_, f)| f.is_empty()) {
            Err(UnsimulatedOperation { index })
        } else {
            // candidates!
            Ok(Candidates { candidates })
        }
    }
}

#[derive(Clone)]
pub struct Candidates {
    pub candidates: Vec<Vec<Gadget>>,
}

impl Candidates {
    pub fn model<T: Borrow<SleighArchInfo>>(
        &self,
        info: T,
    ) -> Result<Vec<Vec<ModeledBlock>>, CrackersError> {
        let info = info.borrow();
        let mut result = vec![];
        for x in &self.candidates {
            let mut v = vec![];
            for g in x {
                v.push(g.model(info)?);
            }
            result.push(v)
        }
        Ok(result)
    }
}
