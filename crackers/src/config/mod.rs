use crate::config::constraint::ConstraintConfig;
use crate::config::meta::MetaConfig;
use crate::config::sleigh::SleighConfig;
use crate::config::specification::SpecificationConfig;
use crate::config::synthesis::SynthesisConfig;
use crate::error::CrackersError;
use crate::gadget::library::builder::GadgetLibraryConfig;
use crate::reference_program::ReferenceProgram;
use crate::synthesis::builder::{SynthesisParams, SynthesisParamsBuilder};
use serde::{Deserialize, Serialize};

pub mod constraint;
pub mod error;
pub mod meta;
pub mod object;
pub mod sleigh;
pub mod specification;
pub mod synthesis;

#[derive(Clone, Debug, Deserialize, Serialize)]
/// This struct represents the serializable configuration found
/// in a crackers .toml file. Once parsed from a file or constructed
/// programmatically, it can be used to produce a [crate::synthesis::builder::SynthesisParams]
/// struct, which can run the actual algorithm
pub struct CrackersConfig {
    #[serde(default)]
    pub meta: MetaConfig,
    pub specification: SpecificationConfig,
    pub library: GadgetLibraryConfig,
    pub sleigh: SleighConfig,
    pub synthesis: SynthesisConfig,
    pub constraint: Option<ConstraintConfig>,
}

impl CrackersConfig {
    pub fn resolve(&self) -> Result<SynthesisParams, CrackersError> {
        let library = self.library.build(&self.sleigh)?;
        let lang_id = library.language_id.clone();
        let mut b = SynthesisParamsBuilder::default();
        if let Some(c) = &self.constraint {
            b.preconditions(c.get_preconditions(&library.arch_info()).collect());
            b.postconditions(c.get_postconditions(&library.arch_info()).collect());
            b.pointer_invariants(c.get_pointer_constraints().collect());
        }
        b.gadget_library(library)
            .seed(self.meta.seed)
            .reference_program(ReferenceProgram::try_load(
                &self.specification,
                &self.sleigh,
                &self.library.operation_blacklist,
                &lang_id,
            )?);
        b.selection_strategy(self.synthesis.strategy);
        b.combine_instructions(self.synthesis.combine_instructions);
        b.candidates_per_slot(self.synthesis.max_candidates_per_slot);
        b.parallel(self.synthesis.parallel).seed(self.meta.seed);

        let params = b.build()?;
        Ok(params)
    }
}
