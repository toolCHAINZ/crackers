pub mod builder;

use std::fmt::{Display, Formatter};

use jingle::modeling::{ModelingContext, State};
use jingle::sleigh::{GeneralizedVarNode, SleighArchInfo};
use jingle::varnode::ResolvedVarnode;
use z3::ast::BV;
use z3::{Context, Model, Translate};

#[derive(Debug)]
pub struct AssignmentModel<T: ModelingContext> {
    model: Model,
    pub gadgets: Vec<T>,
    pub arch_info: SleighArchInfo,
}

impl<T: ModelingContext> AssignmentModel<T> {
    pub fn new(model: Model, gadgets: Vec<T>, arch_info: SleighArchInfo) -> Self {
        Self {
            model,
            gadgets,
            arch_info,
        }
    }

    pub fn model(&self) -> &Model {
        &self.model
    }

    pub fn initial_state(&self) -> Option<&State> {
        self.gadgets.first().map(|f| f.get_original_state())
    }

    pub fn final_state(&self) -> Option<&State> {
        self.gadgets.last().map(|f| f.get_final_state())
    }

    pub fn read_original(&self, vn: GeneralizedVarNode) -> Option<BV> {
        self.initial_state().and_then(|f| f.read(vn).ok())
    }

    pub fn read_output(&self, vn: GeneralizedVarNode) -> Option<BV> {
        self.final_state().and_then(|f| f.read(vn).ok())
    }

    pub fn read_resolved(&self, vn: &ResolvedVarnode) -> Option<BV> {
        self.final_state().and_then(|f| f.read_resolved(vn).ok())
    }

    pub fn print_trace_of_reg(&self, reg: &str) {
        let r = self
            .final_state()
            .unwrap()
            .arch_info()
            .register(reg)
            .unwrap();
        for gadget in &self.gadgets {
            let val = gadget.get_original_state().read_varnode(r).unwrap();
            println!("{} Before: {:?}", reg, self.model.eval(&val, false));
            let val = gadget.get_final_state().read_varnode(r).unwrap();
            println!("{} After: {:?}", reg, self.model.eval(&val, false));
        }
    }

    pub fn initial_reg(&self, reg: &str) -> Option<BV> {
        let r = self
            .final_state()
            .unwrap()
            .arch_info()
            .register(reg)
            .unwrap();
        let val = self.gadgets[0]
            .get_original_state()
            .read_varnode(r)
            .unwrap();
        self.model.eval(&val, false)
    }

    pub fn inputs(&self) -> impl Iterator<Item = ResolvedVarnode> {
        self.gadgets.iter().flat_map(|gadget| gadget.get_inputs())
    }

    pub fn outputs(&self) -> impl Iterator<Item = ResolvedVarnode> {
        self.gadgets.iter().flat_map(|gadget| gadget.get_outputs())
    }
}

impl<T: ModelingContext + Display> Display for AssignmentModel<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "Gadgets:\n")?;
        for block in &self.gadgets {
            writeln!(f, "{block}\n")?;
        }
        Ok(())
    }
}

unsafe impl<T: ModelingContext + Translate> Translate for AssignmentModel<T> {
    fn translate(&self, dest: &Context) -> Self {
        Self {
            model: self.model.translate(dest),
            gadgets: self.gadgets.iter().map(|g| g.translate(dest)).collect(),
            arch_info: self.arch_info.clone(),
        }
    }
}
