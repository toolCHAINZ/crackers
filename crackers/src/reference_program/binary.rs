use super::ReferenceProgram;
use crate::config::error::CrackersConfigError;
use crate::config::error::CrackersConfigError::{
    SpecMissingStartSymbol, SpecMissingTextSection, UnrecognizedArchitecture,
};
use crate::config::sleigh::SleighConfig;
use crate::config::specification::BinaryFileSpecification;
use crate::error::CrackersError;
use crate::error::CrackersError::ModelGenerationError;
use crate::reference_program::MemoryValuation;
use crate::reference_program::step::Step;
use crate::synthesis::partition_iterator::Partition;
use jingle::analysis::varnode::VarNodeSet;
use jingle::modeling::{ModeledInstruction, ModelingContext};
use jingle::sleigh::context::image::gimli::map_gimli_architecture;
use jingle::sleigh::context::loaded::LoadedSleighContext;
use jingle::sleigh::{GeneralizedVarNode, Instruction, OpCode, SleighArchInfo, VarNode};
use jingle::varnode::ResolvedVarnode;
use object::{File, Object, ObjectSymbol};
use std::borrow::Borrow;
use std::cmp::min;
use std::collections::{HashMap, HashSet};
use std::fs;
use z3::{SatResult, Solver};

impl ReferenceProgram {
    pub fn try_load_binary(
        spec: &BinaryFileSpecification,
        sleigh_config: &SleighConfig,
        blacklist: &HashSet<OpCode>,
    ) -> Result<Self, CrackersConfigError> {
        let mut blacklist = blacklist.clone();
        // We _do_ want to allow these in the reference program
        blacklist.remove(&OpCode::CPUI_BRANCH);
        blacklist.remove(&OpCode::CPUI_CALL);

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
        let instrs: Vec<_> = sleigh
            .read_until_branch(addr, spec.max_instructions)
            .collect();
        for op in instrs.iter().flat_map(|i| i.ops.iter()) {
            if blacklist.contains(&op.opcode()) {
                return Err(CrackersConfigError::IllegalPcodeOperation(op.opcode()));
            }
        }

        let steps: Vec<_> = instrs.into_iter().map(Step::from_instr).collect();
        let mut ref_program = Self {
            steps,
            initial_memory: Default::default(),
        };

        ref_program.calc_initial_memory_valuation(sleigh);
        Ok(ref_program)
    }

    fn calc_initial_memory_valuation(&mut self, image: LoadedSleighContext<'_>) {
        let steps = &self.steps;
        let mut covering_set = VarNodeSet::default();
        // initial direct pass
        for x in steps
            .iter()
            .flat_map(|step| step.instructions())
            .flat_map(|i| i.ops.clone())
        {
            for vn in x.inputs() {
                if let GeneralizedVarNode::Direct(vn) = vn {
                    covering_set.insert(&vn);
                }
            }
        }

        // now load indirect until it stablizes
        let mut stablized = false;
        while !stablized {
            stablized = true;
            for x in steps
                .iter()
                .flat_map(|step| step.instructions())
                .flat_map(|i| i.ops.clone())
            {
                for vn in x.inputs() {
                    if let GeneralizedVarNode::Indirect(vn) = vn {
                        if covering_set.covers(&vn.pointer_location) {
                            let pointer_offset_bytes_le = if image.spaces()
                                [image.arch_info().default_code_space_index()]
                            .isBigEndian()
                            {
                                image.read_bytes(&vn.pointer_location).map(|mut f| {
                                    f.reverse();
                                    f
                                })
                            } else {
                                image.read_bytes(&vn.pointer_location)
                            };
                            if let Some(pointer_offset_bytes_le) = pointer_offset_bytes_le {
                                let mut buffer: [u8; 8] = [0; 8];
                                let max = min(buffer.len(), pointer_offset_bytes_le.len());
                                buffer[0..max].copy_from_slice(&pointer_offset_bytes_le[0..max]);
                                let ptr = u64::from_le_bytes(buffer);
                                let new_vn = VarNode {
                                    size: vn.access_size_bytes,
                                    space_index: vn.pointer_space_index,
                                    offset: ptr,
                                };
                                covering_set.insert(&new_vn);
                                stablized = false;
                            }
                        }
                    }
                }
            }
        }

        self.initialize_valuation(&covering_set, &image);
        let extended_constraints = self
            .get_extended_constraints_from_indirect(image.arch_info())
            .unwrap();
        self.initialize_valuation(&extended_constraints, &image);
    }

    fn initialize_valuation(&mut self, covering_set: &VarNodeSet, image: &LoadedSleighContext<'_>) {
        let mut valuation = HashMap::new();
        for x in covering_set.varnodes() {
            if let Some(b) = image.read_bytes(&x) {
                valuation.insert(x, b);
            }
        }
        self.initial_memory = MemoryValuation(valuation);
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

    fn get_extended_constraints_from_indirect<T: Borrow<SleighArchInfo>>(
        &self,
        ctx: T,
    ) -> Result<VarNodeSet, CrackersError> {
        let i: Vec<_> = self.instructions().cloned().collect();
        let i: Instruction = i.as_slice().try_into().unwrap();
        let modeled_instr = ModeledInstruction::new(i, ctx)?;
        let init_constraint = self.initial_memory.to_constraint();
        let constraint = init_constraint(modeled_instr.get_original_state())?;
        let solver = Solver::new();
        let mut vn_set = VarNodeSet::default();
        solver.assert(&constraint);
        match solver.check() {
            SatResult::Sat => {
                let model = solver.get_model().ok_or(ModelGenerationError)?;
                for x in modeled_instr.get_inputs() {
                    match x {
                        ResolvedVarnode::Direct(vn) => {
                            vn_set.insert(&vn);
                        }
                        ResolvedVarnode::Indirect(ivn) => {
                            vn_set.insert(&ivn.pointer_location);
                            if let Some(res) =
                                model.eval(&ivn.pointer, true).and_then(|f| f.as_u64())
                            {
                                let new_vn = VarNode {
                                    size: ivn.access_size_bytes,
                                    space_index: ivn.pointer_space_idx,
                                    offset: res,
                                };
                                vn_set.insert(&new_vn);
                            }
                        }
                    }
                }
            }
            _ => {
                return Err(CrackersError::ModelGenerationError);
            }
        }
        Ok(vn_set)
    }

    pub fn len(&self) -> usize {
        self.steps.len()
    }

    fn instructions(&self) -> impl Iterator<Item = &Instruction> {
        self.steps.iter().flat_map(|step| step.instructions())
    }

    pub fn is_empty(&self) -> bool {
        self.steps.is_empty()
    }

    pub fn steps(&self) -> &[Step] {
        &self.steps
    }

    pub fn initial_memory(&self) -> &MemoryValuation {
        &self.initial_memory
    }
}
