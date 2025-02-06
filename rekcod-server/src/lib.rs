use std::{path::Path, sync::Arc};

use api::{node_proxy::create_node_proxy_client, socketio::socketio_routers};
use app::manager::get_app_tmpl_manager;
use axum::Router;

use rekcod_core::{
    auth::set_token,
    constants::{REKCOD_API_PREFIX_PATH, REKCOD_CONFIG_FILE_NAME, REKCOD_SERVER_PREFIX_PATH},
    obj::RekcodCfg,
};

use tokio_util::sync::CancellationToken;
use tower_http::cors::CorsLayer;
use tracing::info;
use uuid::Uuid;

mod api;
mod app;
pub mod config;
mod db;
mod env;
mod node;
mod server;

pub fn routers() -> Router {
    let ctx = Arc::new(create_node_proxy_client());

    let mut router = Router::new()
        .nest(REKCOD_SERVER_PREFIX_PATH, server::routers(Arc::clone(&ctx)))
        .nest(
            REKCOD_API_PREFIX_PATH,
            server::api_routers(Arc::clone(&ctx)),
        );

    let config = config::rekcod_server_config();
    if config.dashboard {
        router = router.merge(rekcod_dashboard::app_router(
            config.dashboard_base_url.as_deref(),
        ));
    }

    // this is maybe a problem in socketioxide
    // with axum router nest it will not work, maybe set a req path in axum router
    // see https://github.com/Totodore/socketioxide/issues/36
    router = router.layer(
        tower::ServiceBuilder::new()
            .layer(CorsLayer::permissive())
            .layer(socketio_routers()),
    );

    router
}

pub async fn init(_cancel: CancellationToken) -> anyhow::Result<()> {
    // init config
    init_rekcod_client_config().await?;
    // migrate db
    db::migrate().await?;
    // init app tmpl manager
    get_app_tmpl_manager().init().await?;
    Ok(())
}

async fn init_rekcod_client_config() -> anyhow::Result<()> {
    let config = config::rekcod_server_config();
    let config_dir = Path::new(&config.config_path);
    if !config_dir.exists() {
        tokio::fs::create_dir_all(config_dir).await?;
    }

    // check config file exists
    let cfg_path = config_dir.join(REKCOD_CONFIG_FILE_NAME);
    if cfg_path.exists() {
        // read token from file
        let cfg_str = tokio::fs::read_to_string(&cfg_path).await?;
        let c = serde_json::from_str::<RekcodCfg>(&cfg_str)?;
        set_token(c.token.clone());

        info!("init token success: {}", c.token);
        return Ok(());
    }

    let token = Uuid::new_v4().to_string();
    set_token(token.clone());
    let c = RekcodCfg {
        host: format!("127.0.0.1:{}", config.api_port),
        token: token.clone(),
    };

    tokio::fs::write(cfg_path, serde_json::to_string_pretty(&c)?).await?;
    info!("init token success: {}", token);
    Ok(())
}
