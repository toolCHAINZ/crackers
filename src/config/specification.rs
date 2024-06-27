use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct SpecificationConfig {
    pub path: String,
    pub max_instructions: usize,
}
