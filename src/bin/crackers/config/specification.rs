use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct SpecificationConfig {
    pub(crate) path: String,
    pub(crate) max_instructions: usize,
}
