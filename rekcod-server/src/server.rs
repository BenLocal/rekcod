use std::sync::Arc;

use axum::{
    body::Body,
    extract::{Path, Request, State},
    middleware,
    response::{IntoResponse as _, Response},
    routing::{any, post},
    Json, Router,
};
use hyper::{StatusCode, Uri};
use hyper_util::{client::legacy::connect::HttpConnector, rt::TokioExecutor};
use rekcod_core::{
    api::{req::RegisterNodeRequest, resp::ApiJsonResponse},
    auth::token_auth,
    constants::{DOCKER_PROXY_PATH, REKCOD_AGENT_PREFIX_PATH},
    http::ApiError,
    obj::NodeStatus,
};
use serde::{Deserialize, Serialize};
use tracing::{error, info};

use crate::{
    api::app::{
        docker_container_delete_by_node, docker_container_info_by_node,
        docker_container_list_by_node, docker_container_logs_by_node,
        docker_container_restart_by_node, docker_container_start_by_node,
        docker_container_stop_by_node, docker_image_list_by_node, docker_image_pull_auto,
        docker_info_by_node, docker_network_list_by_node, docker_volume_list_by_node, info_node,
        list_node,
    },
    db,
    node::{node_manager, Node},
};

type Client = hyper_util::client::legacy::Client<HttpConnector, Body>;

pub fn api_routers() -> Router {
    Router::new()
        .route("/node/list", post(list_node))
        .route("/node/info", post(info_node))
        .route("/node/docker/info", post(docker_info_by_node))
        .route(
            "/node/docker/container/list",
            post(docker_container_list_by_node),
        )
        .route(
            "/node/docker/container/start/:id",
            post(docker_container_start_by_node),
        )
        .route(
            "/node/docker/container/stop/:id",
            post(docker_container_stop_by_node),
        )
        .route(
            "/node/docker/container/restart/:id",
            post(docker_container_restart_by_node),
        )
        .route(
            "/node/docker/container/logs/:id",
            post(docker_container_logs_by_node),
        )
        .route(
            "/node/docker/container/delete/:id",
            post(docker_container_delete_by_node),
        )
        .route(
            "/node/docker/container/inspect/:id",
            post(docker_container_info_by_node),
        )
        .route("/node/docker/image/list", post(docker_image_list_by_node))
        .route("/node/docker/image/pull_auto", post(docker_image_pull_auto))
        .route(
            "/node/docker/network/list",
            post(docker_network_list_by_node),
        )
        .route("/node/docker/volume/list", post(docker_volume_list_by_node))
}

pub fn routers() -> Router {
    let client: Client =
        hyper_util::client::legacy::Client::<(), ()>::builder(TokioExecutor::new())
            .build(HttpConnector::new());

    let ctx = Arc::new(client);

    Router::new()
        .route("/node/register", post(register_node))
        .route("/node/:node_name/:typ/*sub", any(node_proxy))
        .with_state(Arc::clone(&ctx))
        .layer(middleware::from_fn(token_auth))
        .merge(api_routers())
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
                    sub_key: if reg_node.status {
                        NodeStatus::Online
                    } else {
                        NodeStatus::Offline
                    }
                    .to_string(),
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
                sub_key: if reg_node.status {
                    NodeStatus::Online
                } else {
                    NodeStatus::Offline
                }
                .to_string(),
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

async fn node_proxy(
    State(ctx): State<Arc<Client>>,
    Path((node_name, typ, _sub)): Path<(String, String, String)>,
    mut req: Request,
) -> Result<Response, StatusCode> {
    let path = req.uri().path();
    let path_query = req
        .uri()
        .path_and_query()
        .map(|v| {
            let without_prefix = v
                .as_str()
                .strip_prefix(format!("/node/{}/{}", node_name, typ).as_str())
                .unwrap_or(path);
            without_prefix
        })
        .unwrap_or(path);

    let node = node_manager()
        .get_node(&node_name)
        .await
        .map_err(|e| {
            error!("node {} not found: {}", node_name, e);
            StatusCode::BAD_REQUEST
        })?
        .ok_or(StatusCode::BAD_REQUEST)?;

    let s = match typ.as_str() {
        "proxy.docker" => DOCKER_PROXY_PATH,
        "proxy.base" => REKCOD_AGENT_PREFIX_PATH,
        _ => "",
    };

    let uri = format!(
        "http://{}:{}{}{}",
        node.node.ip, node.node.port, s, path_query
    );

    *req.uri_mut() = Uri::try_from(uri).map_err(|_| StatusCode::BAD_REQUEST)?;

    Ok(ctx
        .request(req)
        .await
        .map_err(|_| StatusCode::BAD_REQUEST)?
        .into_response())
}

#[cfg(test)]
mod tests {
    use std::{collections::HashMap, io::Write as _};

    use bollard::{
        image::{ImportImageOptions, ListImagesOptions, SearchImagesOptions},
        Docker,
    };
    use futures::StreamExt as _;
    use rekcod_core::{auth::get_token, docker::rekcod_connect};
    use tokio::fs::File;
    use tokio_util::codec;

    #[tokio::test]
    async fn test_export_image() -> anyhow::Result<()> {
        let docker_client = rekcod_connect(
            Some(format!("http://{}:{}", "39.100.74.178", 6734)),
            rekcod_core::constants::DOCKER_PROXY_PATH,
            40,
            get_token(),
        )?;

        let mut stream = docker_client.export_image("busybox:latest");

        // write to file
        let mut file = std::fs::File::create("image.tar").unwrap();
        while let Some(res) = stream.next().await {
            file.write_all(&res.unwrap()).unwrap();
        }

        // check
        assert!(file.metadata().unwrap().len() > 0);

        let current = Docker::connect_with_defaults()?;
        // import image
        let options = ImportImageOptions {
            ..Default::default()
        };
        let file = File::open("image.tar").await?;
        let byte_stream =
            codec::FramedRead::new(file, codec::BytesCodec::new()).map(|r| r.unwrap().freeze());
        let _stream =
            current
                .import_image_stream(options, byte_stream, None)
                .map(|res| match res {
                    Ok(info) => Ok(info.progress.unwrap_or("".to_string()).into_bytes()),
                    Err(e) => {
                        println!("import error: {:?}", e);
                        Err(std::io::Error::new(std::io::ErrorKind::Other, e))
                    }
                });

        // delete file
        std::fs::remove_file("image.tar").unwrap();
        Ok(())
    }

    #[tokio::test]
    async fn test_search_image() -> anyhow::Result<()> {
        let docker_client = rekcod_connect(
            Some(format!("http://{}:{}", "39.100.74.178", 6734)),
            rekcod_core::constants::DOCKER_PROXY_PATH,
            40,
            get_token(),
        )?;

        let search_options = SearchImagesOptions {
            term: "busybox:latest".to_string(),
            ..Default::default()
        };

        // Search for an image on Docker Hub
        let res = docker_client.search_images(search_options).await?;
        assert_eq!(res.len(), 1);

        Ok(())
    }

    #[tokio::test]
    async fn test_list_image() -> anyhow::Result<()> {
        let docker_client = rekcod_connect(
            Some(format!("http://{}:{}", "39.100.74.178", 6734)),
            rekcod_core::constants::DOCKER_PROXY_PATH,
            40,
            get_token(),
        )?;

        let mut filters = HashMap::new();
        filters.insert("reference", vec!["busybox:latest"]);

        let search_options = ListImagesOptions {
            all: true,
            filters,
            ..Default::default()
        };

        let res = docker_client.list_images(Some(search_options)).await?;

        assert_eq!(res.len(), 1);
        Ok(())
    }
}
