use crackers::config::constraint::Constraint;
use crackers::config::meta::MetaConfig;
use crackers::config::sleigh::SleighConfig;
use crackers::config::synthesis::SynthesisConfig;
use crackers::error::CrackersError;
use crackers::gadget::library::builder::GadgetLibraryParams;
use crackers::synthesis::builder::{SynthesisParams, SynthesisParamsBuilder};
use jingle::sleigh::Instruction;
use serde::Deserialize;

#[derive(Clone, Debug, Deserialize)]
pub struct CrackersGptConfig {
    #[serde(default)]
    pub(crate) meta: MetaConfig,
    pub(crate) constraint: Option<Constraint>,
    pub(crate) library: GadgetLibraryParams,
    pub(crate) synthesis: SynthesisConfig,
    pub(crate) sleigh: SleighConfig,
}

impl CrackersGptConfig {
    pub fn resolve(&self, spec: Vec<Instruction>) -> Result<SynthesisParams, CrackersError> {
        let library = self.library.build(&self.sleigh)?;
        let mut b = SynthesisParamsBuilder::default();
        if let Some(c) = &self.constraint {
            b.preconditions(c.get_preconditions(&library).collect());
            b.postconditions(c.get_postconditions(&library).collect());
            b.pointer_invariants(c.get_pointer_constraints().collect());
        }
        b.gadget_library(library)
            .seed(self.meta.seed)
            .instructions(spec);
        b.selection_strategy(self.synthesis.strategy);
        b.candidates_per_slot(self.synthesis.max_candidates_per_slot);
        b.parallel(self.synthesis.parallel).seed(self.meta.seed);

        let params = b.build()?;
        Ok(params)
    }
}
