use axum::body::Body;
use hyper::Uri;
use hyper_named_pipe::{NamedPipeConnector, NAMED_PIPE_SCHEME};
use hyper_util::rt::TokioExecutor;

use super::{DockerProxyClient, DockerProxyInterface};

pub type SocketFileClient = hyper_util::client::legacy::Client<NamedPipeConnector, Body>;

impl DockerProxyInterface for SocketFileClient {
    fn new_client() -> super::DockerProxyClient {
        DockerProxyClient::Windows(
            hyper_util::client::legacy::Client::builder(TokioExecutor::new())
                .build(NamedPipeConnector),
        )
    }

    fn uri(&self, path_query: &str) -> anyhow::Result<Uri> {
        let host = hex::encode("//./pipe/docker_engine");
        Ok(Uri::try_from(format!(
            "{}://{}:0{}",
            NAMED_PIPE_SCHEME, host, path_query
        ))?)
    }
}
