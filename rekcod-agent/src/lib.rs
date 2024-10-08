use axum::{
    body::Body,
    extract::{Request, State},
    response::{IntoResponse as _, Response},
    routing::any,
    Router,
};

use docker::DockerProxyClient;
use hyper::StatusCode;
use hyper_util::rt::TokioExecutor;
use hyperlocal::UnixConnector;

const DOCKER_PROXY_PATH: &'static str = "/proxy.docker";

mod docker;

pub fn routers() -> Router {
    let client = DockerProxyClient::new();

    Router::new()
        .route(
            &format!("{}/*path", DOCKER_PROXY_PATH),
            any(docker_proxy_handler),
        )
        .with_state(client)
}

async fn docker_proxy_handler(
    State(client): State<DockerProxyClient>,
    mut req: Request,
) -> Result<Response, StatusCode> {
    let path = req.uri().path();
    let path_query = req
        .uri()
        .path_and_query()
        .map(|v| {
            let without_prefix = v.as_str().strip_prefix(DOCKER_PROXY_PATH).unwrap_or(path);
            without_prefix
        })
        .unwrap_or(path);

    match client {
        #[cfg(unix)]
        DockerProxyClient::Unix(c) => {
            *req.uri_mut() = hyperlocal::Uri::new("/var/run/docker.sock", path_query).into();
            Ok(c.request(req)
                .await
                .map_err(|_| StatusCode::BAD_REQUEST)?
                .into_response())
        }
        #[cfg(windows)]
        DockerProxyClient::Windows(client) => {
            let host = hex::encode("//./pipe/docker_engine");
            *req.uri_mut() = hyperlocal::Uri::new("//./pipe/docker_engine", path_query).into();
            Ok(c.request(req)
                .await
                .map_err(|_| StatusCode::BAD_REQUEST)?
                .into_response())
        }
    }
}
