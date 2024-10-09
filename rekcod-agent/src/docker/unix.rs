use axum::body::Body;
use hyper_util::rt::TokioExecutor;
use hyperlocal::UnixConnector;

use super::{DockerProxyClient, DockerProxyInterface};

pub type SocketFileClient = hyper_util::client::legacy::Client<UnixConnector, Body>;

impl DockerProxyInterface for SocketFileClient {
    fn new_client() -> super::DockerProxyClient {
        DockerProxyClient::Unix(
            hyper_util::client::legacy::Client::builder(TokioExecutor::new()).build(UnixConnector),
        )
    }

    fn uri(&self, path_query: &str) -> anyhow::Result<hyper::Uri> {
        let host = hex::encode("/var/run/docker.sock");
        Ok(format!("{}://{}:0{}", "unix", host, path_query).parse()?)
    }
}
