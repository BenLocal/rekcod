use clap::{arg, command, Args, Subcommand};
use rekcod_core::{
    api::{
        req::NodeListRequest,
        resp::{ApiJsonResponse, NodeItemResponse},
    },
    client::get_client,
};
use tabled::{settings::Style, Table};

use crate::config::rekcod_cli_config;

#[derive(Subcommand, Debug)]
#[command(author, version, about = "node command", long_about = None)]
pub enum NodeArgs {
    List(ListNodeArgs),
}

#[derive(Debug, Args)]
#[command(author, version, about = "list node, alias: ls", alias = "ls", long_about = None)]
pub struct ListNodeArgs {
    #[arg(short, long, default_value_t = false)]
    pub all: bool,
}

pub(crate) async fn run(args: NodeArgs) -> anyhow::Result<()> {
    match args {
        NodeArgs::List(args) => list_node(args).await,
    }
}

async fn list_node(args: ListNodeArgs) -> anyhow::Result<()> {
    let config = rekcod_cli_config();

    let req = NodeListRequest { all: args.all };
    let resp = get_client()?
        .post(format!("{}/node/list", config.http_server_host()))
        .json(&req)
        .send()
        .await?
        .json::<ApiJsonResponse<Vec<NodeItemResponse>>>()
        .await?;

    println!("{:?}", resp);

    if resp.code() != 0 {
        return Err(anyhow::anyhow!("{}", resp.msg()));
    }

    let mut table = if let Some(data) = resp.data() {
        Table::new(data)
    } else {
        Table::default()
    };

    table.with(Style::blank());

    println!("{}", table);
    Ok(())
}
