use std::env;

use axum::body::Body;
use hyper_util::rt::TokioExecutor;
use hyperlocal::UnixConnector;

use super::{DockerProxyClient, DockerProxyInterface};

pub type SocketFileClient = hyper_util::client::legacy::Client<UnixConnector, Body>;

const DEFAULT_DOCKER_HOST: &str = "unix:///var/run/docker.sock";

impl DockerProxyInterface for SocketFileClient {
    fn new_client() -> super::DockerProxyClient {
        DockerProxyClient::Unix(
            hyper_util::client::legacy::Client::builder(TokioExecutor::new()).build(UnixConnector),
        )
    }

    fn uri(&self, path_query: &str) -> anyhow::Result<hyper::Uri> {
        let host = env::var("DOCKER_HOST").unwrap_or_else(|_| DEFAULT_DOCKER_HOST.to_string());
        if !host.starts_with("unix://") {
            return Err(anyhow::anyhow!("DOCKER_HOST just support start with unix://").into());
        }

        let client_addr = host.replacen("unix://", "", 1);
        let host = hex::encode(&client_addr);
        Ok(format!("{}://{}:0{}", "unix", host, path_query).parse()?)
    }
}
