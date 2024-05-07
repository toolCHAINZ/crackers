use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct LibraryConfig {
    pub(crate) path: String,
    pub(crate) max_gadget_length: usize,
    pub(crate) random_sample_size: Option<usize>,
    pub(crate) random_sample_seed: Option<u64>,
}
