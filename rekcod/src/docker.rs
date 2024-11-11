use clap::Args;
use rekcod_core::{
    api::{
        req::NodeInfoRequest,
        resp::{ApiJsonResponse, NodeItemResponse},
    },
    client::get_client,
    docker::DockerCli,
};

use crate::config::rekcod_cli_config;

#[derive(Debug, Args)]
#[command(author, version, about = "docker command", long_about = None)]
pub struct DockerArgs {
    #[arg(short, long)]
    pub node: String,

    #[clap(trailing_var_arg = true)]
    pub sub_command: Vec<String>,
}

pub(crate) async fn run(args: DockerArgs) -> anyhow::Result<()> {
    let mut docker_cli = inner_run(&args).await?;
    docker_cli.run().await?;
    Ok(())
}

async fn inner_run(args: &DockerArgs) -> anyhow::Result<DockerCli> {
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
        return Ok(DockerCli::new(&data.ip, data.port, &args.sub_command)?);
    } else {
        return Err(anyhow::anyhow!("node {} not found", args.node));
    }
}
