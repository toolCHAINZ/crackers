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

use crackers::bench::{BenchCommand, bench};
use crackers::config::CrackersConfig;
use crackers::config::constraint::{
    ConstraintConfig, PointerRange, PointerRangeConstraints,
    StateEqualityConstraint,
};
use crackers::config::sleigh::SleighConfig;
use crackers::config::specification::SpecificationConfig;
use crackers::gadget::library::builder::GadgetLibraryConfig;
use crackers::synthesis::DecisionResult;

#[derive(Parser, Debug)]
struct Arguments {
    pub cfg_path: String,
}

#[derive(Debug, Clone, Subcommand)]
pub enum CrackersCommands {
    New {
        config: Option<PathBuf>,
        #[arg(short, long)]
        library: Option<PathBuf>,
    },
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

    // Initialize basic logging for non-synth commands
    let result = match &config.command {
        CrackersCommands::New { config, library } => {
            init_basic_logging();
            event!(Level::INFO, "Creating new config file");
            new(
                config.clone().unwrap_or(PathBuf::from("./crackers.toml")),
                library.clone()
            )
        }
        CrackersCommands::Synth { config } => {
            // Synth initializes its own logging with config
            synthesize(config.clone())
        }
        CrackersCommands::Bench(cmd) => {
            init_basic_logging();
            event!(Level::INFO, "Running benchmark");
            bench(cmd.clone())
        }
    };

    if let Err(e) = result {
        event!(Level::ERROR, "Command failed: {}", e);
        std::process::exit(1);
    }
}

fn init_basic_logging() {
    let env_filter = EnvFilter::builder()
        .with_default_directive(LevelFilter::INFO.into())
        .from_env_lossy();
    let indicatif_layer = IndicatifLayer::new();
    let writer = indicatif_layer.get_stderr_writer();
    tracing_subscriber::registry()
        .with(env_filter)
        .with(indicatif_layer)
        .with(tracing_subscriber::fmt::layer().with_writer(writer))
        .init();
}

fn new(path: PathBuf, library: Option<PathBuf>) -> anyhow::Result<()> {
    event!(Level::INFO, "Generating new configuration file at: {}", path.display());

    let library_path = if let Some(lib_path) = library {
        event!(Level::INFO, "Using library path: {}", lib_path.display());
        lib_path.to_string_lossy().to_string()
    } else {
        event!(Level::DEBUG, "No library path provided, using empty default");
        String::new()
    };

    let config = CrackersConfig {
        meta: Default::default(),
        specification: SpecificationConfig::RawPcode(
            r"RDI = COPY 0xdeadbeef:8
              RSI = COPY 0x40:8
              RDX = COPY 0x7b:8
              RAX = COPY 0xfacefeed:8
              BRANCH 0xdeadbeef:8
              "
            .to_string(),
        ),
        library: GadgetLibraryConfig {
            max_gadget_length: 5,
            operation_blacklist: Default::default(),
            path: library_path,
            sample_size: None,
            base_address: None,
        },
        sleigh: SleighConfig {
            ghidra_path: "/Applications/ghidra".to_string(),
        },
        constraint: Some(ConstraintConfig {
            precondition: Some(StateEqualityConstraint {
                register: Some(HashMap::from([("RSP".to_string(), 0x8000_0000)])),
                memory: None,
                pointer: None,
            }),
            postcondition: Some(StateEqualityConstraint {
                register: None,
                memory: None,
                pointer: None,
            }),
            pointer: Some(PointerRangeConstraints {
                read: Some(vec![PointerRange {
                    max: 0x7fff_ff80,
                    min: 0x8000_0080,
                }]),
                write: Some(vec![PointerRange {
                    max: 0x7fff_ff80,
                    min: 0x8000_0080,
                }]),
            }),
        }),
        synthesis: Default::default(),
    };

    event!(Level::DEBUG, "Serializing configuration to TOML");
    let toml_content = to_string_pretty(&config)?;

    event!(Level::DEBUG, "Writing configuration to file");
    fs::write(&path, toml_content)?;

    event!(Level::INFO, "Successfully created configuration file at: {}", path.display());
    Ok(())
}

fn synthesize(config: PathBuf) -> anyhow::Result<()> {
    event!(Level::INFO, "Loading configuration from: {}", config.display());
    let cfg_bytes = fs::read(&config)?;
    let s = String::from_utf8(cfg_bytes)?;

    event!(Level::DEBUG, "Parsing configuration");
    let p: CrackersConfig = toml_edit::de::from_str(&s)?;
    let level = Level::from(p.meta.log_level);

    event!(Level::DEBUG, "Initializing logging with level: {:?}", level);
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

    event!(Level::INFO, "Resolving configuration parameters");
    let params = p.resolve()?;

    event!(Level::INFO, "Starting synthesis (combine_instructions: {})", params.combine_instructions);
    let result = match params.combine_instructions {
        true => {
            event!(Level::DEBUG, "Building combined synthesis");
            params.build_combined().and_then(|mut c| c.decide())
        }
        false => {
            event!(Level::DEBUG, "Building single synthesis");
            params.build_single().and_then(|mut c| c.decide())
        }
    };

    match result {
        Ok(res) => match res {
            DecisionResult::AssignmentFound(a) => {
                event!(Level::DEBUG, "Building assignment result");
                let a = a.build()?;
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
