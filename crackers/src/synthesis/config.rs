use jingle::modeling::State;
use jingle::varnode::ResolvedIndirectVarNode;
use z3::ast::Bool;
use z3::Context;

use crate::error::CrackersError;

#[derive(Copy, Clone, Debug)]
pub enum SynthesisSelectionStrategy{
    SatStrategy,
    OptimizeStrategy
}


pub struct SynthesisBuilder<'ctx>{
    selection_strategy: SynthesisSelectionStrategy,
    max_gadget_length: usize,
    max_gadgets_per_slot: usize,
    preconditions: Vec<Box<dyn Fn(&'ctx Context, &State<'ctx>) -> Result<Vec<Bool<'ctx>>, CrackersError>>>,
    postconditions: Vec<Box<dyn Fn(&'ctx Context, &State<'ctx>) -> Result<Vec<Bool<'ctx>>, CrackersError>>>,
    invariants: Vec<Box<dyn Fn(&'ctx Context, &ResolvedIndirectVarNode<'ctx>) -> Result<Vec<Bool<'ctx>>, CrackersError>>>,
}