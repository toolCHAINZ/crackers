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
    ConstraintConfig, PointerRange, PointerRangeConstraints, StateEqualityConstraint,
};
use crackers::config::sleigh::SleighConfig;
use crackers::config::specification::SpecificationConfig;
use crackers::gadget::library::builder::GadgetLibraryConfig;
use crackers::synthesis::DecisionResult;
use crackers::synthesis::assignment_model::AssignmentModel;
use jingle::display::JingleDisplayable;
use jingle::modeling::ModelingContext;
use jingle::sleigh::SpaceType;
use jingle::varnode::ResolvedVarnode;
use std::collections::BTreeSet;

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
    Synth {
        config: PathBuf,
    },
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
                library.clone(),
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
    event!(
        Level::INFO,
        "Generating new configuration file at: {}",
        path.display()
    );

    let library_path = if let Some(lib_path) = library {
        event!(Level::INFO, "Using library path: {}", lib_path.display());
        lib_path.to_string_lossy().to_string()
    } else {
        event!(
            Level::DEBUG,
            "No library path provided, using empty default"
        );
        String::new()
    };

    let config = CrackersConfig {
        meta: Default::default(),
        specification: SpecificationConfig::RawPcode(
            r"
              EDI = COPY 0xdeadbeef:4
              ESI = COPY 0x40:4
              EDX = COPY 0x7b:4
              EAX = COPY 0xfacefeed:4
              BRANCH 0xdeadbeef:1
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
                register: Some(HashMap::from([("ESP".to_string(), 0x8000_0000)])),
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
                    min: 0x7fff_ff80,
                    max: 0x8000_0080,
                }]),
                write: Some(vec![PointerRange {
                    min: 0x7fff_ff80,
                    max: 0x8000_0080,
                }]),
            }),
        }),
        synthesis: Default::default(),
    };

    event!(Level::DEBUG, "Serializing configuration to TOML");
    let toml_content = to_string_pretty(&config)?;

    event!(Level::DEBUG, "Writing configuration to file");
    fs::write(&path, toml_content)?;

    event!(
        Level::INFO,
        "Successfully created configuration file at: {}",
        path.display()
    );
    Ok(())
}

fn synthesize(config: PathBuf) -> anyhow::Result<()> {
    event!(
        Level::INFO,
        "Loading configuration from: {}",
        config.display()
    );
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

    event!(
        Level::INFO,
        "Starting synthesis (combine_instructions: {})",
        params.combine_instructions
    );
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
                event!(Level::INFO, "{}", a);
                print_assignment_details(&a);
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

fn format_resolved_varnode<T: ModelingContext>(
    vn: &ResolvedVarnode,
    model: &AssignmentModel<T>,
) -> String {
    match vn {
        ResolvedVarnode::Direct(d) => {
            format!("{}", d.display(&model.arch_info))
        }
        ResolvedVarnode::Indirect(i) => {
            let space_name = model
                .arch_info
                .get_space(i.pointer_space_idx)
                .map(|s| s.name.as_str())
                .unwrap_or("unknown");
            let access_size = i.access_size_bytes;
            if let Some(pointer_value) = model.model().eval(&i.pointer, true) {
                format!("{space_name}[{pointer_value}]:{access_size:x}")
            } else {
                format!("{space_name}[?]:{access_size:x}")
            }
        }
    }
}

fn print_assignment_details<T: ModelingContext>(model: &AssignmentModel<T>) {
    println!("\n========== Assignment Model Details ==========\n");

    println!(
        "Note: models produced through the CLI only represent the transitions within a chain."
    );
    println!("They do not constrain the system state to redirect execution to the chain.");
    println!(
        "If you need this, consider using the rust or python API to encode your constraint.\n"
    );
    
    // Collect all inputs and their valuations
    println!("--- Inputs (Locations Read) ---");
    let mut inputs_set: BTreeSet<String> = BTreeSet::new();

    for (gadget_idx, gadget) in model.gadgets.iter().enumerate() {
        println!("Gadget {}:", gadget_idx);
        for input in gadget.get_inputs() {
            // Filter out unique space variables (keep only IPTR_PROCESSOR)
            let should_print = match &input {
                ResolvedVarnode::Direct(d) => model
                    .arch_info
                    .get_space(d.space_index)
                    .map(|s| s._type == SpaceType::IPTR_PROCESSOR)
                    .unwrap_or(false),
                ResolvedVarnode::Indirect(_) => true,
            };

            if !should_print {
                continue;
            }

            let input_desc = format_resolved_varnode(&input, model);

            // Try to read the value from the initial state of this gadget
            if let Ok(bv) = gadget.get_original_state().read_resolved(&input) {
                if let Some(val) = model.model().eval(&bv, true) {
                    println!("  {} = {}", input_desc, val);
                    inputs_set.insert(input_desc);
                } else {
                    println!("  {} = <unable to evaluate>", input_desc);
                }
            } else {
                println!("  {} = <unable to read>", input_desc);
            }
        }
    }
    println!();

    // Collect all outputs and their valuations at the end of the chain
    println!("--- Outputs (Locations Written) ---");
    let mut outputs_set: BTreeSet<String> = BTreeSet::new();

    for (gadget_idx, gadget) in model.gadgets.iter().enumerate() {
        println!("Gadget {}:", gadget_idx);
        for output in gadget.get_outputs() {
            // Filter out unique space variables (keep only IPTR_PROCESSOR)
            let should_print = match &output {
                ResolvedVarnode::Direct(d) => model
                    .arch_info
                    .get_space(d.space_index)
                    .map(|s| s._type == SpaceType::IPTR_PROCESSOR)
                    .unwrap_or(false),
                ResolvedVarnode::Indirect(_) => true,
            };

            if !should_print {
                continue;
            }

            let output_desc = format_resolved_varnode(&output, model);

            // Read the value from the final state of this gadget
            if let Ok(bv) = gadget.get_final_state().read_resolved(&output) {
                if let Some(val) = model.model().eval(&bv, true) {
                    println!("  {} = {}", output_desc, val);
                    outputs_set.insert(output_desc);
                } else {
                    println!("  {} = <unable to evaluate>", output_desc);
                }
            } else {
                println!("  {} = <unable to read>", output_desc);
            }
        }
    }

    println!("\n==============================================\n");
}
