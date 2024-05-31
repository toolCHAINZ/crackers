use std::fs;

use clap::Parser;
use tracing::Level;
use tracing_subscriber::FmtSubscriber;
use z3::{Config, Context};

use crackers::synthesis::{AssignmentSynthesis, DecisionResult};

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
    let args = Arguments::parse();
    let cfg_bytes = fs::read(args.cfg_path).unwrap();
    let s = String::from_utf8(cfg_bytes).unwrap();
    let p: CrackersConfig = toml_edit::de::from_str(&s).unwrap();
    let level = p
        .synthesis
        .as_ref()
        .map(|s| Level::from(s.log_level))
        .unwrap_or(Level::INFO);
    let sub = FmtSubscriber::builder().with_max_level(level).finish();
    tracing::subscriber::set_global_default(sub).unwrap();
    let mut p: AssignmentSynthesis = p.resolve(&z3).unwrap();
    let res = p.decide().unwrap();
    match res {
        DecisionResult::ConflictsFound(_, _) => {}
        DecisionResult::AssignmentFound(a) => println!("{}", a),
        DecisionResult::Unsat => {}
    }
}
