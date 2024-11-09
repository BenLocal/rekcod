use clap::Args;
use rekcod_core::{
    api::{
        req::NodeInfoRequest,
        resp::{ApiJsonResponse, NodeItemResponse},
    },
    auth::get_token,
    client::get_client,
    constants::{DOCKER_PROXY_PATH, TOEKN_HEADER_KEY},
};
use tokio::process::Command;

use crate::config::rekcod_cli_config;

#[derive(Debug, Args)]
#[command(author, version, about = "docker compose command", long_about = None)]
pub struct DockerArgs {
    #[arg(short, long)]
    pub node: String,

    #[clap(trailing_var_arg = true)]
    pub sub_command: Vec<String>,
}

struct DockerCli(Command);

pub(crate) async fn run(args: DockerArgs) -> anyhow::Result<()> {
    let mut docker_cli = inner_run(&args).await?;
    let mut out = docker_cli.0.spawn()?;
    out.wait().await?;

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
        let docker_compose_path = match which::which("docker compose") {
            Ok(path) => path,
            Err(_) => match which::which("docker-compose") {
                Ok(path) => path,
                Err(_) => {
                    return Err(anyhow::anyhow!(
                        "docker compose is not installed, please install it first"
                    ));
                }
            },
        };
        let mut cmd = tokio::process::Command::new(docker_compose_path);
        cmd.env(
            "DOCKER_HOST",
            format!("tcp://{}:{}{}", data.ip, data.port, DOCKER_PROXY_PATH),
        );
        cmd.env(
            "DOCKER_CUSTOM_HEADERS",
            format!("{}={}", TOEKN_HEADER_KEY, get_token()),
        );
        cmd.args(&args.sub_command);

        return Ok(DockerCli(cmd));
    } else {
        return Err(anyhow::anyhow!("node {} not found", args.node));
    }
}
