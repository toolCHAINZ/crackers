use std::fs;

use clap::Parser;
use tracing::{event, Level};
use tracing_subscriber::FmtSubscriber;
use z3::{Config, Context};

use crackers::config::CrackersConfig;
use crackers::synthesis::DecisionResult;

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
    let level = Level::from(p.meta.log_level);
    let sub = FmtSubscriber::builder().with_max_level(level).finish();
    tracing::subscriber::set_global_default(sub).unwrap();
    let params = p.resolve().unwrap();
    match params.build(&z3) {
        Ok(mut p) => match p.decide() {
            Ok(res) => match res {
                DecisionResult::AssignmentFound(a) => {
                    event!(Level::INFO, "Synthesis successful :)");
                    println!("{}", a)
                }
                DecisionResult::Unsat(a) => {
                    event!(Level::ERROR, "Synthesis unsuccessful: {:?}", a);
                }
            },
            Err(e) => {
                event!(Level::ERROR, "Synthesis error: {}", e)
            }
        },
        Err(e) => {
            event!(Level::ERROR, "Error setting up synthesis: {}", e)
        }
    };
}
