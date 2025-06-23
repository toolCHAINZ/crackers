use crate::error::CrackersError;
use jingle::JingleError::Sleigh;
use jingle::sleigh::JingleSleighError::ImageLoadError;
use jingle::sleigh::VarNode;
use jingle::sleigh::context::image::{ImageProvider, ImageSection, ImageSectionIterator, Perms};
use object::elf::{PF_R, PF_W, PF_X};
use object::macho::{VM_PROT_EXECUTE, VM_PROT_READ, VM_PROT_WRITE};
use object::pe::{IMAGE_SCN_MEM_EXECUTE, IMAGE_SCN_MEM_READ, IMAGE_SCN_MEM_WRITE};
use object::{File, Object, ObjectSegment, Segment, SegmentFlags};
use std::cmp::{max, min};

#[derive(Debug, PartialEq, Eq)]
pub struct ImageSegment {
    data: Vec<u8>,
    perms: Perms,
    base_address: usize,
}

impl<'a> From<&'a ImageSegment> for ImageSection<'a> {
    fn from(value: &'a ImageSegment) -> Self {
        ImageSection {
            data: value.data.as_slice(),
            perms: value.perms.clone(),
            base_address: value.base_address,
        }
    }
}

impl TryFrom<Segment<'_, '_>> for ImageSegment {
    type Error = CrackersError;

    fn try_from(value: Segment) -> Result<Self, Self::Error> {
        let data = value
            .data()
            .map_err(|_| CrackersError::Jingle(Sleigh(ImageLoadError)))?
            .to_vec();
        Ok(ImageSegment {
            data,
            perms: map_seg_flags(&value.flags())?,
            base_address: value.address() as usize,
        })
    }
}

/// todo: this should go in jingle
fn map_seg_flags(p0: &SegmentFlags) -> Result<Perms, CrackersError> {
    match p0 {
        SegmentFlags::None => Ok(Perms::RWX),
        SegmentFlags::Elf { p_flags } => Ok(Perms {
            read: p_flags & PF_R != 0,
            write: p_flags & PF_W != 0,
            exec: p_flags & PF_X != 0,
        }),
        SegmentFlags::MachO { maxprot, .. } => Ok(Perms {
            read: maxprot & VM_PROT_READ != 0,
            write: maxprot & VM_PROT_WRITE != 0,
            exec: maxprot & VM_PROT_EXECUTE != 0,
        }),
        SegmentFlags::Coff { characteristics } => Ok(Perms {
            read: characteristics & IMAGE_SCN_MEM_READ != 0,
            write: characteristics & IMAGE_SCN_MEM_WRITE != 0,
            exec: characteristics & IMAGE_SCN_MEM_EXECUTE != 0,
        }),
        _ => Err(CrackersError::Jingle(Sleigh(ImageLoadError))),
    }
}

/// A gross hack because we want to process the entire executable segment, rather
/// than the portion that is marked executable for the linker
pub struct SegmentFile {
    segments: Vec<ImageSegment>,
}

impl SegmentFile {
    pub fn new(file: &File) -> Result<Self, CrackersError> {
        let mut segments = vec![];
        for x in file
            .segments()
            .filter(|f| map_seg_flags(&f.flags()).map(|f| f.exec).unwrap_or(false))
        {
            segments.push(x.try_into()?);
        }
        Ok(Self { segments })
    }
}

impl ImageProvider for SegmentFile {
    fn load(&self, vn: &VarNode, output: &mut [u8]) -> usize {
        let mut written = 0;
        output.fill(0);
        let output_start_addr = vn.offset as usize;
        let output_end_addr = output_start_addr + vn.size;
        if let Some(x) = self.get_section_info().find(|s| {
            output_start_addr >= s.base_address
                && output_start_addr < (s.base_address + s.data.len())
        }) {
            let input_start_addr = x.base_address;
            let input_end_addr = input_start_addr + x.data.len();
            let start_addr = max(input_start_addr, output_start_addr);
            let end_addr = max(min(input_end_addr, output_end_addr), start_addr);
            if end_addr > start_addr {
                let i_s = start_addr - x.base_address;
                let i_e = end_addr - x.base_address;
                let o_s = start_addr - vn.offset as usize;
                let o_e = end_addr - vn.offset as usize;
                let out_slice = &mut output[o_s..o_e];
                let in_slice = &x.data[i_s..i_e];
                out_slice.copy_from_slice(in_slice);
                written += end_addr - start_addr;
            }
        }
        written
    }

    fn has_full_range(&self, vn: &VarNode) -> bool {
        self.get_section_info().any(|s| {
            s.base_address <= vn.offset as usize
                && (s.base_address + s.data.len()) >= (vn.offset as usize + vn.size)
        })
    }

    fn get_section_info(&self) -> ImageSectionIterator<'_> {
        ImageSectionIterator::new(self.segments.iter().map(ImageSection::from))
    }
}
