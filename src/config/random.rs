use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct RandomConfig{
    random_sample_size: usize,
    random_seed: i64
}