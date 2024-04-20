use std::fs;
use std::path::Path;

use crackers::error::CrackersError;
use elf::endian::AnyEndian;
use elf::ElfBytes;
use jingle::modeling::{ModeledInstruction, ModelingContext, State};
use jingle::sleigh::context::{Image, SleighContext, SleighContextBuilder};
use jingle::sleigh::{create_varnode, varnode, SpaceManager};
use jingle::varnode::{ResolvedIndirectVarNode, ResolvedVarnode};
use jingle::{JingleError, SleighTranslator};
use tracing::Level;
use tracing_subscriber::FmtSubscriber;
use z3::ast::{Ast, Bool, BV};
use z3::{Config, Context};

use crackers::synthesis::assignment_model::AssignmentModel;
use crackers::synthesis::builder::SynthesisBuilder;
use crackers::synthesis::builder::SynthesisSelectionStrategy::{OptimizeStrategy, SatStrategy};
use crackers::synthesis::DecisionResult;

#[allow(unused)]
const TEST_BYTES: [u8; 41] = [
    0xba, 0x60, 0xd0, 0x09, 0x00, 0x89, 0xd3, 0xb8, 0x2f, 0x62, 0x69, 0x6e, 0x89, 0x02, 0x83, 0xc3,
    0x04, 0xb8, 0x2f, 0x73, 0x68, 0x00, 0x89, 0x03, 0xba, 0x00, 0x00, 0x00, 0x00, 0xb9, 0x00, 0x00,
    0x00, 0x00, 0xb8, 0x0b, 0x00, 0x00, 0x00, 0xcd, 0x80,
];

fn main() {
    let cfg = Config::new();
    let z3 = Context::new(&cfg);
    let sub = FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .finish();
    tracing::subscriber::set_global_default(sub).unwrap();
    let builder =
        SleighContextBuilder::load_ghidra_installation(Path::new("/Applications/ghidra")).unwrap();
    let target_sleigh = builder
        .clone()
        .set_image(Image::from(TEST_BYTES.as_slice()))
        .build("x86:LE:64:default")
        .unwrap();
    let path = Path::new("bin/vuln");
    let data = fs::read(path).unwrap();
    let elf = ElfBytes::<AnyEndian>::minimal_parse(data.as_slice()).unwrap();

    let bin_sleigh = builder
        .set_image(Image::try_from(elf).unwrap())
        .build("x86:LE:64:default")
        .unwrap();
    let mut p = SynthesisBuilder::default()
        .max_gadget_length(4)
        .with_selection_strategy(OptimizeStrategy)
        .specification(target_sleigh.read(0, 11))
        .candidates_per_slot(100)
        .with_precondition(some_other_constraint)
        .with_precondition(initial_stack)
        .with_pointer_invariant(pointer_invariant)
        .build(&z3, &bin_sleigh)
        .unwrap();
    match p.decide().unwrap() {
        DecisionResult::ConflictsFound(_, _) => {}
        DecisionResult::AssignmentFound(a) => naive_alg(a),
        DecisionResult::Unsat => {}
    };
}

fn some_constraint<'a>(z3: &'a Context, state: &State<'a>) -> Result<Bool<'a>, CrackersError> {
    let data = state.read_varnode(&state.varnode("register", 0, 40).unwrap())?;
    let constraint = data._eq(&BV::from_u64(z3, 0, data.get_size()));
    Ok(constraint)
}

fn some_other_constraint<'a>(
    z3: &'a Context,
    state: &State<'a>,
) -> Result<Bool<'a>, CrackersError> {
    let data = state.read_varnode(&state.varnode("register", 0, 0x20).unwrap())?;
    let constraint = data._eq(&BV::from_u64(z3, 0, data.get_size()));
    Ok(constraint)
}

fn initial_stack<'a>(
    z3: &'a Context,
    state: &State<'a>,
) -> Result<Bool<'a>, CrackersError> {
    let data = state.read_varnode(&state.varnode("register", 0x20, 0x8).unwrap())?;
    let constraint = data._eq(&BV::from_u64(z3, 0x4444_0000, data.get_size()));
    Ok(constraint)
}

