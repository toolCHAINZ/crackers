use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct SleighConfig {
    pub(crate) ghidra_path: String,
    architecture: Option<String>
}

