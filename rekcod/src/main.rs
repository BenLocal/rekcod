use clap::Parser;
use rekcod_agent::config::{init_rekcod_agent_config, RekcodAgentConfig};
use rekcod_core::obj::RekcodType;
use rekcod_server::config::{init_rekcod_server_config, RekcodServerConfig};
use tokio_util::sync::CancellationToken;
use tracing::{error, level_filters::LevelFilter};

mod api;
mod config;

#[derive(Parser)]
#[command(name = "rekcod")]
#[command(bin_name = "rekcod")]
enum RekcodArgs {
    Server(ServerArgs),
    Agent(AgentArgs),
}

#[derive(clap::Args, Clone)]
#[command(author, version, about = "run server", long_about = None)]
pub(crate) struct ServerArgs {
    #[arg(short, long, default_value_t = 6734)]
    pub port: u16,

    #[clap(short = 'd', long, default_value = "sqlite://db.sqlite?mode=rwc")]
    pub db_url: String,

    #[clap(long, default_value = "/home/rekcod/data")]
    pub data_path: String,
}

#[derive(clap::Args, Clone)]
#[command(author, version, about = "run agent", long_about = None)]
pub(crate) struct AgentArgs {
    #[arg(short, long, default_value_t = 6734)]
    pub port: u16,

    #[clap(long, default_value = "/home/rekcod/data")]
    pub data_path: String,
}

fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .compact()
        .with_max_level(LevelFilter::INFO)
        .with_thread_ids(true)
        .with_thread_names(true)
        .init();

    let cli = RekcodArgs::parse();

    match cli {
        RekcodArgs::Server(args) => {
            // just for start
            let arg_clone = args.clone();
            init_rekcod_server_config(RekcodServerConfig {
                db_url: arg_clone.db_url,
            });

            let arg_clone = args.clone();
            init_rekcod_agent_config(RekcodAgentConfig {
                data_path: arg_clone.data_path,
                typ: RekcodType::Master,
            });

            config::init_rekcod_config(args.into());
        }
        RekcodArgs::Agent(args) => {
            let arg_clone = args.clone();
            init_rekcod_agent_config(RekcodAgentConfig {
                data_path: arg_clone.data_path,
                typ: RekcodType::Agent,
            });

            config::init_rekcod_config(args.into());
        }
    };

    run_main()
}

#[tokio::main]
async fn run_main() -> anyhow::Result<()> {
    let cancel = CancellationToken::new();

    let cancel_clone = cancel.clone();
    tokio::spawn(async move {
        let ccc = cancel_clone.clone();
        if let Err(e) = api::start(cancel_clone).await {
            error!("api server error: {}", e);
            ccc.cancel();
        }
    });

    let config = config::rekcod_config();
    if config.server_type == config::RekcodServerType::Server {
        let cancel_clone = cancel.clone();
        tokio::spawn(async move {
            let ccc = cancel_clone.clone();
            if let Err(e) = rekcod_server::init(cancel_clone).await {
                println!("server init error: {}", e);
                ccc.cancel();
            }
        });
    }

    loop {
        tokio::select! {
            _ = tokio::signal::ctrl_c() => {
                cancel.cancel();
                break;
            }
            _ = cancel.cancelled() => {
                break;
            }
        }
    }

    Ok(())
}
