use std::{collections::HashMap, sync::Arc};

use axum::{
    body::Body,
    extract::{Path, Request, State},
    middleware,
    response::{IntoResponse as _, Response},
    routing::{any, get, post},
    Json, Router,
};
use bollard::{
    image::{CreateImageOptions, ImportImageOptions, ListImagesOptions},
    secret::SystemInfo,
    Docker,
};
use futures::{Stream, StreamExt};
use hyper::{StatusCode, Uri};
use hyper_util::{client::legacy::connect::HttpConnector, rt::TokioExecutor};
use rekcod_core::{
    api::{req::RegisterNodeRequest, resp::ApiJsonResponse},
    auth::token_auth,
    constants::{DOCKER_PROXY_PATH, REKCOD_AGENT_PREFIX_PATH},
    http::ApiError,
};
use serde::{Deserialize, Serialize};
use tracing::{error, info};

use crate::{
    db,
    node::{node_manager, Node, NodeState},
};

type Client = hyper_util::client::legacy::Client<HttpConnector, Body>;

pub fn routers() -> Router {
    let client: Client =
        hyper_util::client::legacy::Client::<(), ()>::builder(TokioExecutor::new())
            .build(HttpConnector::new());

    let ctx = Arc::new(client);

    Router::new()
        .route("/", get(|| async { "rekcod.server server" }))
        .route("/node/register", post(register_node))
        .route("/node/list", post(list_node))
        .route("/node/:node_name/docker/info", post(docker_info))
        .route(
            "/node/:node_name/docker/image/pull/:image_name",
            post(docker_image_pull),
        )
        .route("/node/:node_name/:typ/*sub", any(node_proxy))
        .with_state(Arc::clone(&ctx))
        .layer(middleware::from_fn(token_auth))
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
                    sub_key: if reg_node.status { "online" } else { "offline" }.to_string(),
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
                sub_key: if reg_node.status { "online" } else { "offline" }.to_string(),
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

    // first need check image exists
    // if some docker server has the image, will use it
    // if not, will pull from docker hub or registry server
    let all = node_manager().get_all_nodes().await?;
    let n = node_manager().get_node(&node_name).await?;
    let src = select_has_docker_image_node(all, &image_name, &node_name).await;
    if let Some(n) = n {
        if let Some(src) = src {
            let result =
                docker_pull_image_from_other_server(&n.docker, &image_name, &src.docker).await?;
            let body = axum::body::Body::from_stream(result);
            return Ok(Response::builder()
                .status(StatusCode::OK)
                .header(axum::http::header::CONTENT_TYPE, "application/octet-stream")
                .body(body)?);
        }

        let image_name = image_name.clone();
        let docker = Arc::clone(&n).docker.clone();
        let result = docker_pull_image_from_hub(docker, image_name).await?;
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

async fn select_has_docker_image_node(
    nodes: Vec<Arc<NodeState>>,
    image_name: &str,
    expect_name: &str,
) -> Option<Arc<NodeState>> {
    for x in nodes.iter().filter(|x| x.node.name != expect_name) {
        let list_options = ListImagesOptions {
            all: true,
            filters: HashMap::from([("reference", vec![image_name])]),
            ..Default::default()
        };
        let images = x.docker.list_images(Some(list_options)).await;

        if images.is_ok() && images.unwrap().len() > 0 {
            info!("select node {} has image {}", x.node.name, image_name);
            return Some(x.clone());
        }
    }

    None
}

async fn docker_pull_image_from_hub<'a>(
    docker: Docker,
    image_name: String,
) -> anyhow::Result<impl Stream<Item = Result<Vec<u8>, std::io::Error>> + 'a> {
    info!("docker image pull");
    let options = Some(CreateImageOptions {
        from_image: image_name,
        ..Default::default()
    });

    let res = docker
        .create_image(options, None, None)
        .map(|res| match res {
            Ok(info) => Ok(info.progress.unwrap_or("".to_string()).into_bytes()),
            Err(e) => Err(std::io::Error::new(std::io::ErrorKind::Other, e)),
        });

    Ok(res)
}

async fn docker_pull_image_from_other_server(
    docker: &Docker,
    image_name: &str,
    src_docker: &Docker,
) -> anyhow::Result<impl Stream<Item = Result<Vec<u8>, std::io::Error>>> {
    // let export_docker_client = rekcod_connect(
    //     Some(format!("http://{}:{}", "39.100.74.178", 6734)),
    //     rekcod_core::constants::DOCKER_PROXY_PATH,
    //     4,
    // )?;
    let stream = src_docker
        .export_image(&image_name)
        .filter_map(|item| async {
            match item {
                Ok(bytes) => Some(bytes),
                Err(_) => None,
            }
        });

    // import image
    let options = ImportImageOptions {
        ..Default::default()
    };
    let result = docker
        .import_image_stream(options, stream, None)
        .map(|res| match res {
            Ok(info) => Ok(format!("{}\n", info.progress.unwrap_or("".to_string())).into_bytes()),
            Err(e) => Err(std::io::Error::new(std::io::ErrorKind::Other, e)),
        });

    Ok(result)
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
    async fn test_list_iamge() -> anyhow::Result<()> {
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
