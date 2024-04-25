use std::fs;

use jingle::sleigh::context::{Image, map_gimli_architecture, SleighContextBuilder};
use object::File;
use serde::Deserialize;
use z3::Context;

use crackers::gadget::library::builder::GadgetLibraryBuilder;
use crackers::synthesis::AssignmentSynthesis;
use crackers::synthesis::builder::SynthesisBuilder;

use crate::error::CrackersBinError;
use crate::error::CrackersBinError::ConfigLoad;

#[derive(Debug, Deserialize)]
pub struct LibraryConfig {
    binary_path: String,
    max_gadget_length: usize,
}

#[derive(Debug, Deserialize)]
pub struct CrackersConfig {
    specification_binary: String,
    library: LibraryConfig,
    ghidra_path: String,
}

impl CrackersConfig {
    fn get_sleigh_builder(&self) -> Result<SleighContextBuilder, CrackersBinError> {
        let builder = SleighContextBuilder::load_ghidra_installation(&self.ghidra_path).map_err(|_| ConfigLoad)?;
        Ok(builder)
    }

    fn load_library_image(&self) -> Result<Vec<u8>, CrackersBinError> {
        let data = fs::read(&self.library.binary_path).map_err(|_| ConfigLoad)?;
        Ok(data)
    }

    fn load_spec(&self) -> Result<Image, CrackersBinError> {
        let data = fs::read(&self.specification_binary).map_err(|_| ConfigLoad)?;
        Ok(Image::from(data))
    }
    pub fn resolve<'a>(&self, z3: &'a Context) -> Result<AssignmentSynthesis<'a>, CrackersBinError> {
        let spec_sleigh_builder = self.get_sleigh_builder().unwrap();
        let library_sleigh_builder = self.get_sleigh_builder().unwrap();

        let data = self.load_library_image().unwrap();
        let library_image = File::parse(&*data).map_err(|_| ConfigLoad).unwrap();
        let spec_image = self.load_spec().unwrap();

        let architecture_str = map_gimli_architecture(&library_image).ok_or(ConfigLoad).unwrap();
        let library_image = Image::try_from(library_image).map_err(|_| ConfigLoad).unwrap();
        let spec_sleigh = spec_sleigh_builder.set_image(spec_image).build(architecture_str).map_err(|_| ConfigLoad).unwrap();
        let library_sleigh = library_sleigh_builder.set_image(library_image).build(architecture_str).map_err(|_| ConfigLoad).unwrap();

        let gadget_library_params = GadgetLibraryBuilder::default()
            .max_gadget_length(&self.library.max_gadget_length);
        let mut b = SynthesisBuilder::default();
        b = b.with_gadget_library_builder(gadget_library_params);
        b = b.specification(spec_sleigh.read(0,5));
        let thing = b.build(&z3, &library_sleigh).map_err(|_| ConfigLoad).unwrap();
        Ok(thing)

    }
}
