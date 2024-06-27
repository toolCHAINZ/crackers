use std::fs;

use jingle::JingleError;
use jingle::sleigh::context::{Image, SleighContext};
use serde::Deserialize;

use crate::config::error::CrackersConfigError;
use crate::config::sleigh::SleighConfig;
use crate::error::CrackersError;

#[derive(Debug, Deserialize)]
pub struct SpecificationConfig {
    pub path: String,
    pub max_instructions: usize,
}

impl SpecificationConfig {
    fn load_image(&self) -> Result<Image, CrackersConfigError> {
        Ok(Image::from(fs::read(&self.path)?))
    }
    pub fn load_spec(
        &self,
        sleigh_config: SleighConfig,
        arch: &str,
    ) -> Result<SleighContext, CrackersError> {
        let builder = sleigh_config.context_builder()?;
        let image = self.load_image()?;
        let ctx = builder
            .set_image(image)
            .build(arch)
            .map_err(|e| JingleError::Sleigh(e))?;
        Ok(ctx)
    }
}
