use serde::Deserialize;
use z3::Context;

use crate::config::constraint::Constraint;
use crate::config::library::LibraryConfig;
use crate::config::meta::MetaConfig;
use crate::config::object::load_sleigh;
use crate::config::sleigh::SleighConfig;
use crate::config::specification::SpecificationConfig;
use crate::config::synthesis::SynthesisConfig;
use crate::error::CrackersError;
use crate::gadget::library::builder::GadgetLibraryParamsBuilder;
use crate::synthesis::builder::{Library, SynthesisParamsBuilder};
use crate::synthesis::AssignmentSynthesis;

pub mod constraint;
pub mod error;
pub mod library;
pub mod meta;
pub mod object;
pub mod sleigh;
pub mod specification;
pub mod synthesis;

#[derive(Clone, Debug, Deserialize)]
pub struct CrackersConfig {
    #[serde(default)]
    pub meta: MetaConfig,
    pub specification: SpecificationConfig,
    pub library: LibraryConfig,
    pub sleigh: SleighConfig,
    pub constraint: Option<Constraint>,
    pub synthesis: SynthesisConfig,
}

impl CrackersConfig {
    pub fn resolve<'z3>(
        &self,
        z3: &'z3 Context,
    ) -> Result<AssignmentSynthesis<'z3>, CrackersError> {
        let library_sleigh = load_sleigh(&self.library.path, &self.sleigh)?;

        let library = GadgetLibraryParamsBuilder::default()
            .seed(self.meta.seed)
            .max_gadget_length(self.library.max_gadget_length)
            .build()?
            .build(&library_sleigh)?;
        let mut b = SynthesisParamsBuilder::default();
        b.gadget_library_builder(Library::Library(library))
            .seed(self.meta.seed)
            .instructions(self.specification.get_spec(&self.sleigh)?);
        b.selection_strategy(self.synthesis.strategy);
        b.candidates_per_slot(self.synthesis.max_candidates_per_slot);
        b.parallel(self.synthesis.parallel).seed(self.meta.seed);

        if let Some(c) = &self.constraint {
            b.preconditions(c.get_preconditions(&library_sleigh).collect());
            b.postconditions(c.get_postconditions(&library_sleigh).collect());
            b.pointer_invariants(c.get_pointer_constraints().collect());
        }
        b.build()?.build(z3, &library_sleigh)
    }
}
