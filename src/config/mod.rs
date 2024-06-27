use std::fs;
use std::sync::Arc;

use jingle::sleigh::context::{Image, map_gimli_architecture, SleighContextBuilder};
use jingle::sleigh::RegisterManager;
use object::{File, Object};
use serde::Deserialize;
use tracing::{event, Level};
use z3::Context;

use crate::config::constraint::{
    Constraint, gen_memory_constraint, gen_pointer_range_transition_invariant,
    gen_register_constraint, gen_register_pointer_constraint,
};
use crate::config::error::CrackersConfigError;
use crate::config::error::CrackersConfigError::UnrecognizedArchitecture;
use crate::config::library::LibraryConfig;
use crate::config::sleigh::SleighConfig;
use crate::config::specification::SpecificationConfig;
use crate::config::synthesis::SynthesisConfig;
use crate::gadget::library::builder::GadgetLibraryBuilder;
use crate::synthesis::AssignmentSynthesis;
use crate::synthesis::builder::SynthesisBuilder;

mod constraint;
pub mod error;
mod library;
pub mod random;
mod sleigh;
mod specification;
mod synthesis;

#[derive(Debug, Deserialize)]
pub struct CrackersConfig {
    specification: SpecificationConfig,
    library: LibraryConfig,
    sleigh: SleighConfig,
    constraint: Option<Constraint>,
    pub synthesis: Option<SynthesisConfig>,
}

impl CrackersConfig {
    fn get_sleigh_builder(&self) -> Result<SleighContextBuilder, CrackersConfigError> {
        let builder = SleighContextBuilder::load_ghidra_installation(&self.sleigh.ghidra_path)?;
        Ok(builder)
    }

    fn load_library_image(&self) -> Result<Vec<u8>, CrackersConfigError> {
        let data = fs::read(&self.library.path)?;
        Ok(data)
    }

    fn load_spec(&self) -> Result<Image, CrackersConfigError> {
        let data = fs::read(&self.specification.path)?;
        Ok(Image::from(data))
    }
    pub fn resolve<'z3>(
        &self,
        z3: &'z3 Context,
    ) -> Result<AssignmentSynthesis<'z3>, CrackersConfigError> {
        let spec_sleigh_builder = self.get_sleigh_builder()?;
        let library_sleigh_builder = self.get_sleigh_builder()?;

        let data = self.load_library_image()?;
        let library_image = File::parse(&*data)?;
        let spec_image = self.load_spec()?;

        let architecture_str = map_gimli_architecture(&library_image).ok_or(
            UnrecognizedArchitecture(format!("{:?}", library_image.architecture())),
        )?;
        event!(
            Level::INFO,
            "Using SLEIGH architecture {}",
            architecture_str
        );
        let library_image = Image::try_from(library_image)?;
        let spec_sleigh = spec_sleigh_builder
            .set_image(spec_image)
            .build(architecture_str)?;
        let library_sleigh = library_sleigh_builder
            .set_image(library_image)
            .build(architecture_str)?;

        let gadget_library_params = GadgetLibraryBuilder::default()
            .max_gadget_length(self.library.max_gadget_length)
            .random_sample_seed(self.library.random)
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
        b.build(z3, &library_sleigh).map_err(CrackersBinError::from)
    }
}
