use axum::body::Body;
use hyper_util::rt::TokioExecutor;
use hyperlocal::UnixConnector;

pub type SocketFileClient = hyper_util::client::legacy::Client<UnixConnector, Body>;

pub fn new_client() -> SocketFileClient {
    hyper_util::client::legacy::Client::builder(TokioExecutor::new()).build(UnixConnector)
}
