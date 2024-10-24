use axum::{
    extract::{Request, State},
    response::{IntoResponse as _, Response},
    routing::any,
    Router,
};
use rekcod_core::constants::{DOCKER_PROXY_PATH, REKCOD_AGENT_PREFIX_PATH};
use tokio_util::sync::CancellationToken;

use crate::docker::DockerProxyInterface;
use docker::DockerProxyClient;
use hyper::StatusCode;

mod agent;
pub mod config;
mod docker;
mod register;
mod sys;

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

pub async fn init(cancel: CancellationToken) -> anyhow::Result<()> {
    let cancel_clone = cancel.clone();
    tokio::spawn(async move {
        let cancel_clone_end = cancel_clone.clone();
        if let Err(e) = register::register_node(cancel_clone).await {
            println!("agent register error: {}", e);
            cancel_clone_end.cancel();
        }
    });

    let cancel_clone = cancel.clone();
    tokio::spawn(async move {
        let cancel_clone_end = cancel_clone.clone();
        if let Err(e) = sys::sys_monitor(cancel_clone).await {
            println!("sys monitor error: {}", e);
            cancel_clone_end.cancel();
        }
    });

    Ok(())
}
