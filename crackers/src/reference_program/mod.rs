use crate::config::error::CrackersConfigError;
use crate::config::error::CrackersConfigError::{
    SpecMissingStartSymbol, SpecMissingTextSection, UnrecognizedArchitecture,
};
use crate::config::sleigh::SleighConfig;
use crate::config::specification::SpecificationConfig;
use crate::reference_program::step::Step;
use crate::synthesis::partition_iterator::Partition;
use anyhow::Context;
use jingle::analysis::varnode::VarNodeSet;
use jingle::sleigh::context::image::gimli::map_gimli_architecture;
use jingle::sleigh::context::loaded::LoadedSleighContext;
use jingle::sleigh::{GeneralizedVarNode, VarNode};
use object::{File, Object, ObjectSymbol};
use std::collections::HashMap;
use std::fs;
use tracing::Instrument;

mod step;

#[derive(Debug, Clone)]
pub struct ReferenceProgram {
    steps: Vec<Step>,
    initial_memory: HashMap<VarNode, Vec<u8>>,
}

impl ReferenceProgram {
    pub fn try_load(
        spec: &SpecificationConfig,
        sleigh_config: &SleighConfig,
    ) -> Result<Self, CrackersConfigError> {
        let bytes = fs::read(&spec.path)?;
        let gimli_file = File::parse(&*bytes)?;
        let sleigh_context_builder = sleigh_config.context_builder()?;

        let sym = gimli_file
            .symbol_by_name("_start")
            .ok_or(SpecMissingStartSymbol)?;
        let _section = gimli_file
            .section_by_name(".text")
            .ok_or(SpecMissingTextSection)?;
        let arch = map_gimli_architecture(&gimli_file).ok_or(UnrecognizedArchitecture(format!(
            "{:?}",
            gimli_file.architecture()
        )))?;
        let sleigh = sleigh_context_builder.build(arch)?;
        let mut sleigh = sleigh.initialize_with_image(&gimli_file)?;
        let mut addr = sym.address();
        if let Some(o) = spec.base_address {
            sleigh.set_base_address(o);
            addr = addr.wrapping_add(o);
        }
        let steps: Vec<_> = sleigh
            .read_until_branch(addr, spec.max_instructions)
            .map(Step::from_instr)
            .collect();
        Ok(Self {
            steps,
            initial_memory: HashMap::new(),
        })
    }

    fn calc_initial_memory_valuation(
        steps: &[Step],
        image: LoadedSleighContext<'_>,
    ) -> HashMap<VarNode, Vec<u8>> {
        let mut covering_set = VarNodeSet::default();
        let mut valuation = HashMap::new();
        for x in steps
            .iter()
            .flat_map(|step| step.instructions())
            .flat_map(|i| i.ops.clone())
        {
            for vn in x.inputs() {
                match vn {
                    GeneralizedVarNode::Direct(vn) => {
                        covering_set.insert(&vn);
                    }
                    _ => {}
                }
            }
        }
        for x in covering_set.varnodes() {
            if let Some(b) = image.read_bytes(&x){
                valuation.insert(x, b);
            }
        }
        valuation
    }

    pub fn partitions(&self) -> impl Iterator<Item = Self> {
        let init = self.initial_memory.clone();
        self.steps.partitions().map(move |steps| {
            let steps: Vec<_> = steps.into_iter().map(|s| Step::combine(s.iter())).collect();
            Self {
                steps,
                initial_memory: init.clone(),
            }
        })
    }
}
