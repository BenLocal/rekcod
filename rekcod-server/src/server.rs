use axum::{
    routing::{get, post},
    Json, Router,
};
use rekcod_core::http::ApiError;
use serde::{Deserialize, Serialize};
use tracing::info;

pub fn routers() -> Router {
    Router::new()
        .route("/", get(|| async { "rekcod.server server" }))
        .route("/node/register", post(register_node))
}

#[derive(Serialize, Deserialize, Default)]
#[serde(default)]
pub struct RegisterNodeResponse {}

#[derive(Serialize, Deserialize, Default, Debug)]
#[serde(default)]
pub struct RegisterNodeRequest {
    pub host_name: String,
    pub ip: String,
}

async fn register_node(
    Json(req): Json<RegisterNodeRequest>,
) -> Result<Json<RegisterNodeResponse>, ApiError> {
    info!("register node: {:?}", req);
    Ok(Json(RegisterNodeResponse {}))
}
