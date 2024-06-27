use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct RandomConfig{
    pub random_sample_size: usize,
    pub random_seed: Option<i64>
}