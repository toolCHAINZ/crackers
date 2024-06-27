use rand::random;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct MetaConfig {
    pub seed: i64,
}

impl Default for MetaConfig {
    fn default() -> Self {
        Self { seed: random() }
    }
}
