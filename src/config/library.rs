use std::fs;

use jingle::JingleError;
use jingle::sleigh::context::{Image, map_gimli_architecture, SleighContext};
use object::{File, Object};
use serde::Deserialize;

use crate::config::error::CrackersConfigError;
use crate::config::error::CrackersConfigError::UnrecognizedArchitecture;
use crate::config::random::RandomConfig;
use crate::config::sleigh::SleighConfig;
use crate::error::CrackersError;

#[derive(Debug, Deserialize)]
pub struct LibraryConfig {
    pub path: String,
    pub max_gadget_length: usize,
    #[serde(flatten)]
    pub random: Option<RandomConfig>,
}

impl LibraryConfig {
    fn load_image(&self) -> Result<(Image, &'static str), CrackersConfigError> {
        let data = fs::read(&self.path)?;
        let file = File::parse(&*data)?;
        let arch = map_gimli_architecture(&file).ok_or(UnrecognizedArchitecture(format!(
            "{:?}",
            file.architecture()
        )))?;
        let img = Image::try_from(file)?;

        Ok((img, arch))
    }

    pub fn load(&self, sleigh_config: SleighConfig) -> Result<SleighContext, CrackersError> {
        let (img, arch) = self.load_image()?;
        let builder = sleigh_config.context_builder()?;
        let ctx = builder
            .set_image(img)
            .build(arch)
            .map_err(|e| JingleError::Sleigh(e))?;
        Ok(ctx)
    }
}
