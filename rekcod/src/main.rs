use std::env;

use clap::{command, Parser, Subcommand};
use config::RekcodCliConfig;
use node::NodeArgs;
use rekcod_core::{auth::set_token, obj::RekcodCfg};
use tracing::{error, info};

mod config;
mod node;

#[derive(Parser)]
#[command(name = "rekcod")]
#[command(bin_name = "rekcod")]
struct RekcodArgs {
    #[arg(short, long)]
    pub rekcod_config: Option<String>,

    #[command(subcommand)]
    pub command: RekcodSubCommand,
}

#[derive(Subcommand, Debug)]
enum RekcodSubCommand {
    #[command(subcommand)]
    Node(NodeArgs),
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .compact()
        .with_max_level(tracing::Level::INFO)
        .with_thread_ids(true)
        .with_thread_names(true)
        .init();

    let args = RekcodArgs::parse();

    let cfg = match init_token(args.rekcod_config).await {
        Ok(c) => c,
        Err(e) => {
            error!("{:?}", e);
            std::process::exit(1);
        }
    };

    config::init_rekcod_cli_config(RekcodCliConfig { host: cfg.host });

    match args.command {
        RekcodSubCommand::Node(args) => {
            node::run(args).await;
        }
    }
}

async fn init_token(rekcod_config: Option<String>) -> anyhow::Result<RekcodCfg> {
    let path = rekcod_config.unwrap_or(env::var("REKCOD_CONFIG").map(|x| x.to_string())?);
    info!("config path: {}", path);
    let cfg_str = tokio::fs::read_to_string(&path).await?;

    let c = serde_json::from_str::<RekcodCfg>(&cfg_str)?;
    set_token(c.token.clone());

    Ok(c)
}
