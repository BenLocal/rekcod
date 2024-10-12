use axum::{
    extract::Path,
    response::Response,
    routing::{any, get, post},
    Json, Router,
};
use bollard::{image::ImportImageOptions, secret::SystemInfo};
use futures::{StreamExt, TryStreamExt};
use hyper::StatusCode;
use rekcod_core::{
    api::{req::RegisterNodeRequest, resp::ApiJsonResponse},
    http::ApiError,
};
use serde::{Deserialize, Serialize};
use tokio_util::io::ReaderStream;
use tracing::info;

use crate::{
    db,
    node::{node_manager, Node},
};

pub fn routers() -> Router {
    Router::new()
        .route("/", get(|| async { "rekcod.server server" }))
        .route("/node/register", post(register_node))
        .route("/node/list", post(list_node))
        .route("/node/:node_name/docker/info", post(docker_info))
        .route(
            "/node/:node_name/docker/image/pull/:image_name",
            post(docker_image_pull),
        )
}

#[derive(Serialize, Deserialize, Default)]
#[serde(default)]
pub struct RegisterNodeResponse {}

async fn register_node(
    Json(req): Json<RegisterNodeRequest>,
) -> Result<Json<ApiJsonResponse<RegisterNodeResponse>>, ApiError> {
    let node = node_manager().get_node(&req.name).await?;

    let node_name = req.name.clone();
    if let Some(cache) = node {
        // check node has been registered and change
        let reg_node = Node::try_from(req)?;
        if !reg_node.eq(&cache.node) {
            // update node info
            info!("update node info: {:?}", reg_node);
            let repositry = db::repository().await;
            repositry
                .kvs
                .update_value(&db::kvs::KvsForDb {
                    module: "node".to_string(),
                    key: node_name.clone(),
                    value: serde_json::to_string(&reg_node)?,
                    ..Default::default()
                })
                .await?;

            // update cache
            node_manager().delete_node(&node_name).await?;
        }
    } else {
        // insert node info
        let repositry = db::repository().await;
        let reg_node = Node::try_from(req)?;
        info!("insert node info: {:?}", reg_node);
        let value = serde_json::to_string(&reg_node)?;
        // insert node info
        repositry
            .kvs
            .insert(&db::kvs::KvsForDb {
                module: "node".to_string(),
                key: node_name.clone(),
                value: value,
                ..Default::default()
            })
            .await?;

        // update cache
        node_manager().delete_node(&node_name).await?;
    }

    let resp = RegisterNodeResponse {};
    Ok(ApiJsonResponse::success(resp).into())
}

async fn list_node() -> Result<Json<ApiJsonResponse<Vec<Node>>>, ApiError> {
    info!("list node");
    let repositry = db::repository().await;
    let nodes = repositry
        .kvs
        .select("node", None, None, None)
        .await?
        .into_iter()
        .map(|kvs| Node::try_from(kvs).unwrap())
        .collect();

    Ok(ApiJsonResponse::success(nodes).into())
}

async fn docker_info(
    Path(node_name): Path<String>,
) -> Result<Json<ApiJsonResponse<SystemInfo>>, ApiError> {
    info!("docker info: {}", node_name);
    let n = node_manager().get_node(&node_name).await?;

    if let Some(n) = n {
        let docker_client = &n.docker;
        return Ok(ApiJsonResponse::success(docker_client.info().await?).into());
    }

    Ok(ApiJsonResponse::empty_success().into())
}

async fn docker_image_pull(
    Path((node_name, image_name)): Path<(String, String)>,
) -> Result<Response, ApiError> {
    info!("docker image pull: {}", node_name);

    let n = node_manager().get_node(&node_name).await?;

    if let Some(n) = n {
        let docker_client = &n.docker;
        let stream = docker_client
            .export_image(&image_name)
            .filter_map(|item| async {
                match item {
                    Ok(bytes) => {
                        println!("export info: {:?}", bytes.len());
                        Some(bytes)
                    }
                    Err(_) => None,
                }
            });

        let options = ImportImageOptions {
            ..Default::default()
        };

        let result = docker_client
            .import_image_stream(options, stream, None)
            .map(|res| match res {
                Ok(info) => {
                    println!("info: {:?}", info);
                    Ok(info.progress.unwrap_or("".to_string()).into_bytes())
                }
                Err(e) => {
                    println!("error: {}", e);
                    Err(std::io::Error::new(std::io::ErrorKind::Other, e))
                }
            });

        let body = axum::body::Body::from_stream(result);
        return Ok(Response::builder()
            .status(StatusCode::OK)
            .header(axum::http::header::CONTENT_TYPE, "application/octet-stream")
            .body(body)?);
    }

    Ok(Response::builder()
        .status(StatusCode::OK)
        .header(axum::http::header::CONTENT_TYPE, "application/octet-stream")
        .body(axum::body::Body::empty())?)
}
