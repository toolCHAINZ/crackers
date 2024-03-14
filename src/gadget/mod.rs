use jingle::modeling::ModeledBlock;
use jingle::sleigh::context::SleighContext;
use jingle::sleigh::{Instruction, SpaceInfo, SpaceManager};
use jingle::JingleError;
use serde::{Deserialize, Serialize};
use z3::Context;

use crate::gadget::iterator::GadgetIterator;
use crate::gadget::signature::OutputSignature;

mod error;
mod iterator;
pub mod signature;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Gadget {
    pub instructions: Vec<Instruction>,
}

impl Gadget {
    pub fn address(&self) -> Option<u64> {
        self.instructions.first().map(|f| f.address)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GadgetLibrary {
    gadgets: Vec<Gadget>,
    spaces: Vec<SpaceInfo>,
    default_code_space_index: usize,
}

impl GadgetLibrary {
    pub fn model_gadget<'ctx>(
        &self,
        z3: &'ctx Context,
        gadget: &Gadget,
    ) -> Result<ModeledBlock<'ctx>, JingleError> {
        for x in &gadget.instructions {
            println!("{:x}: {}", x.address, x.disassembly);
        }
        dbg!(ModeledBlock::read(
            z3,
            self,
            gadget.instructions.clone().into_iter()
        ))
    }

    pub fn size(&self) -> usize {
        self.gadgets.len()
    }

    pub fn get_modeled_gadgets_for_instruction<'a, 'ctx>(
        &'a self,
        z3: &'ctx Context,
        i: &Instruction,
    ) -> GadgetIterator<'a, 'ctx> {
        GadgetIterator::new(z3, self, OutputSignature::from(i))
    }

    pub fn build_from_image(sleigh: &SleighContext) -> Result<Self, JingleError> {
        let mut lib: GadgetLibrary = GadgetLibrary {
            gadgets: vec![],
            spaces: sleigh.get_all_space_info().to_vec(),
            default_code_space_index: sleigh.get_code_space_idx(),
        };
        for section in sleigh.image.sections.iter().filter(|s| s.perms.exec) {
            let start = section.base_address as u64;
            let end = start + section.data.len() as u64;
            let mut curr = start;

            while curr < end {
                let instrs: Vec<Instruction> = sleigh.read(curr, 10).collect();
                if let Some(i) = instrs.iter().position(|b| b.terminates_basic_block()) {
                    lib.gadgets.push(Gadget {
                        instructions: instrs[0..i].to_vec(),
                    })
                }
                curr += 1
            }
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

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::Path;

    use elf::endian::AnyEndian;
    use elf::ElfBytes;
    use jingle::sleigh::context::{Image, SleighContextBuilder};

    use crate::gadget::GadgetLibrary;

    #[test]
    fn test_load_library() {
        let builder =
            SleighContextBuilder::load_ghidra_installation(Path::new("/Applications/ghidra"))
                .unwrap();
        let path = Path::new("bin/vuln");
        let data = fs::read(path).unwrap();
        let elf = ElfBytes::<AnyEndian>::minimal_parse(data.as_slice()).unwrap();

        let bin_sleigh = builder
            .set_image(Image::try_from(elf).unwrap())
            .build("x86:LE:64:default")
            .unwrap();
        let _lib = GadgetLibrary::build_from_image(&bin_sleigh).unwrap();
    }
}
