use serde::Deserialize;

#[derive(Clone, Debug, Deserialize)]
pub struct LibraryConfig {
    pub path: String,
    pub max_gadget_length: usize,
    pub sample_size: Option<usize>,
}
