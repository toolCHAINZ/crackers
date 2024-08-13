use std::fs;

use clap::Parser;
use crackers::synthesis::DecisionResult;
use tracing::{event, Level};
use tracing_subscriber::FmtSubscriber;

use crate::config::CrackersGptConfig;
use crate::procedure::GptProcedure;

mod agents;
mod config;
mod evaluator;
mod procedure;
mod specification;

#[derive(Parser, Debug)]
struct Arguments {
    pub cfg_path: String,
}
#[tokio::main]
async fn main() {
    let args = Arguments::parse();
    let cfg_bytes = fs::read(args.cfg_path).unwrap();
    let s = String::from_utf8(cfg_bytes).unwrap();

    let p: CrackersGptConfig = toml_edit::de::from_str(&s).unwrap();
    let level = Level::from(p.meta.log_level);
    let sub = FmtSubscriber::builder().with_max_level(level).finish();
    tracing::subscriber::set_global_default(sub).unwrap();
    let mut gpt_proc = GptProcedure::new(p).unwrap();
    let rest = gpt_proc.run().await.unwrap();
    match rest {
        DecisionResult::AssignmentFound(a) => {
            event!(Level::INFO, "It worked!");
            println!("{}", a)
        }
        DecisionResult::Unsat(_) => {
            event!(Level::ERROR, "Didn't work :(")
        }
    }
}
