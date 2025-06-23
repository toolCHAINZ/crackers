//! Some utility functions for loading a [object::File] from a path

use std::fs;
use std::path::Path;

use jingle::sleigh::context::image::gimli::{OwnedFile, map_gimli_architecture};
use jingle::sleigh::context::loaded::LoadedSleighContext;
use object::{File, Object};

use crate::config::error::CrackersConfigError;
use crate::config::error::CrackersConfigError::UnrecognizedArchitecture;
use crate::config::sleigh::SleighConfig;
use crate::gadget::library::image::SegmentFile;

fn load_image<T: AsRef<Path>>(path: T) -> Result<(SegmentFile, &'static str), CrackersConfigError> {
    let data = fs::read(path.as_ref())?;
    let file = File::parse(&*data)?;
    let arch = map_gimli_architecture(&file).ok_or(UnrecognizedArchitecture(format!(
        "{:?}",
        file.architecture()
    )))?;
    let img = SegmentFile::new(&file).map_err(|_| CrackersConfigError::LibraryParse)?;
    Ok((img, arch))
}

/// gross hack
fn load_image_spec<T: AsRef<Path>>(
    path: T,
) -> Result<(OwnedFile, &'static str), CrackersConfigError> {
    let data = fs::read(path.as_ref())?;
    let file = File::parse(&*data)?;
    let arch = map_gimli_architecture(&file).ok_or(UnrecognizedArchitecture(format!(
        "{:?}",
        file.architecture()
    )))?;
    let img = OwnedFile::new(&file).map_err(|_| CrackersConfigError::LibraryParse)?;
    Ok((img, arch))
}

pub fn load_sleigh<T: AsRef<Path>>(
    file_path: T,
    sleigh_config: &SleighConfig,
) -> Result<LoadedSleighContext<'_>, CrackersConfigError> {
    let (img, arch) = load_image(file_path)?;
    let builder = sleigh_config.context_builder()?;
    let ctx = builder.build(arch)?;
    let ctx = ctx.initialize_with_image(img)?;
    Ok(ctx)
}

pub fn load_sleigh_spec<T: AsRef<Path>>(
    file_path: T,
    sleigh_config: &SleighConfig,
) -> Result<LoadedSleighContext<'_>, CrackersConfigError> {
    let (img, arch) = load_image_spec(file_path)?;
    let builder = sleigh_config.context_builder()?;
    let ctx = builder.build(arch)?;
    let ctx = ctx.initialize_with_image(img)?;
    Ok(ctx)
}
