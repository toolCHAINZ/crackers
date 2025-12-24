use jingle::JingleError;
use jingle::modeling::ModeledInstruction;
use jingle::sleigh::context::loaded::LoadedSleighContext;
use jingle::sleigh::{Instruction, SleighArchInfo};
use rand::SeedableRng;
use rand::rngs::StdRng;
use rand::seq::IndexedRandom;
use std::borrow::Borrow;
use tracing::{Level, event};

use crate::gadget::Gadget;
use crate::gadget::another_iterator::TraceCandidateIterator;
use crate::gadget::library::builder::GadgetLibraryConfig;

pub mod builder;
pub mod image;

#[derive(Clone, Debug)]
pub struct GadgetLibrary {
    pub(crate) gadgets: Vec<Gadget>,
    arch_info: SleighArchInfo,
    pub(crate) language_id: String,
}

impl AsRef<SleighArchInfo> for GadgetLibrary {
    fn as_ref(&self) -> &SleighArchInfo {
        &self.arch_info
    }
}

impl GadgetLibrary {
    pub fn size(&self) -> usize {
        self.gadgets.len()
    }

    pub(crate) fn arch_info(&self) -> SleighArchInfo {
        self.arch_info.clone()
    }
    pub fn get_random_candidates_for_trace<'a, S: Borrow<SleighArchInfo>>(
        &'a self,
        info: S,
        trace: &[ModeledInstruction],
        seed: i64,
    ) -> impl Iterator<Item = Vec<Option<&'a Gadget>>> {
        let mut rng = StdRng::seed_from_u64(seed as u64);
        let r = self.gadgets.choose_multiple(&mut rng, self.gadgets.len());
        TraceCandidateIterator::new(info, r, trace.to_vec())
    }
    pub(super) fn build_from_image(
        sleighs: Vec<LoadedSleighContext>,
        builder: &GadgetLibraryConfig,
    ) -> Result<Self, JingleError> {
        // We expect at least one sleigh (the primary library) to be provided.
        // Use the first sleigh's arch info / language id as the library-wide info.
        let mut iter = sleighs.into_iter();
        let first = iter.next().unwrap();
        let mut lib: GadgetLibrary = GadgetLibrary {
            gadgets: vec![],
            arch_info: first.arch_info().clone(),
            language_id: first.get_language_id().to_string(),
        };

        event!(Level::INFO, "Loading gadgets from sleighs");

        // process the first sleigh
        for section in first.get_sections().filter(|s| s.perms.exec) {
            let start = section.base_address as u64;
            let end = start + section.data.len() as u64;
            let mut curr = start;

            while curr < end {
                let instrs: Vec<Instruction> =
                    first.read(curr, builder.max_gadget_length).collect();
                if let Some(i) = instrs.iter().position(|b| b.terminates_basic_block()) {
                    let gadget = Gadget {
                        code_space_idx: first.arch_info().default_code_space_index(),
                        spaces: first.arch_info().spaces().to_vec(),
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

        // process remaining sleighs (additional loaded libraries)
        for sleigh in iter {
            for section in sleigh.get_sections().filter(|s| s.perms.exec) {
                let start = section.base_address as u64;
                let end = start + section.data.len() as u64;
                let mut curr = start;

                while curr < end {
                    let instrs: Vec<Instruction> =
                        sleigh.read(curr, builder.max_gadget_length).collect();
                    if let Some(i) = instrs.iter().position(|b| b.terminates_basic_block()) {
                        let gadget = Gadget {
                            code_space_idx: sleigh.arch_info().default_code_space_index(),
                            spaces: sleigh.arch_info().spaces().to_vec(),
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
        }

        Ok(lib)
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
            GadgetLibrary::build_from_image(vec![bin_sleigh], &GadgetLibraryConfig::default())
                .unwrap();
    }
}
