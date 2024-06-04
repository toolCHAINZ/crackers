use std::fs;
use std::sync::Arc;

use jingle::sleigh::context::{map_gimli_architecture, Image, SleighContextBuilder};
use jingle::sleigh::RegisterManager;
use object::File;
use serde::Deserialize;
use tracing::{event, Level};
use z3::Context;

use crackers::gadget::library::builder::GadgetLibraryBuilder;
use crackers::synthesis::builder::SynthesisBuilder;
use crackers::synthesis::AssignmentSynthesis;

use crate::config::constraint::{
    gen_memory_constraint, gen_pointer_range_transition_invariant, gen_register_constraint,
    gen_register_pointer_constraint, Constraint,
};
use crate::config::library::LibraryConfig;
use crate::config::sleigh::SleighConfig;
use crate::config::specification::SpecificationConfig;
use crate::config::synthesis::SynthesisConfig;
use crate::error::CrackersBinError;
use crate::error::CrackersBinError::ConfigLoad;

mod constraint;
mod library;
mod sleigh;
mod specification;
mod synthesis;

#[derive(Debug, Deserialize)]
pub struct CrackersConfig {
    specification: SpecificationConfig,
    library: LibraryConfig,
    sleigh: SleighConfig,
    constraint: Option<Constraint>,
    pub(crate) synthesis: Option<SynthesisConfig>,
}

impl CrackersConfig {
    fn get_sleigh_builder(&self) -> Result<SleighContextBuilder, CrackersBinError> {
        let builder = SleighContextBuilder::load_ghidra_installation(&self.sleigh.ghidra_path)
            .map_err(|_| ConfigLoad("Could not load sleigh".to_string()))?;
        Ok(builder)
    }

    fn load_library_image(&self) -> Result<Vec<u8>, CrackersBinError> {
        let data = fs::read(&self.library.path)
            .map_err(|_| ConfigLoad("Could not load image".to_string()))?;
        Ok(data)
    }

    fn load_spec(&self) -> Result<Image, CrackersBinError> {
        let data = fs::read(&self.specification.path)
            .map_err(|_| ConfigLoad("Could not load sleigh spec".to_string()))?;
        Ok(Image::from(data))
    }
    pub fn resolve<'z3>(
        &self,
        z3: &'z3 Context,
    ) -> Result<AssignmentSynthesis<'z3>, CrackersBinError> {
        let spec_sleigh_builder = self.get_sleigh_builder()?;
        let library_sleigh_builder = self.get_sleigh_builder()?;

        let data = self.load_library_image()?;
        let library_image = File::parse(&*data)
            .map_err(|_| ConfigLoad("Could not parse provided library binary".to_string()))?;
        let spec_image = self.load_spec()?;

        let architecture_str = map_gimli_architecture(&library_image).ok_or(ConfigLoad(
            "Could not identify library binary's architecture".to_string(),
        ))?;
        event!(
            Level::INFO,
            "Using SLEIGH architecture {}",
            architecture_str
        );
        let library_image = Image::try_from(library_image).map_err(|_| {
            ConfigLoad("Could not convert library image for usage in sleigh".to_string())
        })?;
        let spec_sleigh = spec_sleigh_builder
            .set_image(spec_image)
            .build(architecture_str)
            .map_err(|_| ConfigLoad("Could not build sleigh context for chain".to_string()))?;
        let library_sleigh = library_sleigh_builder
            .set_image(library_image)
            .build(architecture_str)
            .map_err(|_| ConfigLoad("Could not build sleigh context for library".to_string()))?;

        let gadget_library_params = GadgetLibraryBuilder::default()
            .max_gadget_length(self.library.max_gadget_length)
            .random_sample_seed(self.library.random_sample_seed)
            .random_sample_size(self.library.random_sample_size);
        let mut b = SynthesisBuilder::default();
        b = b.with_gadget_library_builder(gadget_library_params);
        b = b.specification(spec_sleigh.read(0, self.specification.max_instructions));
        if let Some(a) = &self.synthesis {
            b = b.with_selection_strategy(a.strategy);
            b = b.candidates_per_slot(a.max_candidates_per_slot);
            b = b.parallel(a.parallel);
        }
        if let Some(c) = &self.constraint {
            if let Some(pre) = &c.precondition {
                if let Some(mem) = &pre.memory {
                    b = b.with_precondition(Arc::new(gen_memory_constraint(mem.clone())));
                }
                if let Some(reg) = &pre.register {
                    for (name, value) in reg {
                        if let Some(vn) = library_sleigh.get_register(name) {
                            b = b.with_precondition(Arc::new(gen_register_constraint(
                                vn,
                                *value as u64,
                            )));
                        } else {
                            event!(Level::WARN, "Unrecognized register name: {}", name);
                        }
                    }
                }
                if let Some(pointer) = &pre.pointer {
                    for (name, value) in pointer {
                        if let Some(vn) = library_sleigh.get_register(name) {
                            b = b.with_precondition(Arc::new(gen_register_pointer_constraint(
                                vn,
                                value.clone(),
                                c.pointer,
                            )))
                        }
                    }
                }
            }
            // todo: gross to repeat this stuff
            if let Some(post) = &c.postcondition {
                if let Some(mem) = &post.memory {
                    b = b.with_postcondition(Arc::new(gen_memory_constraint(mem.clone())));
                }
                if let Some(reg) = &post.register {
                    for (name, value) in reg {
                        if let Some(vn) = library_sleigh.get_register(name) {
                            b = b.with_postcondition(Arc::new(gen_register_constraint(
                                vn,
                                *value as u64,
                            )));
                        } else {
                            event!(Level::WARN, "Unrecognized register name: {}", name);
                        }
                    }
                }
                if let Some(pointer) = &post.pointer {
                    for (name, value) in pointer {
                        if let Some(vn) = library_sleigh.get_register(name) {
                            b = b.with_postcondition(Arc::new(gen_register_pointer_constraint(
                                vn,
                                value.clone(),
                                c.pointer,
                            )))
                        }
                    }
                }
            }
            if let Some(pointer) = &c.pointer {
                b = b.with_pointer_invariant(Arc::new(gen_pointer_range_transition_invariant(
                    *pointer,
                )));
            }
        }
        b.build(z3, &library_sleigh)
            .map_err(CrackersBinError::from)
    }
}
