use std::{collections::HashMap, sync::Arc};

use axum::{
    body::Body,
    extract::{Path, Query},
    response::Response,
    Json,
};
use bollard::{
    container::{
        InspectContainerOptions, ListContainersOptions, LogsOptions, RemoveContainerOptions,
    },
    image::{CreateImageOptions, ImportImageOptions, ListImagesOptions},
    network::ListNetworksOptions,
    secret::{
        ContainerInspectResponse, ContainerSummary, ImageSummary, Network, SystemInfo,
        VolumeListResponse,
    },
    volume::ListVolumesOptions,
    Docker,
};
use futures::{Stream, StreamExt as _};
use hyper::{header, StatusCode};
use rekcod_core::{
    api::{req::NodeDockerQueryRequest, resp::ApiJsonResponse},
    http::ApiError,
};
use tracing::info;

use crate::node::{node_manager, NodeState};

macro_rules! get_state {
    ($name:expr) => {
        node_manager()
            .get_node(&$name)
            .await?
            .ok_or_else(|| anyhow::anyhow!("node {} not found", &$name))?
    };
}

macro_rules! docker_exec {
    ($exec:expr) => {
        Ok(ApiJsonResponse::success($exec?).into())
    };
}

pub async fn docker_image_list_by_node(
    Query(query): Query<NodeDockerQueryRequest>,
) -> Result<Json<ApiJsonResponse<Vec<ImageSummary>>>, ApiError> {
    let state = get_state!(query.node_name);

    let options = Some(ListImagesOptions::<&str> {
        all: true,
        ..Default::default()
    });

    docker_exec!(state.docker.list_images(options).await)
}

pub async fn docker_info_by_node(
    Query(query): Query<NodeDockerQueryRequest>,
) -> Result<Json<ApiJsonResponse<SystemInfo>>, ApiError> {
    let state = get_state!(query.node_name);
    docker_exec!(state.docker.info().await)
}

pub async fn docker_container_start_by_node(
    Query(query): Query<NodeDockerQueryRequest>,
    Path(id): Path<String>,
) -> Result<Json<ApiJsonResponse<()>>, ApiError> {
    let state = get_state!(query.node_name);
    docker_exec!(state.docker.start_container::<&str>(&id, None).await)
}

pub async fn docker_container_stop_by_node(
    Query(query): Query<NodeDockerQueryRequest>,
    Path(id): Path<String>,
) -> Result<Json<ApiJsonResponse<()>>, ApiError> {
    let state = get_state!(query.node_name);
    docker_exec!(state.docker.stop_container(&id, None).await)
}

pub async fn docker_container_logs_by_node(
    Query(query): Query<NodeDockerQueryRequest>,
    Path(id): Path<String>,
) -> Result<Response, ApiError> {
    let state = get_state!(query.node_name);
    let stream = state
        .docker
        .logs(
            &id,
            Some(LogsOptions {
                follow: true,
                stdout: true,
                stderr: true,
                tail: "100".to_string(),
                ..Default::default()
            }),
        )
        .map(|res| match res {
            Ok(info) => Ok(info.into_bytes()),
            Err(e) => Err(std::io::Error::new(std::io::ErrorKind::Other, e)),
        });
    return Ok(Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, "application/octet-stream")
        .body(Body::from_stream(stream))?);
}

pub async fn docker_container_info_by_node(
    Query(query): Query<NodeDockerQueryRequest>,
    Path(id): Path<String>,
) -> Result<Json<ApiJsonResponse<ContainerInspectResponse>>, ApiError> {
    let state = get_state!(query.node_name);
    let options = Some(InspectContainerOptions {
        size: true,
        ..Default::default()
    });
    docker_exec!(state.docker.inspect_container(&id, options).await)
}

pub async fn docker_container_delete_by_node(
    Query(query): Query<NodeDockerQueryRequest>,
    Path(id): Path<String>,
) -> Result<Json<ApiJsonResponse<()>>, ApiError> {
    let state = get_state!(query.node_name);
    let options = Some(RemoveContainerOptions {
        force: true,
        ..Default::default()
    });
    docker_exec!(state.docker.remove_container(&id, options).await)
}

pub async fn docker_container_restart_by_node(
    Query(query): Query<NodeDockerQueryRequest>,
    Path(id): Path<String>,
) -> Result<Json<ApiJsonResponse<()>>, ApiError> {
    let state = get_state!(query.node_name);
    docker_exec!(state.docker.restart_container(&id, None).await)
}

pub async fn docker_container_list_by_node(
    Query(query): Query<NodeDockerQueryRequest>,
) -> Result<Json<ApiJsonResponse<Vec<ContainerSummary>>>, ApiError> {
    let state = get_state!(query.node_name);
    let options = Some(ListContainersOptions::<&str> {
        all: true,
        ..Default::default()
    });
    docker_exec!(state.docker.list_containers(options).await)
}

pub async fn docker_network_list_by_node(
    Query(query): Query<NodeDockerQueryRequest>,
) -> Result<Json<ApiJsonResponse<Vec<Network>>>, ApiError> {
    let state = get_state!(query.node_name);
    let options = Some(ListNetworksOptions::<&str> {
        ..Default::default()
    });
    docker_exec!(state.docker.list_networks(options).await)
}

pub async fn docker_volume_list_by_node(
    Query(query): Query<NodeDockerQueryRequest>,
) -> Result<Json<ApiJsonResponse<VolumeListResponse>>, ApiError> {
    let state = get_state!(query.node_name);
    let options = Some(ListVolumesOptions::<&str> {
        ..Default::default()
    });
    docker_exec!(state.docker.list_volumes(options).await)
}

pub async fn docker_image_pull_auto(
    Path((node_name, image_name)): Path<(String, String)>,
) -> Result<Response, ApiError> {
    info!("docker image pull: {}", node_name);

    // first need check image exists
    // if some docker server has the image, will use it
    // if not, will pull from docker hub or registry server
    let all = node_manager().get_all_nodes(false).await?;
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