fn pointer_invariant<'a>(
    z3: &'a Context,
    input: &ResolvedIndirectVarNode<'a>,
) -> Result<Option<Bool<'a>>, CrackersError> {
    let constraint = input.pointer.bvuge(&BV::from_u64(z3, 0x4444_0000, input.pointer.get_size()));
    let constraint2 = input.pointer.bvule(&BV::from_u64(z3, 0x4444_0400, input.pointer.get_size()));
    Ok(Some(Bool::and(z3, &[constraint, constraint2])))
}

fn get_target_instructions<'ctx>(
    sleigh: &'ctx SleighContext,
    z3: &'ctx Context,
) -> Result<Vec<ModeledInstruction<'ctx>>, JingleError> {
    let modeler = SleighTranslator::new(sleigh, z3);
    let mut instrs = vec![];
    let mut i = 0;
    while i < TEST_BYTES.len() {
        let model = modeler.model_instruction_at(i as u64)?;
        i += model.instr.length;
        instrs.push(model);
    }
    Ok(instrs)
}

fn naive_alg(result: AssignmentModel) {
    for b in &result.gadgets {
        for x in &b.instructions {
            println!("{:x} {}", x.address, x.disassembly);
        }
        println!();
    }
    println!("inputs:");

    for x in result
        .gadgets
        .as_slice()
        .get_inputs()
        .iter()
        .filter(|v| result.gadgets.as_slice().should_varnode_constrain(v))
    {
        let bv = result.read_resolved(x).unwrap();
        match x {
            ResolvedVarnode::Direct(_) => println!(
                "{} = {}",
                x.display(result.initial_state().unwrap()).unwrap(),
                result.model().eval(&bv, false).unwrap()
            ),
            ResolvedVarnode::Indirect(i) => {
                let ptr = result.model().eval(&i.pointer, false).unwrap().simplify();
                println!(
                    "{}[{}] = {}",
                    i.pointer_space_idx,
                    ptr,
                    result.model().eval(&bv, false).unwrap()
                )
            }
        }
    }
    println!("outputs:");
    for x in result
        .gadgets
        .as_slice()
        .get_outputs()
        .iter()
        .filter(|v| result.gadgets.as_slice().should_varnode_constrain(v))
    {
        let bv = result.final_state().unwrap().read_resolved(x).unwrap();
        println!(
            "{} = {}",
            x.display(result.final_state().unwrap()).unwrap(),
            result.model().eval(&bv, false).unwrap()
        )
    }
    println!("stack");
    let final_state = result.final_state().unwrap();
    let initial_state = result.initial_state().unwrap();
    let reg = varnode!(result.initial_state().unwrap(), "register"[0x20]:8).unwrap();
    let stack_reg = final_state.read_varnode(&reg).unwrap().simplify();
    let ptr = result
        .model()
        .eval(&stack_reg, false)
        .unwrap()
        .as_u64()
        .unwrap();
    for i in -32i32..0i32 {
        let addr = ptr.wrapping_add((i as u64).wrapping_mul(8));
        let varnode = varnode!(final_state, "ram"[addr]:8).unwrap();
        let display = varnode.display(final_state).unwrap();
        let read = result
            .initial_state()
            .unwrap()
            .read_varnode(&varnode)
            .unwrap();
        let val = result.model().eval(&read, false).unwrap();
        println!("{} = {}", display, val);
    }
    // buffer stuff
    println!("buffer");
    let ptr = 0x9d060u64;
    for i in 0i32..8i32 {
        let addr = ptr.wrapping_add(i as u64 * 4);
        let varnode = varnode!(final_state, "ram"[addr]:4).unwrap();
        let display = varnode.display(final_state).unwrap();
        let read = final_state.read_varnode(&varnode).unwrap();
        let val = result.model().eval(&read, false).unwrap();
        println!("{} = {}", display, val);
    }
    println!("buffer");
    let ptr = 0x9d060u64;
    for i in 0i32..8i32 {
        let addr = ptr.wrapping_add(i as u64 * 4);
        let varnode = varnode!(final_state, "ram"[addr]:4).unwrap();
        let display = varnode.display(final_state).unwrap();
        let read = initial_state.read_varnode(&varnode).unwrap();
        let val = result.model().eval(&read, false).unwrap();
        println!("{} = {}", display, val);
    }
}
