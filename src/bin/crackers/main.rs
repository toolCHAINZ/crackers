use std::fs;

use clap::Parser;
use jingle::modeling::{ModelingContext, State};
use jingle::sleigh::{varnode, SpaceManager};
use jingle::varnode::{ResolvedIndirectVarNode, ResolvedVarnode};
use tracing::Level;
use tracing_subscriber::FmtSubscriber;
use z3::ast::{Ast, Bool, BV};
use z3::{Config, Context};

use crackers::error::CrackersError;
use crackers::synthesis::assignment_model::AssignmentModel;
use crackers::synthesis::DecisionResult;

use crate::config::CrackersConfig;

mod config;
mod error;

#[derive(Parser, Debug)]
struct Arguments {
    pub cfg_path: String,
}

fn main() {
    let cfg = Config::new();
    let z3 = Context::new(&cfg);
    let sub = FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .finish();
    tracing::subscriber::set_global_default(sub).unwrap();
    let args = Arguments::parse();
    let cfg_bytes = fs::read(&args.cfg_path).unwrap();
    let s = String::from_utf8(cfg_bytes).unwrap();
    let p: CrackersConfig = toml_edit::de::from_str(&s).unwrap();
    dbg!(&p);
    let mut p = p.resolve(&z3).unwrap();
    match p.decide().unwrap() {
        DecisionResult::ConflictsFound(_, _) => {}
        DecisionResult::AssignmentFound(a) => todo!(""),
        DecisionResult::Unsat => {}
    };
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
    let ptr = 0x7fffffffde00u64;
    for i in 0..(0x7ffffffff000 - ptr) / 4 {
        let addr = ptr.wrapping_add(i as u64 * 4);
        let varnode = varnode!(final_state, "ram"[addr]:4).unwrap();
        let display = varnode.display(final_state).unwrap();
        let read = final_state.read_varnode(&varnode).unwrap();
        let val = result.model().eval(&read, false).unwrap();
        println!("{} = {}", display, val);
    }
}
