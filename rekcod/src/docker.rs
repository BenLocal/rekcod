use bollard::{container::ListContainersOptions, Docker};
use clap::{command, Args, Subcommand};
use rekcod_core::{
    api::{
        req::NodeInfoRequest,
        resp::{ApiJsonResponse, NodeItemResponse},
    },
    auth::get_token,
    client::get_client,
    docker::rekcod_connect,
};
use tabled::{settings::Style, Table, Tabled};
use tracing::error;

use crate::config::rekcod_cli_config;

#[derive(Debug, Args)]
#[command(author, version, about = "docker command", long_about = None)]
pub struct DockerArgs {
    #[arg(short, long)]
    pub node: String,

    #[command(subcommand)]
    pub command: DockerSubCommand,
}

#[derive(Debug, Subcommand)]
pub enum DockerSubCommand {
    List(DockerListArgs),
}

#[derive(Debug, Args)]
#[command(author, version, about = "list node, alias: ls", alias = "ls", long_about = None)]
pub struct DockerListArgs {
    #[arg(short, long, default_value_t = false)]
    pub all: bool,
}

pub(crate) async fn run(args: DockerArgs) {
    println!("docker: {:?}", args);

    if let Ok(docker_client) = inner_run(&args).await {
        if let Err(e) = match args.command {
            DockerSubCommand::List(sub) => list_docker(&docker_client, &sub).await,
        } {
            error!("{}", e);
        }
    } else {
        error!("docker connect error");
    }
}

async fn inner_run(args: &DockerArgs) -> anyhow::Result<Docker> {
    let config = rekcod_cli_config();
    let req = NodeInfoRequest {
        name: args.node.clone(),
    };
    let resp = get_client()?
        .post(format!("{}/node/info", config.http_server_host()))
        .json(&req)
        .send()
        .await?
        .json::<ApiJsonResponse<NodeItemResponse>>()
        .await?;

    if resp.code() != 0 {
        return Err(anyhow::anyhow!("{}", resp.msg()));
    }

    if let Some(data) = resp.data() {
        let docker_client = rekcod_connect(
            Some(format!("http://{}:{}", data.ip, data.port)),
            rekcod_core::constants::DOCKER_PROXY_PATH,
            40,
            get_token(),
        )?;
        return Ok(docker_client);
    } else {
        return Err(anyhow::anyhow!("node {} not found", args.node));
    }
}

#[derive(Debug, Tabled)]
#[tabled(rename_all = "UPPERCASE")]
struct ContainerInfo {
    id: String,
    name: String,
    status: String,
    image: String,
}

impl From<bollard::models::ContainerSummary> for ContainerInfo {
    fn from(ci: bollard::models::ContainerSummary) -> Self {
        Self {
            name: ci
                .names
                .map(|s| s.into_iter().next())
                .flatten()
                .unwrap_or("".to_string()),
            status: ci.status.unwrap_or("".to_string()),
            image: ci.image.unwrap_or("".to_string()),
            id: ci.id.unwrap_or("".to_string()).chars().take(12).collect(),
        }
    }
}

async fn list_docker(docker_client: &Docker, args: &DockerListArgs) -> anyhow::Result<()> {
    println!("list docker: {:?}", args);

    let options = Some(ListContainersOptions::<String> {
        all: args.all,
        ..Default::default()
    });
    let ls = docker_client
        .list_containers(options)
        .await?
        .into_iter()
        .map(ContainerInfo::from)
        .collect::<Vec<_>>();

    let mut table = if ls.len() > 0 {
        Table::new(&ls)
    } else {
        Table::default()
    };

    table.with(Style::blank());
    println!("{}", table);
    Ok(())
}
