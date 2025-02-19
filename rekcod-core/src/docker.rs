use std::{ffi::OsStr, path::Path, process::Stdio, sync::Arc};

use axum::http::HeaderValue;
use bollard::{BollardRequest, Docker};
use once_cell::sync::Lazy;
use tokio::{io::AsyncWriteExt as _, process::Command};
use tracing::info;

use crate::{
    auth::get_token,
    constants::{DOCKER_PROXY_PATH, TOEKN_HEADER_KEY},
};

static DOCKER_LOCAL: Lazy<Docker> = Lazy::new(|| Docker::connect_with_defaults().unwrap());

pub fn rekcod_connect<S>(
    client_addr: Option<S>,
    path_prefix: &str,
    timeout: u64,
    token: &'static str,
) -> anyhow::Result<Docker>
where
    S: Into<String>,
{
    let http_connector = hyper_util::client::legacy::connect::HttpConnector::new();
    let mut client_builder =
        hyper_util::client::legacy::Client::builder(hyper_util::rt::TokioExecutor::new());
    client_builder.pool_max_idle_per_host(0);
    let http_client = Arc::new(client_builder.build(http_connector));

    let path_prefix = Arc::new(path_prefix.to_owned());
    let docker = Docker::connect_with_custom_transport(
        move |req: BollardRequest| {
            let http_client = Arc::clone(&http_client);
            let path_prefix = Arc::clone(&path_prefix);
            Box::pin(async move {
                let (mut p, b) = req.into_parts();
                // let _prev = p.headers.insert("host", host);
                let mut uri = p.uri.into_parts();
                uri.path_and_query = uri
                    .path_and_query
                    .map(|paq| {
                        info!("proxy docker request url: {:?}", paq);
                        hyper::http::uri::PathAndQuery::try_from(format!(
                            "{}{}",
                            path_prefix,
                            paq.as_str()
                        ))
                    })
                    .transpose()
                    .map_err(bollard::errors::Error::from)?;
                p.uri = uri.try_into().map_err(bollard::errors::Error::from)?;
                p.headers
                    .insert(TOEKN_HEADER_KEY, HeaderValue::from_static(token));

                let req = BollardRequest::from_parts(p, b);
                http_client
                    .request(req)
                    .await
                    .map_err(bollard::errors::Error::from)
            })
        },
        client_addr,
        timeout,
        bollard::API_DEFAULT_VERSION,
    )?;

    Ok(docker)
}

pub fn local_connect() -> &'static Docker {
    &DOCKER_LOCAL
}

pub struct DockerCli(Command);

impl DockerCli {
    pub fn new<I, S>(ip: &str, port: u16, args: I) -> anyhow::Result<Self>
    where
        I: IntoIterator<Item = S>,
        S: AsRef<OsStr>,
    {
        let docker_path = which::which("docker")?;
        let mut cmd = tokio::process::Command::new(docker_path);
        cmd.env(
            "DOCKER_HOST",
            format!("tcp://{}:{}{}", ip, port, DOCKER_PROXY_PATH),
        );
        cmd.env(
            "DOCKER_CUSTOM_HEADERS",
            format!("{}={}", TOEKN_HEADER_KEY, get_token()),
        );
        cmd.args(args);

        return Ok(DockerCli(cmd));
    }

    pub async fn run(&mut self) -> anyhow::Result<()> {
        let mut out = self.0.spawn()?;
        out.wait().await?;
        Ok(())
    }
}

pub struct DockerComposeCli(Command);

impl DockerComposeCli {
    pub fn new<I, S, P>(
        ip: &str,
        port: u16,
        args: I,
        current_dir: Option<P>,
    ) -> anyhow::Result<Self>
    where
        I: IntoIterator<Item = S>,
        S: AsRef<OsStr>,
        P: AsRef<Path>,
    {
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
            format!("tcp://{}:{}{}", ip, port, DOCKER_PROXY_PATH),
        );
        cmd.env(
            "DOCKER_CUSTOM_HEADERS",
            format!("{}={}", TOEKN_HEADER_KEY, get_token()),
        );
        // disable buildkit
        cmd.env("DOCKER_BUILDKIT", "0");
        cmd.args(args);

        if let Some(current_dir) = current_dir {
            cmd.current_dir(current_dir);
        }
        return Ok(DockerComposeCli(cmd));
    }

    pub async fn run(&mut self) -> anyhow::Result<()> {
        let mut out = self.0.spawn()?;
        out.wait().await?;
        Ok(())
    }

    pub async fn run_cache(&mut self, docker_compose_file: &str) -> anyhow::Result<()> {
        let mut child = self.0.stdin(Stdio::piped()).spawn()?;
        let stdin = match child.stdin.as_mut() {
            Some(stdin) => stdin,
            None => return Err(anyhow::anyhow!("stdin is not available")),
        };
        stdin.write_all(docker_compose_file.as_bytes()).await?;
        child.wait().await?;
        Ok(())
    }
}
