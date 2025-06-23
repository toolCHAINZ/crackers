pub mod builder;

use std::fmt::{Display, Formatter};

use jingle::modeling::{ModelingContext, State};
use jingle::sleigh::{ArchInfoProvider, GeneralizedVarNode};
use jingle::varnode::ResolvedVarnode;
use z3::Model;
use z3::ast::BV;

#[derive(Debug)]
pub struct AssignmentModel<'ctx, T: ModelingContext<'ctx>> {
    model: Model<'ctx>,
    pub gadgets: Vec<T>,
}

impl<'ctx, T: ModelingContext<'ctx>> AssignmentModel<'ctx, T> {
    pub fn new(model: Model<'ctx>, gadgets: Vec<T>) -> Self {
        Self { model, gadgets }
    }

    pub fn model(&self) -> &Model<'ctx> {
        &self.model
    }

    pub fn initial_state<'a>(&'a self) -> Option<&'a State<'ctx>> {
        self.gadgets.first().map(|f| f.get_original_state())
    }

    pub fn final_state<'a>(&'a self) -> Option<&'a State<'ctx>> {
        self.gadgets.last().map(|f| f.get_final_state())
    }

    pub fn read_original<'a>(&'a self, vn: GeneralizedVarNode) -> Option<BV<'ctx>> {
        self.initial_state().and_then(|f| f.read(vn).ok())
    }

    pub fn read_output<'a>(&'a self, vn: GeneralizedVarNode) -> Option<BV<'ctx>> {
        self.final_state().and_then(|f| f.read(vn).ok())
    }

    pub fn read_resolved<'a>(&'a self, vn: &ResolvedVarnode<'ctx>) -> Option<BV<'ctx>> {
        self.final_state().and_then(|f| f.read_resolved(vn).ok())
    }

    pub fn print_trace_of_reg(&self, reg: &str) {
        let r = self.final_state().unwrap().get_register(reg).unwrap();
        for gadget in &self.gadgets {
            let val = gadget.get_original_state().read_varnode(r).unwrap();
            println!("{} Before: {:?}", reg, self.model.eval(&val, false));
            let val = gadget.get_final_state().read_varnode(r).unwrap();
            println!("{} After: {:?}", reg, self.model.eval(&val, false));
        }
    }

    pub fn initial_reg<'a>(&'a self, reg: &str) -> Option<BV<'ctx>> {
        let r = self.final_state().unwrap().get_register(reg).unwrap();
        let val = self.gadgets[0]
            .get_original_state()
            .read_varnode(r)
            .unwrap();
        self.model.eval(&val, false)
    }
}

impl<'ctx, T: ModelingContext<'ctx> + Display> Display for AssignmentModel<'ctx, T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "Gadgets:\n")?;
        for block in &self.gadgets {
            writeln!(f, "{block}\n")?;
        }
        Ok(())
    }
}
