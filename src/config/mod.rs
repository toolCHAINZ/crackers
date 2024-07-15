use serde::Deserialize;
use z3::Context;

use crate::config::constraint::Constraint;
use crate::config::meta::MetaConfig;
use crate::config::sleigh::SleighConfig;
use crate::config::specification::SpecificationConfig;
use crate::config::synthesis::SynthesisConfig;
use crate::error::CrackersError;
use crate::gadget::library::builder::GadgetLibraryParams;
use crate::synthesis::AssignmentSynthesis;
use crate::synthesis::builder::SynthesisParamsBuilder;

pub mod constraint;
pub mod error;
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
    pub library: GadgetLibraryParams,
    pub sleigh: SleighConfig,
    pub constraint: Option<Constraint>,
    pub synthesis: SynthesisConfig,
}

impl CrackersConfig {
    pub fn resolve<'z3>(
        &self,
        z3: &'z3 Context,
    ) -> Result<AssignmentSynthesis<'z3>, CrackersError> {
        let library = self.library.build(&self.sleigh)?;
        let mut b = SynthesisParamsBuilder::default();
        if let Some(c) = &self.constraint {
            b.preconditions(c.get_preconditions(&library).collect());
            b.postconditions(c.get_postconditions(&library).collect());
            b.pointer_invariants(c.get_pointer_constraints().collect());
        }
        b.gadget_library(library)
            .seed(self.meta.seed)
            .instructions(self.specification.get_spec(&self.sleigh)?);
        b.selection_strategy(self.synthesis.strategy);
        b.candidates_per_slot(self.synthesis.max_candidates_per_slot);
        b.parallel(self.synthesis.parallel).seed(self.meta.seed);

        b.build()?.build(z3)
    }
}
