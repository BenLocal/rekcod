use std::{env, path::Path};

use clap::{command, Parser, Subcommand};
use config::RekcodCliConfig;
use docker::DockerArgs;
use node::NodeArgs;
use rekcod_core::{
    auth::set_token,
    constants::{REKCOD_CONFIG_DEFAULT_PATH, REKCOD_CONFIG_FILE_NAME},
    obj::RekcodCfg,
};
use tracing::{debug, error};

mod config;
mod docker;
mod docker_compose;
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

    Docker(DockerArgs),

    DockerCompose(docker_compose::DockerArgs),
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

    if let Err(e) = match args.command {
        RekcodSubCommand::Node(args) => node::run(args).await,
        RekcodSubCommand::Docker(args) => docker::run(args).await,
        RekcodSubCommand::DockerCompose(docker_args) => docker_compose::run(docker_args).await,
    } {
        error!("{:?}", e);
        std::process::exit(1);
    }
}

async fn init_token(rekcod_config: Option<String>) -> anyhow::Result<RekcodCfg> {
    let path = get_rekcod_config_path(rekcod_config)?;
    debug!("config path: {}", path);
    let cfg_str = tokio::fs::read_to_string(&path).await?;

    let c = serde_json::from_str::<RekcodCfg>(&cfg_str)?;
    set_token(c.token.clone());

    Ok(c)
}

fn get_rekcod_config_path(rekcod_config: Option<String>) -> anyhow::Result<String> {
    if let Some(path) = rekcod_config {
        return Ok(path);
    }

    let path = env::var("REKCOD_CONFIG").map(|x| x.to_string());
    if let Ok(path) = path {
        return Ok(path);
    }

    // get default config path
    let path = Path::new(REKCOD_CONFIG_DEFAULT_PATH).join(REKCOD_CONFIG_FILE_NAME);

    if path.exists() {
        return Ok(path.to_string_lossy().to_string());
    }

    Err(anyhow::anyhow!("can not get rekcod config path"))
}
