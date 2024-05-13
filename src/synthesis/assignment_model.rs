use std::fmt::{Display, Formatter};

use jingle::modeling::{ModelingContext, State};
use jingle::sleigh::GeneralizedVarNode;
use jingle::varnode::ResolvedVarnode;
use z3::ast::BV;
use z3::Model;

#[derive(Debug)]
pub struct AssignmentModel<'ctx, T: ModelingContext<'ctx>> {
    model: Model<'ctx>,
    pub gadgets: Vec<T>,
}

impl<'ctx, T: ModelingContext<'ctx>> AssignmentModel<'ctx, T> {
    pub fn generate(
        model: Model<'ctx>,
        gadgets: Vec<T>,
    ) -> Self {
        Self {
            model,
            gadgets,
        }
    }

    pub fn model(&self) -> &Model<'ctx> {
        &self.model
    }

    pub fn initial_state(&'ctx self) -> Option<&'ctx State<'ctx>> {
        self.gadgets.first().map(|f| f.get_original_state())
    }

    pub fn final_state(&'ctx self) -> Option<&'ctx State<'ctx>> {
        self.gadgets.last().map(|f| f.get_final_state())
    }

    pub fn read_original(&'ctx self, vn: GeneralizedVarNode) -> Option<BV<'ctx>> {
        self.initial_state().and_then(|f| f.read(vn).ok())
    }

    pub fn read_output(&'ctx self, vn: GeneralizedVarNode) -> Option<BV<'ctx>> {
        self.final_state().and_then(|f| f.read(vn).ok())
    }

    pub fn read_resolved(&'ctx self, vn: &ResolvedVarnode<'ctx>) -> Option<BV<'ctx>> {
        self.final_state().and_then(|f| f.read_resolved(vn).ok())
    }
}

impl<'ctx, T: ModelingContext<'ctx> + Display> Display for AssignmentModel<'ctx, T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "Gadgets:\n")?;
        for block in &self.gadgets {
            writeln!(f, "{}\n", block)?;
        }
        Ok(())
    }
}
