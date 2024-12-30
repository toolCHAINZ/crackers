use jingle::modeling::ModeledInstruction;
use jingle::sleigh::context::loaded::LoadedSleighContext;
use jingle::sleigh::{Instruction, RegisterManager, SpaceInfo, SpaceManager, VarNode};
use jingle::{JingleContext, JingleError};
use rand::rngs::StdRng;
use rand::seq::SliceRandom;
use rand::SeedableRng;
use std::collections::HashMap;
use std::sync::Arc;
use tracing::{event, Level};

use crate::gadget::another_iterator::TraceCandidateIterator;
use crate::gadget::library::builder::GadgetLibraryParams;
use crate::gadget::Gadget;
use crate::synthesis::builder::StateConstraintGenerator;

pub mod builder;
pub mod image;

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
        jingle: &JingleContext<'ctx>,
        trace_length: usize,
        postconditions: &[Arc<StateConstraintGenerator>],
        seed: i64,
    ) -> impl Iterator<Item = Vec<Option<&'a Gadget>>> + 'ctx {
        let mut rng = StdRng::seed_from_u64(seed as u64);
        let r = self.gadgets.choose_multiple(&mut rng, self.gadgets.len());
        TraceCandidateIterator::new(jingle, r, trace_length, postconditions.to_vec())
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
        self.varnode_to_register.get(location).map(|c| c.as_str())
    }

    fn get_registers(&self) -> Vec<(VarNode, String)> {
        self.varnode_to_register.clone().into_iter().collect()
    }
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::Path;

    use crate::gadget::library::builder::GadgetLibraryParams;
    use crate::gadget::library::GadgetLibrary;
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
            GadgetLibrary::build_from_image(bin_sleigh, &GadgetLibraryParams::default()).unwrap();
    }
}

