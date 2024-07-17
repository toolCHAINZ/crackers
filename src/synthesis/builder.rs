use std::sync::Arc;

use derive_builder::Builder;
use jingle::modeling::{ModeledBlock, State};
use jingle::sleigh::Instruction;
use serde::Deserialize;
use z3::ast::Bool;
use z3::Context;

use crate::error::CrackersError;
use crate::gadget::library::builder::GadgetLibraryParams;
use crate::gadget::library::GadgetLibrary;
use crate::synthesis::AssignmentSynthesis;
use crate::synthesis::partition_iterator::Partition;

#[derive(Copy, Clone, Debug, Deserialize)]
pub enum SynthesisSelectionStrategy {
    #[serde(rename = "sat")]
    SatStrategy,
    #[serde(rename = "optimize")]
    OptimizeStrategy,
}

pub type StateConstraintGenerator = dyn for<'a, 'b> Fn(&'a Context, &'b State<'a>) -> Result<Bool<'a>, CrackersError>
    + Send
    + Sync
    + 'static;
pub type TransitionConstraintGenerator = dyn for<'a, 'b> Fn(&'a Context, &'b ModeledBlock<'a>) -> Result<Option<Bool<'a>>, CrackersError>
    + Send
    + Sync
    + 'static;

#[derive(Clone, Debug)]
pub enum Library {
    Library(GadgetLibrary),
    Params(GadgetLibraryParams),
}
#[derive(Clone, Builder)]
pub struct SynthesisParams {
    pub seed: i64,
    pub selection_strategy: SynthesisSelectionStrategy,
    #[builder(setter(custom))]
    pub gadget_library: Arc<GadgetLibrary>,
    pub candidates_per_slot: usize,
    pub parallel: usize,
    pub instructions: Vec<Instruction>,
    pub preconditions: Vec<Arc<StateConstraintGenerator>>,
    pub postconditions: Vec<Arc<StateConstraintGenerator>>,
    pub pointer_invariants: Vec<Arc<TransitionConstraintGenerator>>,
}

impl SynthesisParamsBuilder{
    pub fn gadget_library(&mut self, gadget_library: GadgetLibrary) -> &mut Self{
        self.gadget_library = Some(gadget_library.into());
        self
    }
}

impl SynthesisParams {
    pub fn build<'a>(&self, z3: &'a Context) -> Result<AssignmentSynthesis<'a>, CrackersError> {
        let s = AssignmentSynthesis::new(z3, self)?;
        Ok(s)
    }

    pub fn build_iter<'a: 'b, 'b>(
        &'b self,
        z3: &'a  Context,
    ) -> impl Iterator<Item = Result<AssignmentSynthesis<'a>, CrackersError>> + 'b{
        let mut base = self.clone();
        self.instructions.partitions().map(move |part| {
            let mut base = base.clone();
            let instrs: Vec<Instruction> = part
                .into_iter()
                .map(|instrs| Instruction::try_from(instrs).unwrap())
                .collect();
            base.instructions = instrs;
            AssignmentSynthesis::new(z3, &base)
        })
    }
}
