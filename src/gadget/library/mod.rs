use std::collections::{HashMap, HashSet};
use std::fmt::Display;
use std::fs::File;
use std::path::Path;
use std::slice::Iter;

use jingle::modeling::{ModeledBlock, ModeledInstruction};
use jingle::sleigh::context::SleighContext;
use jingle::sleigh::{Instruction, SpaceInfo, SpaceManager};
use jingle::JingleError;
use rand::rngs::StdRng;
use rand::seq::SliceRandom;
use rand::{random, SeedableRng};
use serde::{Deserialize, Serialize};
use tracing::{event, instrument, Level};
use z3::Context;

use crate::error::CrackersError;
use crate::error::CrackersError::{LibraryDeserialization, LibrarySerialization};
use crate::gadget::another_iterator::TraceCandidateIterator;
use crate::gadget::iterator::GadgetIterator;
use crate::gadget::library::builder::GadgetLibraryBuilder;
use crate::gadget::Gadget;

pub mod builder;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GadgetLibrary {
    pub(crate) gadgets: Vec<Gadget>,
    spaces: Vec<SpaceInfo>,
    default_code_space_index: usize,
}

impl GadgetLibrary {
    pub fn size(&self) -> usize {
        self.gadgets.len()
    }

    pub fn get_candidates_for_trace<'a, 'ctx>(
        &'a self,
        z3: &'ctx Context,
        trace: &[ModeledInstruction<'ctx>],
    ) -> impl Iterator<Item = Vec<Option<Gadget>>> + 'ctx {
        TraceCandidateIterator::new(z3, self.gadgets.clone().into_iter(), trace.to_vec())
    }
    pub fn get_gadgets_for_instruction<'a, 'ctx>(
        &'a self,
        z3: &'ctx Context,
        i: &Instruction,
    ) -> Result<GadgetIterator<'a, 'ctx>, CrackersError> {
        GadgetIterator::new(z3, self, i.clone())
    }

    pub(super) fn build_from_image(
        sleigh: &SleighContext,
        builder: &GadgetLibraryBuilder,
    ) -> Result<Self, JingleError> {
        let mut lib: GadgetLibrary = GadgetLibrary {
            gadgets: vec![],
            spaces: sleigh.get_all_space_info().to_vec(),
            default_code_space_index: sleigh.get_code_space_idx(),
        };
        event!(Level::INFO, "Loading gadgets from sleigh");
        for section in sleigh.image.sections.iter().filter(|s| s.perms.exec) {
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
        if let Some(random_sample_size) = builder.random_sample_size {
            let seed = builder.random_sample_seed.unwrap_or(random());
            event!(Level::INFO, "Using seed: {}", seed);
            let mut rng = StdRng::seed_from_u64(seed as u64);
            let rand_gadgets = lib
                .gadgets
                .choose_multiple(&mut rng, random_sample_size)
                .cloned();
            event!(Level::INFO, "Randomly selected {}", rand_gadgets.len());
            lib.gadgets = rand_gadgets.collect();
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
        if let Ok(r) = File::options()
            .create(true)
            .truncate(true)
            .write(true)
            .open(path)
        {
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

    use elf::endian::AnyEndian;
    use elf::ElfBytes;
    use jingle::sleigh::context::{Image, SleighContextBuilder};

    use crate::gadget::library::builder::GadgetLibraryBuilder;
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
            GadgetLibrary::build_from_image(&bin_sleigh, &GadgetLibraryBuilder::default()).unwrap();
    }
}
