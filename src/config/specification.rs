use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct SpecificationConfig {
    pub max_instructions: usize,
}

impl SpecificationConfig {

}
