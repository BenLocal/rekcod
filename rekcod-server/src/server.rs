use axum::{
    routing::{get, post},
    Json, Router,
};
use rekcod_core::{
    api::{req::RegisterNodeRequest, resp::ApiJsonResponse},
    http::ApiError,
};
use serde::{Deserialize, Serialize};
use tracing::info;

use crate::db;

pub fn routers() -> Router {
    Router::new()
        .route("/", get(|| async { "rekcod.server server" }))
        .route("/node/register", post(register_node))
}

#[derive(Serialize, Deserialize, Default)]
#[serde(default)]
pub struct RegisterNodeResponse {}

async fn register_node(
    Json(req): Json<RegisterNodeRequest>,
) -> Result<Json<ApiJsonResponse<RegisterNodeResponse>>, ApiError> {
    info!("register node: {:?}", req);
    let repositry = db::repository().await;

    let value = serde_json::to_string(&req)?;
    // insert node info
    repositry
        .kvs
        .insert(&db::kvs::KvsForDb {
            id: 0,
            module: "node".to_string(),
            key: req.name,
            sub_key: "".to_string(),
            third_key: "".to_string(),
            value: value,
        })
        .await?;

    let resp = RegisterNodeResponse {};
    Ok(ApiJsonResponse::success(resp).into())
}
