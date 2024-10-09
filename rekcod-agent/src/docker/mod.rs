use hyper::Uri;

#[cfg(unix)]
pub mod unix;
#[cfg(windows)]
pub mod win;

#[derive(Debug, Clone)]
pub enum DockerProxyClient {
    #[cfg(unix)]
    Unix(unix::SocketFileClient),
    #[cfg(windows)]
    Windows(win::SocketFileClient),
}

impl DockerProxyClient {
    pub fn new() -> DockerProxyClient {
        #[cfg(unix)]
        return unix::SocketFileClient::new_client();
        #[cfg(windows)]
        return win::SocketFileClient::new_client();
    }
}

pub(crate) trait DockerProxyInterface {
    fn new_client() -> DockerProxyClient;

    fn uri(&self, path_query: &str) -> anyhow::Result<Uri>;
}
