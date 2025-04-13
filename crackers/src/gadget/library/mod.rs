use jingle::modeling::ModeledInstruction;
use jingle::sleigh::context::loaded::LoadedSleighContext;
use jingle::sleigh::{ArchInfoProvider, Instruction, SpaceInfo, VarNode};
use jingle::{JingleContext, JingleError};
use rand::SeedableRng;
use rand::rngs::StdRng;
use rand::seq::SliceRandom;
use tracing::{Level, event};

use crate::gadget::Gadget;
use crate::gadget::another_iterator::TraceCandidateIterator;
use crate::gadget::library::builder::GadgetLibraryConfig;

pub mod builder;
pub mod image;

#[derive(Clone, Debug)]
pub struct GadgetLibrary {
    pub(crate) gadgets: Vec<Gadget>,
    spaces: Vec<SpaceInfo>,
    default_code_space_index: usize,
    registers: Vec<(VarNode, String)>,
}

impl GadgetLibrary {
    pub fn size(&self) -> usize {
        self.gadgets.len()
    }

    pub fn get_random_candidates_for_trace<'ctx, 'a: 'ctx>(
        &'a self,
        jingle: &JingleContext<'ctx>,
        trace: &[ModeledInstruction<'ctx>],
        seed: i64,
    ) -> impl Iterator<Item = Vec<Option<&'a Gadget>>> + 'ctx {
        let mut rng = StdRng::seed_from_u64(seed as u64);
        let r = self.gadgets.choose_multiple(&mut rng, self.gadgets.len());
        TraceCandidateIterator::new(jingle, r, trace.to_vec())
    }
    pub(super) fn build_from_image(
        sleigh: LoadedSleighContext,
        builder: &GadgetLibraryConfig,
    ) -> Result<Self, JingleError> {
        let spaces: Vec<_> = sleigh.get_all_space_info().cloned().collect();
        let mut registers = vec![];
        let default_code_space_index = sleigh.get_code_space_idx();
        for (varnode, register) in sleigh.get_registers() {
            registers.push((varnode.clone(), register));
        }
        let mut lib: GadgetLibrary = GadgetLibrary {
            gadgets: vec![],
            spaces,
            default_code_space_index,
            registers: registers
                .iter()
                .map(|(varnode, register)| (varnode.clone(), register.to_string()))
                .collect(),
        };
        event!(Level::INFO, "Loading gadgets from sleigh");
        for section in sleigh.get_sections().filter(|s| s.perms.exec) {
            let start = section.base_address as u64;
            let end = start + section.data.len() as u64;
            let mut curr = start;

            while curr < end {
                let instrs: Vec<Instruction> =
                    sleigh.read(curr, builder.max_gadget_length).collect();
                if let Some(i) = instrs.iter().position(|b| b.terminates_basic_block()) {
                    let gadget = Gadget {
                        code_space_idx: sleigh.get_code_space_idx(),
                        spaces: sleigh.get_all_space_info().cloned().collect(),
                        instructions: instrs[0..=i].to_vec(),
                    };
                    if !gadget.has_blacklisted_op(&builder.operation_blacklist) {
                        lib.gadgets.push(gadget);
                    }
                }
                curr += 1
            }
            event!(Level::INFO, "Found {} gadgets...", lib.gadgets.len());
        }
        Ok(lib)
    }
}

impl ArchInfoProvider for GadgetLibrary {
    fn get_space_info(&self, idx: usize) -> Option<&SpaceInfo> {
        self.spaces.get(idx)
    }
    fn get_all_space_info(&self) -> impl Iterator<Item = &SpaceInfo> {
        self.spaces.iter()
    }

    fn get_code_space_idx(&self) -> usize {
        self.default_code_space_index
    }

    fn get_register(&self, name: &str) -> Option<&VarNode> {
        self.registers
            .iter()
            .find(|(_, reg_name)| reg_name.as_str() == name)
            .map(|(vn, _)| vn)
    }

    fn get_register_name(&self, location: &VarNode) -> Option<&str> {
        self.registers
            .iter()
            .find(|(vn, _)| vn == location)
            .map(|(_, name)| name.as_str())
    }

    fn get_registers(&self) -> impl Iterator<Item = (&VarNode, &str)> {
        self.registers.iter().map(|(vn, name)| (vn, name.as_str()))
    }
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::Path;

    use crate::gadget::library::GadgetLibrary;
    use crate::gadget::library::builder::GadgetLibraryConfig;
    use jingle::sleigh::context::SleighContextBuilder;
    use object::File;

    #[test]
    fn test_load_library() {
        let builder =
            SleighContextBuilder::load_ghidra_installation(Path::new("/Applications/ghidra"))
                .unwrap();
        let path = Path::new("../bin/vuln");
        let data = fs::read(path).unwrap();
        let file = File::parse(&*data).unwrap();
        let sleigh = builder.build("x86:LE:64:default").unwrap();
        let bin_sleigh = sleigh.initialize_with_image(file).unwrap();
        let _lib =
            GadgetLibrary::build_from_image(bin_sleigh, &GadgetLibraryConfig::default()).unwrap();
    }
}
