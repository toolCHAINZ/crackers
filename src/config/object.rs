//! Some utility functions for loading a [object::File] from a path

use std::fs;
use std::path::Path;

use jingle::sleigh::context::{Image, map_gimli_architecture, SleighContext};
use object::{File, Object};

use crate::config::error::CrackersConfigError;
use crate::config::error::CrackersConfigError::UnrecognizedArchitecture;
use crate::config::sleigh::SleighConfig;

fn load_image<T: AsRef<Path>>(path: T) -> Result<(Image, &'static str), CrackersConfigError> {
    let data = fs::read(path.as_ref())?;
    let file = File::parse(&*data)?;
    let arch = map_gimli_architecture(&file).ok_or(UnrecognizedArchitecture(format!(
        "{:?}",
        file.architecture()
    )))?;
    let img = Image::try_from(file)?;

    Ok((img, arch))
}

pub fn load_sleigh<T: AsRef<Path>>(file_path: T, sleigh_config: &SleighConfig) -> Result<SleighContext, CrackersConfigError> {
    let (img, arch) = load_image(file_path)?;
    let builder = sleigh_config.context_builder()?;
    let ctx = builder
        .set_image(img)
        .build(arch)?;
    Ok(ctx)
}