use std::fs;

use jingle::sleigh::context::{Image, map_gimli_architecture, SleighContextBuilder};
use jingle::sleigh::RegisterManager;
use object::File;
use serde::Deserialize;
use tracing::{event, Level};
use z3::Context;

use crackers::gadget::library::builder::GadgetLibraryBuilder;
use crackers::synthesis::AssignmentSynthesis;
use crackers::synthesis::builder::SynthesisBuilder;

use crate::config::constraint::{Constraint, gen_memory_constraint, gen_pointer_range_invariant, gen_register_constraint, gen_register_pointer_constraint};
use crate::config::library::LibraryConfig;
use crate::config::sleigh::SleighConfig;
use crate::config::specification::SpecificationConfig;
use crate::config::synthesis::SynthesisConfig;
use crate::error::CrackersBinError;
use crate::error::CrackersBinError::ConfigLoad;

mod library;
mod specification;
mod sleigh;
mod constraint;
mod synthesis;

#[derive(Debug, Deserialize)]
pub struct CrackersConfig {
    specification: SpecificationConfig,
    library: LibraryConfig,
    sleigh: SleighConfig,
    constraint: Option<Constraint>,
    synthesis: Option<SynthesisConfig>
}

impl CrackersConfig {
    fn get_sleigh_builder(&self) -> Result<SleighContextBuilder, CrackersBinError> {
        let builder = SleighContextBuilder::load_ghidra_installation(&self.sleigh.ghidra_path).map_err(|_| ConfigLoad)?;
        Ok(builder)
    }

    fn load_library_image(&self) -> Result<Vec<u8>, CrackersBinError> {
        let data = fs::read(&self.library.path).map_err(|_| ConfigLoad)?;
        Ok(data)
    }

    fn load_spec(&self) -> Result<Image, CrackersBinError> {
        let data = fs::read(&self.specification.path).map_err(|_| ConfigLoad)?;
        Ok(Image::from(data))
    }
    pub fn resolve<'a>(&self, z3: &'a Context) -> Result<AssignmentSynthesis<'a>, CrackersBinError> {
        let spec_sleigh_builder = self.get_sleigh_builder().unwrap();
        let library_sleigh_builder = self.get_sleigh_builder().unwrap();

        let data = self.load_library_image().unwrap();
        let library_image = File::parse(&*data).map_err(|_| ConfigLoad).unwrap();
        let spec_image = self.load_spec().unwrap();

        let architecture_str = map_gimli_architecture(&library_image).ok_or(ConfigLoad).unwrap();
        event!(Level::INFO, "Using SLEIGH architecture {}", architecture_str);
        let library_image = Image::try_from(library_image).map_err(|_| ConfigLoad).unwrap();
        let spec_sleigh = spec_sleigh_builder.set_image(spec_image).build(architecture_str).map_err(|_| ConfigLoad).unwrap();
        let library_sleigh = library_sleigh_builder.set_image(library_image).build(architecture_str).map_err(|_| ConfigLoad).unwrap();

        let gadget_library_params = GadgetLibraryBuilder::default()
            .max_gadget_length(self.library.max_gadget_length)
            .random_sample_seed(self.library.random_sample_seed)
            .random_sample_size(self.library.random_sample_size);
        let mut b = SynthesisBuilder::default();
        b = b.with_gadget_library_builder(gadget_library_params);
        b = b.specification(spec_sleigh.read(0, self.specification.max_instructions));
        if let Some(a) = &self.synthesis{
            b = b.with_selection_strategy(a.strategy);
            b = b.candidates_per_slot(a.max_candidates_per_slot);
        }
        if let Some(c) = &self.constraint{
            if let Some(pre) = &c.precondition{
                if let Some(mem) = &pre.memory{
                    b = b.with_precondition(gen_memory_constraint(mem.clone()));
                }
                if let Some(reg) = &pre.register{
                    for (name, value) in reg {
                        if let Some(vn) = library_sleigh.get_register(&name){
                            b = b.with_precondition(gen_register_constraint(vn, *value));
                        }else{
                            event!(Level::WARN, "Unrecognized register name: {}", name);
                        }
                    }
                }
                if let Some(pointer) = &pre.pointer{
                    for (name, value) in pointer{
                        if let Some(vn) = library_sleigh.get_register(&name){
                            b = b.with_precondition(gen_register_pointer_constraint(vn, value.clone(), c.pointer.clone()))
                        }
                    }
                }
            }
            // todo: gross to repeat this stuff
            if let Some(post) = &c.postcondition{
                if let Some(mem) = &post.memory{
                    b = b.with_postcondition(gen_memory_constraint(mem.clone()));
                }
                if let Some(reg) = &post.register{
                    for (name, value) in reg {
                        if let Some(vn) = library_sleigh.get_register(&name){
                            b = b.with_postcondition(gen_register_constraint(vn, *value));
                        }else{
                            event!(Level::WARN, "Unrecognized register name: {}", name);
                        }
                    }
                }
                if let Some(pointer) = &post.pointer{
                    for (name, value) in pointer{
                        if let Some(vn) = library_sleigh.get_register(&name){
                            b = b.with_postcondition(gen_register_pointer_constraint(vn, value.clone(), c.pointer.clone()))
                        }
                    }
                }
            }
            if let Some(pointer) = &c.pointer{
                b = b.with_pointer_invariant(gen_pointer_range_invariant(pointer.clone()));
            }
        }
        let thing = b.build(&z3, &library_sleigh).map_err(|_| ConfigLoad).unwrap();
        Ok(thing)
    }
}
