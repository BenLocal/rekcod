use axum::{
    extract::{Request, State},
    response::{IntoResponse as _, Response},
    routing::any,
    Router,
};
use rekcod_core::constants::{DOCKER_PROXY_PATH, REKCOD_AGENT_PREFIX_PATH};

use crate::docker::DockerProxyInterface;
use docker::DockerProxyClient;
use hyper::StatusCode;

mod agent;
pub mod config;
mod docker;

pub fn routers() -> Router {
    let client = DockerProxyClient::new();

    Router::new()
        .route(
            &format!("{}/*path", DOCKER_PROXY_PATH),
            any(docker_proxy_handler),
        )
        .with_state(client)
        .nest(REKCOD_AGENT_PREFIX_PATH, agent::routers())
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

    let c = match client {
        #[cfg(unix)]
        DockerProxyClient::Unix(c) => c,
        #[cfg(windows)]
        DockerProxyClient::Windows(c) => c,
    };

    *req.uri_mut() = c.uri(path_query).map_err(|_| StatusCode::BAD_REQUEST)?;
    Ok(c.request(req)
        .await
        .map_err(|_| StatusCode::BAD_REQUEST)?
        .into_response())
}
