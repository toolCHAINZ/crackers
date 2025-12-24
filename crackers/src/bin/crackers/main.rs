use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

use clap::{Parser, Subcommand};
use jingle::analysis::varnode::VarNodeSet;
use toml_edit::ser::to_string_pretty;
use tracing::{Level, event};
use tracing_indicatif::IndicatifLayer;
use tracing_subscriber::EnvFilter;
use tracing_subscriber::filter::LevelFilter;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;

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
use jingle::sleigh::{SpaceType, VarNode};
use jingle::varnode::ResolvedVarnode;
use std::collections::BTreeSet;

#[derive(Parser, Debug)]
struct Arguments {
    pub cfg_path: String,
}

#[derive(Debug, Clone, Subcommand)]
pub enum CrackersCommands {
    /// Create a new crackers configuration file, optionally with a target library
    New {
        config: Option<PathBuf>,
        #[arg(short, long)]
        library: Option<PathBuf>,
    },
    /// Attempt to synthesize a code-reuse attack based on the provided configuration file
    Synth { config: PathBuf },
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
            // If the user explicitly provided a config path, refuse to overwrite an existing file.
            if let Some(cfg_path) = config {
                if cfg_path.exists() {
                    event!(
                        Level::WARN,
                        "Refusing to create new config: file already exists: {}",
                        cfg_path.display()
                    );
                    std::process::exit(1);
                }
                new(cfg_path.clone(), library.clone())
            } else {
                // If the user did not specify a config path, check the default file and
                // refuse to overwrite it as well.
                let default_path = PathBuf::from("./crackers.toml");
                if default_path.exists() {
                    event!(
                        Level::WARN,
                        "Refusing to create new config: default file already exists: {}",
                        default_path.display()
                    );
                    std::process::exit(1);
                }
                new(default_path, library.clone())
            }
        }
        CrackersCommands::Synth { config } => {
            // Synth initializes its own logging with config
            synthesize(config.clone())
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
            loaded_libraries: None,
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

/// Shared helper to collect varnodes (registers and iptr-style) from the model.
///
/// - `collect_inputs`: if true, collects gadget inputs; otherwise collects outputs.
/// - `use_original_state`: if true, reads values from the chain's original state; otherwise final state.
///
/// Returns: (register_vector, varnode_set, iptr_description_set)
fn collect_varnodes<T: ModelingContext>(
    model: &AssignmentModel<T>,
    collect_inputs: bool,
    use_original_state: bool,
) -> (Vec<(String, String)>, VarNodeSet, BTreeSet<String>) {
    let mut reg_vec: Vec<(String, String)> = Vec::new();
    let mut reg_seen: BTreeSet<String> = BTreeSet::new();

    let mut iptr_vn_set: VarNodeSet = VarNodeSet::default();
    let mut iptr_descs: BTreeSet<String> = BTreeSet::new();

    for gadget in model.gadgets.iter() {
        let iter = if collect_inputs {
            gadget.get_inputs()
        } else {
            gadget.get_outputs()
        };

        for vn in iter {
            match &vn {
                ResolvedVarnode::Direct(d) => {
                    let space_info = model.arch_info.get_space(d.space_index);
                    let is_register = space_info.map(|s| s.name == "register").unwrap_or(false);
                    let is_iptr = space_info
                        .map(|s| s._type == SpaceType::IPTR_PROCESSOR)
                        .unwrap_or(false);

                    let desc = format_resolved_varnode(&vn, model);

                    // choose state to read from
                    let read_result = if use_original_state {
                        gadget.get_original_state().read_resolved(&vn)
                    } else {
                        gadget.get_final_state().read_resolved(&vn)
                    };

                    let val_str = match read_result {
                        Ok(bv) => match model.model().eval(&bv, true) {
                            Some(v) => format!("{}", v),
                            None => "<unable to evaluate>".to_string(),
                        },
                        Err(_) => "<unable to read>".to_string(),
                    };

                    if is_register {
                        if !reg_seen.contains(&desc) {
                            reg_seen.insert(desc.clone());
                            reg_vec.push((desc, val_str));
                        }
                    } else if is_iptr {
                        // convert Direct resolved varnode into VarNode and insert
                        let vn_struct = VarNode {
                            size: d.size,
                            space_index: d.space_index,
                            offset: d.offset,
                        };
                        iptr_vn_set.insert(&vn_struct);
                        iptr_descs.insert(format!("{} = {}", desc, val_str));
                    }
                }
                ResolvedVarnode::Indirect(i) => {
                    // insert the pointer_location varnode into set
                    let ptr_loc = &i.pointer_location;
                    let vn_struct = VarNode {
                        size: ptr_loc.size,
                        space_index: ptr_loc.space_index,
                        offset: ptr_loc.offset,
                    };
                    iptr_vn_set.insert(&vn_struct);

                    let desc = format_resolved_varnode(&vn, model);
                    let read_result = if use_original_state {
                        gadget.get_original_state().read_resolved(&vn)
                    } else {
                        gadget.get_final_state().read_resolved(&vn)
                    };
                    let val_str = match read_result {
                        Ok(bv) => match model.model().eval(&bv, true) {
                            Some(v) => format!("{}", v),
                            None => "<unable to evaluate>".to_string(),
                        },
                        Err(_) => "<unable to read>".to_string(),
                    };
                    iptr_descs.insert(format!("{} = {}", desc, val_str));
                }
            }
        }
    }

    (reg_vec, iptr_vn_set, iptr_descs)
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

    // Inputs (read values from original state)
    println!("--- Inputs (Locations Read) ---\n");
    let (mut reg_vec, _iptr_vn_set, iptr_descs) = collect_varnodes(model, true, true);

    // sort alphabetically by register description
    reg_vec.sort_by(|a, b| a.0.cmp(&b.0));

    for (desc, val) in &reg_vec {
        println!("  {} = {}", desc, val);
    }
    for desc in &iptr_descs {
        println!("  {}", desc);
    }

    println!();

    // Outputs (read values from final state)
    println!("--- Outputs (Locations Written) ---\n");
    let (mut out_reg_vec, _out_iptr_vn_set, out_iptr_descs) = collect_varnodes(model, false, false);

    // sort alphabetically by register description
    out_reg_vec.sort_by(|a, b| a.0.cmp(&b.0));

    for (desc, val) in &out_reg_vec {
        println!("  {} = {}", desc, val);
    }
    for desc in &out_iptr_descs {
        println!("  {}", desc);
    }

    println!("\n==============================================\n");
}
