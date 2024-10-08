use axum::{extract::Request, response::Response};
use hyper::StatusCode;
use hyper_util::client::legacy::ResponseFuture;

#[cfg(unix)]
pub mod unix;
#[cfg(windows)]
pub mod win;

#[derive(Debug, Clone)]
pub enum DockerProxyClient {
    #[cfg(unix)]
    Unix(unix::SocketFileClient),
    #[cfg(windows)]
    Windows(win::NamedPipeClient),
}

impl DockerProxyClient {
    pub fn new() -> DockerProxyClient {
        #[cfg(unix)]
        return DockerProxyClient::Unix(unix::new_client());
        #[cfg(windows)]
        return DockerProxyClient::Windows(win::new_client());
    }
}
