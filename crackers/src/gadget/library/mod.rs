use std::collections::{HashMap, HashSet};
use std::fmt::Display;
use std::fs::File;
use std::path::Path;

use jingle::JingleError;
use jingle::modeling::ModeledBlock;
use jingle::sleigh::{Instruction, OpCode, SpaceInfo, SpaceManager};
use jingle::sleigh::context::SleighContext;
use serde::{Deserialize, Serialize};
use tracing::{event, instrument, Level};
use z3::Context;

use crate::error::CrackersError;
use crate::error::CrackersError::{LibraryDeserialization, LibrarySerialization};
use crate::gadget::Gadget;
use crate::gadget::iterator::ModeledGadgetIterator;
use crate::gadget::signature::OutputSignature;

pub mod builder;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GadgetLibrary {
    pub(crate) gadgets: Vec<Gadget>,
    pub(crate) ancestor_graph: HashMap<u64, HashSet<u64>>,
    spaces: Vec<SpaceInfo>,
    default_code_space_index: usize,
}

impl GadgetLibrary {
    pub fn size(&self) -> usize {
        self.gadgets.len()
    }

    pub fn get_modeled_gadgets_for_instruction<'a, 'ctx>(
        &'a self,
        z3: &'ctx Context,
        i: &Instruction,
    ) -> ModeledGadgetIterator<'a, 'ctx> {
        ModeledGadgetIterator::new(z3, self, OutputSignature::from(i))
    }

    pub(super) fn build_from_image(
        sleigh: &SleighContext,
        len: usize,
        operation_blacklist: &HashSet<OpCode>,
    ) -> Result<Self, JingleError> {
        let mut lib: GadgetLibrary = GadgetLibrary {
            gadgets: vec![],
            ancestor_graph: HashMap::new(),
            spaces: sleigh.get_all_space_info().to_vec(),
            default_code_space_index: sleigh.get_code_space_idx(),
        };
        event!(Level::INFO, "Loading gadgets from sleigh");
        for section in sleigh.image.sections.iter().filter(|s| s.perms.exec) {
            let start = section.base_address as u64;
            let end = start + section.data.len() as u64;
            let mut curr = start;

            while curr < end {
                let instrs: Vec<Instruction> = sleigh.read(curr, len).collect();
                if let Some(i) = instrs.iter().position(|b| b.terminates_basic_block()) {
                    for x in instrs.iter().skip(1) {
                        if let Some(mut v) = lib.ancestor_graph.get_mut(&x.address) {
                            v.insert(curr);
                        } else {
                            lib.ancestor_graph.insert(x.address, HashSet::from([curr]));
                        }
                    }
                    let gadget = Gadget {
                        instructions: instrs[0..=i].to_vec(),
                    };
                    if !gadget.has_blacklisted_op(operation_blacklist) {
                        lib.gadgets.push(gadget);
                    }
                }
                curr += 1
            }
            event!(Level::INFO, "Found {} gadgets...", lib.gadgets.len());
        }
        Ok(lib)
    }

    #[instrument(skip_all, fields(%path))]
    pub fn load_from_file<T: AsRef<Path> + Display>(path: &T) -> Result<Self, CrackersError> {
        if let Ok(r) = File::options().read(true).open(path) {
            event!(Level::INFO, "Loading gadget library...");
            return rmp_serde::from_read(r).map_err(|_| LibraryDeserialization);
        }
        Err(LibraryDeserialization)
    }

    #[instrument(skip_all, fields(%path))]
    pub fn write_to_file<T: AsRef<Path> + Display>(&self, path: &T) -> Result<(), CrackersError> {
        if let Ok(r) = File::options().create(true).write(true).open(path) {
            event!(Level::INFO, "Writing gadget library...");

            return self
                .serialize(&mut rmp_serde::Serializer::new(&r))
                .map_err(|_| LibrarySerialization);
        }
        Err(LibrarySerialization)
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

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::Path;

    use elf::ElfBytes;
    use elf::endian::AnyEndian;
    use jingle::sleigh::context::{Image, SleighContextBuilder};

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
        let _lib = GadgetLibrary::build_from_image(&bin_sleigh, 4).unwrap();
    }
}
