use std::collections::HashMap;

use jingle::JingleError;
use jingle::modeling::ModeledInstruction;
use jingle::sleigh::{Instruction, RegisterManager, SpaceInfo, SpaceManager, VarNode};
use jingle::sleigh::context::loaded::LoadedSleighContext;
use rand::rngs::StdRng;
use rand::SeedableRng;
use rand::seq::SliceRandom;
use tracing::{event, Level};
use z3::Context;

use crate::gadget::another_iterator::TraceCandidateIterator;
use crate::gadget::Gadget;
use crate::gadget::library::builder::GadgetLibraryParams;

pub mod builder;

#[derive(Clone, Debug)]
pub struct GadgetLibrary {
    pub(crate) gadgets: Vec<Gadget>,
    spaces: Vec<SpaceInfo>,
    default_code_space_index: usize,
    register_to_varnode: HashMap<String, VarNode>,
    varnode_to_register: HashMap<VarNode, String>,
}

impl GadgetLibrary {
    pub fn size(&self) -> usize {
        self.gadgets.len()
    }

    pub fn get_random_candidates_for_trace<'ctx, 'a: 'ctx>(
        &'a self,
        z3: &'ctx Context,
        trace: &[ModeledInstruction<'ctx>],
        seed: i64,
    ) -> impl Iterator<Item = Vec<Option<&'a Gadget>>> + 'ctx {
        let mut rng = StdRng::seed_from_u64(seed as u64);
        let r = self.gadgets.choose_multiple(&mut rng, self.gadgets.len());
        TraceCandidateIterator::new(z3, r, trace.to_vec())
    }
    pub(super) fn build_from_image(
        sleigh: LoadedSleighContext,
        builder: &GadgetLibraryParams,
    ) -> Result<Self, JingleError> {
        let spaces = sleigh.get_all_space_info().to_vec();
        let default_code_space_index = sleigh.get_code_space_idx();
        let mut register_to_varnode = HashMap::new();
        let mut varnode_to_register = HashMap::new();
        for (varnode, register) in sleigh.get_registers() {
            register_to_varnode.insert(register.clone(), varnode.clone());
            varnode_to_register.insert(varnode, register);
        }
        let mut lib: GadgetLibrary = GadgetLibrary {
            gadgets: vec![],
            spaces,
            default_code_space_index,
            register_to_varnode,
            varnode_to_register,
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
                        spaces: sleigh.get_all_space_info().to_vec(),
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

impl SpaceManager for GadgetLibrary {
    fn get_space_info(&self, idx: usize) -> Option<&SpaceInfo> {
        self.spaces.get(idx)
    }

    fn get_all_space_info(&self) -> &[SpaceInfo] {
        self.spaces.as_slice()
    }

    fn get_code_space_idx(&self) -> usize {
        self.default_code_space_index
    }
}

impl RegisterManager for GadgetLibrary {
    fn get_register(&self, name: &str) -> Option<VarNode> {
        self.register_to_varnode.get(name).cloned()
    }

    fn get_register_name(&self, location: &VarNode) -> Option<&str> {
        self.varnode_to_register.get(&location).map(|c| c.as_str())
    }

    fn get_registers(&self) -> Vec<(VarNode, String)> {
        self.varnode_to_register.clone().into_iter().collect()
    }
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::Path;

    use elf::ElfBytes;
    use elf::endian::AnyEndian;
    use jingle::sleigh::context::SleighContextBuilder;

    use crate::gadget::library::builder::GadgetLibraryParams;
    use crate::gadget::library::GadgetLibrary;

    #[test]
    fn test_load_library() {
        let builder =
            SleighContextBuilder::load_ghidra_installation(Path::new("/Applications/ghidra"))
                .unwrap();
        let path = Path::new("../bin/vuln");
        let data = fs::read(path).unwrap();
        let elf = ElfBytes::<AnyEndian>::minimal_parse(data.as_slice()).unwrap();

        let bin_sleigh = builder
            .set_image(Image::try_from(elf).unwrap())
            .build("x86:LE:64:default")
            .unwrap();
        let _lib =
            GadgetLibrary::build_from_image(bin_sleigh, &GadgetLibraryParams::default()).unwrap();
    }
}

