use jingle::{
    display::JingleDisplayable,
    sleigh::{Disassembly, Instruction, PcodeOperation, SleighArchInfo, context::SleighContext},
};

use crate::{
    config::error::CrackersConfigError,
    reference_program::{ReferenceProgram, step::Step, valuation::MemoryValuation},
};

impl ReferenceProgram {
    pub fn try_load_parsed_pcode(
        sleigh: &SleighContext,
        pcode: &str,
    ) -> Result<Self, CrackersConfigError> {
        let ops = sleigh.parse_pcode_listing(pcode)?;
        let instrs: Vec<Step> = ops
            .into_iter()
            .enumerate()
            .map(|(i, o)| Step::from_instr(op_to_instr(o, sleigh.arch_info(), i as u64)))
            .collect();
        Ok(ReferenceProgram {
            initial_memory: MemoryValuation::default(),
            steps: instrs,
        })
    }
}

fn op_to_instr(op: PcodeOperation, arch_info: &SleighArchInfo, offset: u64) -> Instruction {
    Instruction {
        disassembly: Disassembly {
            mnemonic: format!("{}", op.display(arch_info)),
            args: "".to_string(),
        },
        ops: vec![op],
        length: 1,
        address: offset,
    }
}
