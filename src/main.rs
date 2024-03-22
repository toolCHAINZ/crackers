use std::fs;
use std::path::Path;

use elf::endian::AnyEndian;
use elf::ElfBytes;
use jingle::modeling::{ModeledBlock, ModeledInstruction, ModelingContext};
use jingle::sleigh::context::{Image, SleighContext, SleighContextBuilder};
use jingle::sleigh::{create_varnode, varnode};
use jingle::varnode::ResolvedVarnode;
use jingle::{JingleError, SleighTranslator};
use tracing::Level;
use tracing_subscriber::FmtSubscriber;
use z3::ast::Ast;
use z3::{Config, Context};

use crackers::gadget::GadgetLibrary;
use crackers::synthesis::assignment_problem::AssignmentProblem;
use crackers::synthesis::greedy::GreedySynthesizer;

#[allow(unused)]
const TEST_BYTES: [u8; 37] = [
    0xba, 0x60, 0xd0, 0x09, 0x00, 0x89, 0xd3, 0xb8, 0x2f, 0x62, 0x69, 0x6e, 0x89, 0x02, 0xba, 0x64,
    0xd0, 0x09, 0x00, 0xb8, 0x2f, 0x73, 0x68, 0x00, 0x89, 0x02, 0x31, 0xd2, 0x31, 0xd2, 0xb8, 0x0b,
    0x00, 0x00, 0x00, 0xcd, 0x80,
];

fn main() {
    let cfg = Config::new();
    let z3 = Context::new(&cfg);
    let sub = FmtSubscriber::builder()
        .with_max_level(Level::DEBUG)
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

    let targets = get_target_instructions(&target_sleigh, &z3).unwrap();
    let library = GadgetLibrary::build_from_image(&bin_sleigh).unwrap();
    //library.write_to_file(&"gadgets.bin").unwrap();
    //naive_alg(&z3, targets, library);
    let mut p = AssignmentProblem::new(&z3, target_sleigh.read(0, 7).collect(), library);
    p.decide().unwrap();
}

fn get_target_instructions<'ctx>(
    sleigh: &'ctx SleighContext,
    z3: &'ctx Context,
) -> Result<Vec<ModeledInstruction<'ctx>>, JingleError> {
    let modeler = SleighTranslator::new(&sleigh, &z3);
    let mut instrs = vec![];
    let mut i = 0;
    while i < TEST_BYTES.len() {
        let model = modeler.model_instruction_at(i as u64)?;
        i += model.instr.length;
        instrs.push(model);
    }
    Ok(instrs)
}

fn naive_alg(z3: &Context, targets: Vec<ModeledInstruction>, gadgets: GadgetLibrary) {
    let spec = ModeledBlock::try_from(targets.as_slice()).unwrap();
    let greedy = GreedySynthesizer::new(z3, targets.clone(), gadgets);
    let result = greedy.decide().unwrap();
    result.solver.check_assumptions(&[
        result.reaches(&spec).unwrap(),
        result.refines(&spec).unwrap(),
    ]);
    let model = result.solver.get_model().unwrap();
    println!("inputs:");

    for x in result
        .get_inputs()
        .iter()
        .filter(|v| result.should_varnode_constrain(v))
    {
        let bv = result.get_original_state().read_resolved(x).unwrap();
        match x {
            ResolvedVarnode::Direct(_) => println!(
                "{} = {}",
                x.display(result.get_original_state()).unwrap(),
                model.eval(&bv, false).unwrap()
            ),
            ResolvedVarnode::Indirect(i) => {
                let ptr = model.eval(&i.pointer, false).unwrap().simplify();
                println!(
                    "{}[{}] = {}",
                    i.pointer_space_idx,
                    ptr,
                    model.eval(&bv, false).unwrap()
                )
            }
        }
    }
    println!("outputs:");
    for x in spec
        .get_outputs()
        .iter()
        .filter(|v| result.should_varnode_constrain(v))
    {
        let bv = result.get_final_state().read_resolved(x).unwrap();
        println!(
            "{} = {}",
            x.display(result.get_final_state()).unwrap(),
            model.eval(&bv, false).unwrap()
        )
    }
    println!("stack");
    let final_state = result.get_final_state();
    let reg = varnode!(result.get_original_state(), "register"[0x20]:8).unwrap();
    let stack_reg = final_state.read_varnode(&reg).unwrap().simplify();
    let ptr = model.eval(&stack_reg, false).unwrap().as_u64().unwrap();
    for i in -32i32..0i32 {
        let addr = ptr.wrapping_add((i as u64).wrapping_mul(8));
        let varnode = varnode!(final_state, "ram"[addr]:8).unwrap();
        let display = varnode.display(final_state).unwrap();
        let read = result.get_original_state().read_varnode(&varnode).unwrap();
        let val = model.eval(&read, false).unwrap();
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
        let val = model.eval(&read, false).unwrap();
        println!("{} = {}", display, val);
    }
    std::fs::write("../../smt.txt", result.solver.to_smt2()).unwrap();
}
