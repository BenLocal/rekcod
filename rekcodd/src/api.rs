use axum::{routing::get, Router};
use tokio_util::sync::CancellationToken;
use tracing::info;

use crate::config;

pub(crate) async fn start(cancel: CancellationToken) -> anyhow::Result<()> {
    let config = config::rekcod_config();

    let mut app = Router::new()
        .route("/healthz", get(|| async { "UP" }))
        .merge(rekcod_agent::routers());

    if config.server_type == config::RekcodServerType::Server {
        app = app.merge(rekcod_server::routers());
    }

    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{}", config.api_port)).await?;
    info!("listening on {}", listener.local_addr()?);
    axum::serve(listener, app)
        .with_graceful_shutdown(async move {
            tokio::select! {
                _ = cancel.cancelled() => {
                    info!("api server shutdown");
                },
            }
        })
        .await
        .map_err(|e| e.into())
}
