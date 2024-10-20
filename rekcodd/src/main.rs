use clap::Parser;
use rekcod_agent::config::{init_rekcod_agent_config, RekcodAgentConfig};
use rekcod_core::{auth::set_token, obj::RekcodType};
use rekcod_server::config::{init_rekcod_server_config, RekcodServerConfig};
use tokio_util::sync::CancellationToken;
use tracing::{error, level_filters::LevelFilter};

mod api;
mod config;

#[derive(Parser)]
#[command(name = "rekcodd")]
#[command(bin_name = "rekcodd")]
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

    #[clap(long, default_value = "/etc/rekcod")]
    pub etc_path: String,
}

#[derive(clap::Args, Clone)]
#[command(author, version, about = "run agent", long_about = None)]
pub(crate) struct AgentArgs {
    #[arg(short, long, default_value_t = 6734)]
    pub port: u16,

    #[clap(long, default_value = "/home/rekcod/data")]
    pub data_path: String,

    #[clap(long)]
    pub master_host: String,

    #[clap(long, default_value = "/etc/rekcod")]
    pub etc_path: String,

    #[clap(long)]
    pub token: String,
}

impl Into<RekcodAgentConfig> for AgentArgs {
    fn into(self) -> RekcodAgentConfig {
        RekcodAgentConfig {
            data_path: self.data_path,
            master_host: self.master_host,
            typ: RekcodType::Agent,
            api_port: self.port,
            etc_path: self.etc_path,
        }
    }
}

impl Into<RekcodServerConfig> for ServerArgs {
    fn into(self) -> RekcodServerConfig {
        RekcodServerConfig {
            db_url: self.db_url,
            etc_path: self.etc_path,
            api_port: self.port,
        }
    }
}

impl Into<RekcodAgentConfig> for ServerArgs {
    fn into(self) -> RekcodAgentConfig {
        RekcodAgentConfig {
            data_path: self.data_path,
            master_host: format!("127.0.0.1:{}", self.port),
            typ: RekcodType::Master,
            api_port: self.port,
            etc_path: self.etc_path,
        }
    }
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
            init_rekcod_server_config(arg_clone.into());

            let arg_clone = args.clone();
            init_rekcod_agent_config(arg_clone.into());

            config::init_rekcod_config(args.into());
        }
        RekcodArgs::Agent(args) => {
            let arg_clone = args.clone();
            init_rekcod_agent_config(arg_clone.into());

            // init agent token
            set_token(args.token.clone());
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
            let cancel_clone_end = cancel_clone.clone();
            if let Err(e) = rekcod_server::init(cancel_clone).await {
                println!("server init error: {}", e);
                cancel_clone_end.cancel();
            }
        });
    }

    // init agent after server
    let cancel_clone = cancel.clone();
    tokio::spawn(async move {
        let cancel_clone_end = cancel_clone.clone();
        if let Err(e) = rekcod_agent::init(cancel_clone).await {
            println!("agent init error: {}", e);
            cancel_clone_end.cancel();
        }
    });

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