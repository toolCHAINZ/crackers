use serde::Deserialize;

use crate::config::random::RandomConfig;

#[derive(Debug, Deserialize)]
pub struct LibraryConfig {
    pub path: String,
    pub max_gadget_length: usize,
    #[serde(flatten)]
    pub random: Option<RandomConfig>,
}

