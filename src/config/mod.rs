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
use crate::gadget::library::builder::GadgetLibraryBuilder;
use crate::synthesis::AssignmentSynthesis;
use crate::synthesis::builder::SynthesisBuilder;

pub mod constraint;
pub mod error;
pub mod library;
pub mod meta;
pub mod object;
pub mod sleigh;
pub mod specification;
pub mod synthesis;

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
