use clap::Parser;
use tokio_util::sync::CancellationToken;

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
}

#[derive(clap::Args, Clone)]
#[command(author, version, about = "run agent", long_about = None)]
pub(crate) struct AgentArgs {
    #[arg(short, long, default_value_t = 6734)]
    pub port: u16,
}

fn main() -> anyhow::Result<()> {
    let cli = RekcodArgs::parse();

    match cli {
        RekcodArgs::Server(args) => config::init_rekcod_config(args.into()),
        RekcodArgs::Agent(args) => config::init_rekcod_config(args.into()),
    };

    run_main()
}

#[tokio::main]
async fn run_main() -> anyhow::Result<()> {
    let cancel = CancellationToken::new();

    let cancel_clone = cancel.clone();
    tokio::spawn(async move {
        let ccc = cancel_clone.clone();
        if let Err(_) = api::start(cancel_clone).await {
            println!("api server error");
            ccc.cancel();
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
