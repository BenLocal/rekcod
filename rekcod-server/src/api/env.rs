use std::collections::HashMap;

use axum::{debug_handler, Json};
use rekcod_core::{
    api::{
        req::EnvRequest,
        resp::{ApiJsonResponse, EnvResponse},
    },
    http::ApiError,
};

use crate::db;

pub async fn get_global_env() -> Result<Json<ApiJsonResponse<EnvResponse>>, ApiError> {
    let db = db::repository().await;
    let env_db = db.kvs.select_one("env", Some("global"), None, None).await?;

    let env = env_db.map(|env| env.value).map_or("".to_string(), |v| v);
    Ok(ApiJsonResponse::success(EnvResponse { values: env }).into())
}

#[debug_handler]
pub async fn set_global_env(
    Json(req): Json<EnvRequest>,
) -> Result<Json<ApiJsonResponse<()>>, ApiError> {
    let db = db::repository().await;
    db.kvs
        .insert_or_update_value(&db::kvs::KvsForDb {
            module: "env".to_string(),
            key: "global".to_string(),
            sub_key: "".to_string(),
            value: req.values,
            ..Default::default()
        })
        .await?;
    Ok(ApiJsonResponse::success(()).into())
}
