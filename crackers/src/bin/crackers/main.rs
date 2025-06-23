use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

use clap::{Parser, Subcommand};
use toml_edit::ser::to_string_pretty;
use tracing::{Level, event};
use tracing_indicatif::IndicatifLayer;
use tracing_subscriber::EnvFilter;
use tracing_subscriber::filter::LevelFilter;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use z3::{Config, Context};

use crackers::bench::{BenchCommand, bench};
use crackers::config::CrackersConfig;
use crackers::config::constraint::{
    ConstraintConfig, MemoryEqualityConstraint, PointerRange, PointerRangeConstraints,
    StateEqualityConstraint,
};
use crackers::config::sleigh::SleighConfig;
use crackers::config::specification::SpecificationConfig;
use crackers::synthesis::DecisionResult;

#[derive(Parser, Debug)]
struct Arguments {
    pub cfg_path: String,
}

#[derive(Debug, Clone, Subcommand)]
pub enum CrackersCommands {
    New { config: Option<PathBuf> },
    Synth { config: PathBuf },
    Bench(BenchCommand),
}

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct CrackersParams {
    #[command(subcommand)]
    command: CrackersCommands,
}

fn main() {
    let config = CrackersParams::parse();
    match config.command {
        CrackersCommands::New { config } => {
            new(config.unwrap_or(PathBuf::from("./crackers.toml"))).unwrap()
        }
        CrackersCommands::Synth { config } => synthesize(config).unwrap(),
        CrackersCommands::Bench(cmd) => bench(cmd).unwrap(),
    }
}

fn new(path: PathBuf) -> anyhow::Result<()> {
    let config = CrackersConfig {
        meta: Default::default(),
        specification: SpecificationConfig {
            path: "spec.o".to_string(),
            max_instructions: 1,
            base_address: None,
        },
        library: Default::default(),
        sleigh: SleighConfig {
            ghidra_path: "/Applications/ghidra".to_string(),
        },
        constraint: Some(ConstraintConfig {
            precondition: Some(StateEqualityConstraint {
                register: Some(HashMap::from([("ABC".to_string(), 123)])),
                memory: Some(MemoryEqualityConstraint {
                    size: 4,
                    space: "ram".to_string(),
                    address: 0x80_0000,
                    value: 0,
                }),
                pointer: Some(HashMap::from([("DEF".to_string(), "hello".to_string())])),
            }),
            postcondition: Some(StateEqualityConstraint {
                register: Some(HashMap::from([("ABC".to_string(), 456)])),
                memory: Some(MemoryEqualityConstraint {
                    size: 4,
                    space: "ram".to_string(),
                    address: 0x80_0000,
                    value: 0,
                }),
                pointer: Some(HashMap::from([("DEF".to_string(), "goodbye".to_string())])),
            }),
            pointer: Some(PointerRangeConstraints {
                read: Some(vec![PointerRange {
                    max: 0xf000_0000,
                    min: 0xc000_0000,
                }]),
                write: Some(vec![PointerRange {
                    max: 0xf000_0000,
                    min: 0xc000_0000,
                }]),
            }),
        }),
        synthesis: Default::default(),
    };

    fs::write(path, to_string_pretty(&config)?)?;
    Ok(())
}

fn synthesize(config: PathBuf) -> anyhow::Result<()> {
    let cfg = Config::new();
    let z3 = Context::new(&cfg);
    let cfg_bytes = fs::read(config)?;
    let s = String::from_utf8(cfg_bytes)?;
    let p: CrackersConfig = toml_edit::de::from_str(&s)?;
    let level = Level::from(p.meta.log_level);
    let env_filter = EnvFilter::builder()
        .with_default_directive(LevelFilter::ERROR.into())
        .from_env()?
        .add_directive(format!("crackers={level}").parse()?);
    let indicatif_layer = IndicatifLayer::new();
    let writer = indicatif_layer.get_stderr_writer();
    tracing_subscriber::registry()
        .with(env_filter)
        .with(indicatif_layer)
        .with(tracing_subscriber::fmt::layer().with_writer(writer))
        .init();
    let params = p.resolve()?;
    let result = match params.combine_instructions {
        true => params.build_combined(&z3).and_then(|mut c| c.decide()),
        false => params.build_single(&z3).and_then(|mut c| c.decide()),
    };
    match result {
        Ok(res) => match res {
            DecisionResult::AssignmentFound(a) => {
                let z3 = Context::new(&Config::new());
                let a = a.build(&z3)?;
                event!(Level::INFO, "Synthesis successful :)");
                event!(Level::INFO, "{}", a)
            }
            DecisionResult::Unsat(a) => {
                event!(Level::ERROR, "Synthesis unsuccessful: {:?}", a);
            }
        },
        Err(e) => {
            event!(Level::ERROR, "Synthesis error: {}", e)
        }
    }
    Ok(())
}
