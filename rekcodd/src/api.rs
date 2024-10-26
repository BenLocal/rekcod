use axum::{routing::get, Router};
use rekcod_server::api::socketio::socketio_routers;
use tokio_util::sync::CancellationToken;
use tower_http::cors::CorsLayer;
use tracing::info;

use crate::config;

pub(crate) async fn start(cancel: CancellationToken) -> anyhow::Result<()> {
    let config = config::rekcod_config();

    let mut app = Router::new()
        .route("/healthz", get(|| async { "UP" }))
        .merge(rekcod_agent::routers());

    if config.server_type == config::RekcodServerType::Server {
        app = app.merge(rekcod_server::routers());

        // this is maybe a problem in socketioxide
        // with axum router nest it will not work, maybe set a req path in axum router
        // see https://github.com/Totodore/socketioxide/issues/36
        app = app.layer(
            tower::ServiceBuilder::new()
                .layer(CorsLayer::permissive())
                .layer(socketio_routers()),
        );
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
