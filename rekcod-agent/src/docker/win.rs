pub type SocketFileClient = hyper_util::client::legacy::Client<NamedPipeConnector, Body>;

pub fn new_client() -> SocketFileClient {
    hyper_util::client::legacy::Client::builder(TokioExecutor::new()).build(NamedPipeConnector)
}
