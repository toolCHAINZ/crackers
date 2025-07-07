use crate::error::CrackersError;
use crate::gadget::Gadget;
use crate::gadget::library::GadgetLibrary;
use crate::reference_program::ReferenceProgram;
use crate::synthesis::assignment_model::AssignmentModel;
use crate::synthesis::builder::{StateConstraintGenerator, TransitionConstraintGenerator};
use crate::synthesis::pcode_theory::pcode_assignment::PcodeAssignment;
use jingle::JingleContext;
use jingle::modeling::{ModeledBlock, ModeledInstruction};
use jingle::sleigh::{ArchInfoProvider, SpaceInfo, VarNode};
use std::fmt::{Debug, Formatter};
use std::sync::Arc;
use z3::{Context, Solver};

// todo: this should probably just be a struct in Jingle and we can stop with this trait nonsense
#[derive(Clone, Debug)]
pub struct ArchInfo {
    spaces: Vec<SpaceInfo>,
    default_code_space_index: usize,
    registers: Vec<(VarNode, String)>,
}

impl From<&GadgetLibrary> for ArchInfo {
    fn from(value: &GadgetLibrary) -> Self {
        Self {
            default_code_space_index: value.get_code_space_idx(),
            registers: value
                .get_registers()
                .map(|(a, b)| (a.clone(), b.to_string()))
                .collect(),
            spaces: value.get_all_space_info().cloned().collect(),
        }
    }
}

impl ArchInfoProvider for ArchInfo {
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
        self.registers.iter().find(|f| f.1 == name).map(|f| &f.0)
    }

    fn get_register_name(&self, location: &VarNode) -> Option<&str> {
        self.registers
            .iter()
            .find(|f| f.0 == *location)
            .map(|f| f.1.as_str())
    }

    fn get_registers(&self) -> impl Iterator<Item = (&VarNode, &str)> {
        self.registers.iter().map(|(f, v)| (f, v.as_str()))
    }
}

#[derive(Clone)]
pub struct AssignmentModelBuilder {
    pub templates: ReferenceProgram,
    pub gadgets: Vec<Gadget>,
    pub preconditions: Vec<Arc<StateConstraintGenerator>>,
    pub postconditions: Vec<Arc<StateConstraintGenerator>>,
    pub pointer_invariants: Vec<Arc<TransitionConstraintGenerator>>,
    pub arch_info: ArchInfo,
}

impl Debug for AssignmentModelBuilder {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AssignmentModelBuilder")
            .field("templates", &self.templates)
            .field("gadgets", &self.gadgets)
            .field("arch_info", &self.arch_info)
            .finish()
    }
}

impl AssignmentModelBuilder {
    fn make_pcode_model<'ctx>(
        &self,
        jingle: &JingleContext<'ctx>,
    ) -> Result<PcodeAssignment<'ctx>, CrackersError> {
        let modeled_spec: Result<Vec<ModeledInstruction<'ctx>>, _> = self
            .templates
            .steps()
            .iter()
            .map(|i| i.model(jingle).map_err(CrackersError::from))
            .collect();
        let modeled_spec = modeled_spec?;
        let modeled_gadgets: Result<_, _> = self
            .gadgets
            .iter()
            .cloned()
            .map(|i| i.model(jingle))
            .collect();
        let modeled_gadgets = modeled_gadgets?;
        Ok(PcodeAssignment::new(
            self.templates.initial_memory().clone(),
            modeled_spec,
            modeled_gadgets,
            self.preconditions.clone(),
            self.postconditions.clone(),
            self.pointer_invariants.clone(),
        ))
    }
    pub fn build<'ctx>(
        &self,
        z3: &'ctx Context,
    ) -> Result<AssignmentModel<'ctx, ModeledBlock<'ctx>>, CrackersError> {
        let jingle = JingleContext::new(z3, &self.arch_info);

        let pcode_model = self.make_pcode_model(&jingle)?;
        let s = Solver::new(jingle.z3);
        pcode_model.check(&jingle, &s)
    }
}
