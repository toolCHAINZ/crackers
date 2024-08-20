use std::fs;
use std::path::PathBuf;

use clap::Subcommand;
use serde::{Deserialize, Serialize};
use toml_edit::ser::to_string_pretty;

#[derive(Deserialize, Serialize)]
pub struct BenchCommandConfig {
    meta: BenchMeta,
    bench: BenchConfig,
    output: OutputConfig,
}
#[derive(Deserialize, Serialize)]
pub struct BenchMeta {
    #[serde(rename = "crackers_config")]
    crackers_config_path: PathBuf,
}

#[derive(Deserialize, Serialize)]
pub struct BenchConfig {
    min_gadgets_per_step: usize,
    max_gadgets_per_step: usize,
    gadget_step: usize,
}

#[derive(Deserialize, Serialize)]
pub enum OutputFormat {
    Latex,
}
#[derive(Deserialize, Serialize)]
pub struct OutputConfig {
    path: PathBuf,
    format: OutputFormat,
}
#[derive(Clone, Debug, Subcommand)]
pub enum BenchCommand {
    New { path: Option<PathBuf> },
}
pub fn bench(cfg: BenchCommand) -> anyhow::Result<()> {
    match cfg {
        BenchCommand::New { path } => new(path.unwrap_or(PathBuf::from("bench.toml"))),
    }
}

fn new(path: PathBuf) -> anyhow::Result<()> {
    let bench = BenchCommandConfig {
        meta: BenchMeta {
            crackers_config_path: PathBuf::from("crackers.toml"),
        },
        bench: BenchConfig {
            min_gadgets_per_step: 50,
            max_gadgets_per_step: 1000,
            gadget_step: 50,
        },
        output: OutputConfig {
            path: PathBuf::from("out.tex"),
            format: OutputFormat::Latex,
        },
    };
    fs::write(path, to_string_pretty(&bench)?)?;
    Ok(())
}
