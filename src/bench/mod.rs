use std::fs;
use std::path::PathBuf;

use clap::Parser;
use tracing::level_filters::LevelFilter;
use tracing::{event, Level};
use tracing_indicatif::IndicatifLayer;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::EnvFilter;
use z3::{Config, Context};

use crate::config::CrackersConfig;
use crate::synthesis::DecisionResult;

#[derive(Clone, Debug, Parser)]
pub struct BenchCommand {
    crackers_config: PathBuf,
    gadgets_per_slot: usize,
}
pub fn bench(config: BenchCommand) -> anyhow::Result<()> {
    let z3_cfg = Config::new();
    let z3 = Context::new(&z3_cfg);
    let cfg_bytes = fs::read(config.crackers_config)?;
    let s = String::from_utf8(cfg_bytes)?;
    let mut p: CrackersConfig = toml_edit::de::from_str(&s)?;
    p.synthesis.max_candidates_per_slot = config.gadgets_per_slot;

    let level = Level::from(p.meta.log_level);
    let env_filter = EnvFilter::builder()
        .with_default_directive(LevelFilter::ERROR.into())
        .from_env()?
        .add_directive(format!("crackers={}", level).parse()?);
    let indicatif_layer = IndicatifLayer::new();
    let writer = indicatif_layer.get_stderr_writer();
    tracing_subscriber::registry()
        .with(env_filter)
        .with(indicatif_layer)
        .with(tracing_subscriber::fmt::layer().with_writer(writer))
        .init();
    let params = p.resolve()?;
    match params.build_single(&z3) {
        Ok(mut a) => match a.decide() {
            Ok(a) => match a {
                DecisionResult::AssignmentFound(_) => {
                    event!(Level::INFO, "Synthesis succeeded!")
                }
                DecisionResult::Unsat(_) => {
                    event!(Level::INFO, "Synthesis failed!")
                }
            },
            Err(e) => {
                event!(Level::ERROR, "Synthesis encountered an error: {}", e)
            }
        },
        Err(_) => {
            event!(Level::ERROR, "Unable to find gadgets for a step")
        }
    }
    Ok(())
}
