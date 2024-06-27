use std::fs;
use std::sync::Arc;

use jingle::sleigh::context::{map_gimli_architecture, Image, SleighContextBuilder};
use jingle::sleigh::RegisterManager;
use serde::Deserialize;
use tracing::{event, Level};
use z3::Context;

use crate::config::constraint::{
    gen_memory_constraint, gen_pointer_range_transition_invariant, gen_register_constraint,
    gen_register_pointer_constraint, Constraint,
};
use crate::config::error::CrackersConfigError;
use crate::config::error::CrackersConfigError::UnrecognizedArchitecture;
use crate::config::library::LibraryConfig;
use crate::config::meta::MetaConfig;
use crate::config::object::load_sleigh;
use crate::config::sleigh::SleighConfig;
use crate::config::specification::SpecificationConfig;
use crate::config::synthesis::SynthesisConfig;
use crate::error::CrackersError;
use crate::gadget::library::builder::GadgetLibraryBuilder;
use crate::synthesis::builder::SynthesisBuilder;
use crate::synthesis::AssignmentSynthesis;

mod constraint;
pub mod error;
mod library;
mod meta;
mod object;
mod sleigh;
mod specification;
mod synthesis;

#[derive(Debug, Deserialize)]
pub struct CrackersConfig {
    #[serde(default)]
    meta: MetaConfig,
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
    ) -> Result<AssignmentSynthesis<'z3>, CrackersError> {
        let library_sleigh = load_sleigh(&self.library.path, &self.sleigh)?;

        let mut gadget_library_params = GadgetLibraryBuilder::default();
        gadget_library_params
            .max_gadget_length(self.library.max_gadget_length)
            .with_seed(self.meta.seed);
        let mut b = SynthesisBuilder::default();
        b.with_gadget_library_builder(gadget_library_params)
            .seed(self.meta.seed)
            .specification(self.specification.get_spec(&self.sleigh)?.into_iter());
        if let Some(a) = &self.synthesis {
            b.with_selection_strategy(a.strategy);
            b.candidates_per_slot(a.max_candidates_per_slot);
            b.parallel(a.parallel).seed(self.meta.seed);
        }
        if let Some(c) = &self.constraint {
            for x in c.get_preconditions(&library_sleigh) {
                b.with_precondition(x);
            }
            for x in c.get_postconditions(&library_sleigh) {
                b.with_postcondition(x);
            }
            for x in c.get_pointer_constraints() {
                b.with_pointer_invariant(x);
            }
        }
        b.build(z3, &library_sleigh)
    }
}
