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
    api::{
        req::{NodeDockerQueryRequest, NodeInfoRequest, NodeListRequest, NodeSysInfoRequest},
        resp::{ApiJsonResponse, NodeItemResponse, SystemInfoResponse},
    },
    client::get_client,
    http::ApiError,
};
use tracing::info;

use crate::node::{node_manager, NodeState};

pub async fn list_node(
    Json(req): Json<NodeListRequest>,
) -> Result<Json<ApiJsonResponse<Vec<NodeItemResponse>>>, ApiError> {
    let nodes = node_manager()
        .get_all_nodes(req.all)
        .await?
        .into_iter()
        .map(|ns| ns.node.clone().into())
        .collect();

    Ok(ApiJsonResponse::success(nodes).into())
}

pub async fn info_node(
    Json(req): Json<NodeInfoRequest>,
) -> Result<Json<ApiJsonResponse<NodeItemResponse>>, ApiError> {
    let node = node_manager()
        .get_node(&req.name)
        .await?
        .map(|ns| ns.node.clone().into());

    Ok(ApiJsonResponse::success_optional(node).into())
}

pub async fn node_sys_info(
    Json(req): Json<NodeSysInfoRequest>,
) -> Result<Json<ApiJsonResponse<SystemInfoResponse>>, ApiError> {
    let n = node_manager().get_node(&req.name).await?;

    if let Some(n) = n {
        let res = get_client()?
            .get(format!("{}/sys", n.get_node_agent()))
            .send()
            .await?;

        return Ok(res
            .json::<ApiJsonResponse<SystemInfoResponse>>()
            .await?
            .into());
    }

    Ok(ApiJsonResponse::empty_success().into())
}

pub async fn docker_image_list_by_node(
    Query(query): Query<NodeDockerQueryRequest>,
) -> Result<Json<ApiJsonResponse<Vec<ImageSummary>>>, ApiError> {
    info!("docker image list: {:?}", query);
    let n = node_manager().get_node(&query.node_name).await?;

    let options = Some(ListImagesOptions::<&str> {
        all: true,
        ..Default::default()
    });

    if let Some(n) = n {
        let docker_client = &n.docker;
        return Ok(ApiJsonResponse::success(docker_client.list_images(options).await?).into());
    }

    Ok(ApiJsonResponse::empty_success().into())
}

pub async fn docker_info_by_node(
    Query(query): Query<NodeDockerQueryRequest>,
) -> Result<Json<ApiJsonResponse<SystemInfo>>, ApiError> {
    info!("docker info: {:?}", query);
    let n = node_manager().get_node(&query.node_name).await?;

    if let Some(n) = n {
        let docker_client = &n.docker;
        return Ok(ApiJsonResponse::success(docker_client.info().await?).into());
    }

    Ok(ApiJsonResponse::empty_success().into())
}

pub async fn docker_container_start_by_node(
    Query(query): Query<NodeDockerQueryRequest>,
    Path(id): Path<String>,
) -> Result<Json<ApiJsonResponse<()>>, ApiError> {
    info!("docker container start: {:?}", query);
    let n = node_manager().get_node(&query.node_name).await?;

    if let Some(n) = n {
        let docker_client = &n.docker;
        docker_client.start_container::<&str>(&id, None).await?;
        return Ok(ApiJsonResponse::success(()).into());
    }

    Ok(ApiJsonResponse::empty_success().into())
}

pub async fn docker_container_stop_by_node(
    Query(query): Query<NodeDockerQueryRequest>,
    Path(id): Path<String>,
) -> Result<Json<ApiJsonResponse<()>>, ApiError> {
    info!("docker container stop: {:?}", query);
    let n = node_manager().get_node(&query.node_name).await?;

    if let Some(n) = n {
        let docker_client = &n.docker;
        docker_client.stop_container(&id, None).await?;
        return Ok(ApiJsonResponse::success(()).into());
    }

    Ok(ApiJsonResponse::empty_success().into())
}

pub async fn docker_container_logs_by_node(
    Query(query): Query<NodeDockerQueryRequest>,
    Path(id): Path<String>,
) -> Result<Response, ApiError> {
    let n = node_manager().get_node(&query.node_name).await?;
    if let Some(n) = n {
        let docker_client = &n.docker;
        let stream = docker_client
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

    Ok(Response::builder()
        .status(StatusCode::NOT_FOUND)
        .body(Body::empty())?)
}

pub async fn docker_container_info_by_node(
    Query(query): Query<NodeDockerQueryRequest>,
    Path(id): Path<String>,
) -> Result<Json<ApiJsonResponse<ContainerInspectResponse>>, ApiError> {
    info!("docker container restart: {:?}", query);
    let n = node_manager().get_node(&query.node_name).await?;

    if let Some(n) = n {
        let docker_client = &n.docker;
        let options = Some(InspectContainerOptions {
            size: true,
            ..Default::default()
        });
        let resp = docker_client.inspect_container(&id, options).await?;
        return Ok(ApiJsonResponse::success(resp).into());
    }

    Ok(ApiJsonResponse::empty_success().into())
}

pub async fn docker_container_delete_by_node(
    Query(query): Query<NodeDockerQueryRequest>,
    Path(id): Path<String>,
) -> Result<Json<ApiJsonResponse<()>>, ApiError> {
    info!("docker container restart: {:?}", query);
    let n = node_manager().get_node(&query.node_name).await?;

    if let Some(n) = n {
        let docker_client = &n.docker;
        let options = Some(RemoveContainerOptions {
            force: true,
            ..Default::default()
        });
        docker_client.remove_container(&id, options).await?;
        return Ok(ApiJsonResponse::success(()).into());
    }

    Ok(ApiJsonResponse::empty_success().into())
}

pub async fn docker_container_restart_by_node(
    Query(query): Query<NodeDockerQueryRequest>,
    Path(id): Path<String>,
) -> Result<Json<ApiJsonResponse<()>>, ApiError> {
    info!("docker container restart: {:?}", query);
    let n = node_manager().get_node(&query.node_name).await?;

    if let Some(n) = n {
        let docker_client = &n.docker;
        docker_client.restart_container(&id, None).await?;
        return Ok(ApiJsonResponse::success(()).into());
    }

    Ok(ApiJsonResponse::empty_success().into())
}

pub async fn docker_container_list_by_node(
    Query(query): Query<NodeDockerQueryRequest>,
) -> Result<Json<ApiJsonResponse<Vec<ContainerSummary>>>, ApiError> {
    info!("docker container list: {:?}", query);
    let n = node_manager().get_node(&query.node_name).await?;

    if let Some(n) = n {
        let options = Some(ListContainersOptions::<&str> {
            all: true,
            ..Default::default()
        });

        let docker_client = &n.docker;
        return Ok(ApiJsonResponse::success(docker_client.list_containers(options).await?).into());
    }

    Ok(ApiJsonResponse::empty_success().into())
}

pub async fn docker_network_list_by_node(
    Query(query): Query<NodeDockerQueryRequest>,
) -> Result<Json<ApiJsonResponse<Vec<Network>>>, ApiError> {
    info!("docker network list: {:?}", query);
    let n = node_manager().get_node(&query.node_name).await?;

    if let Some(n) = n {
        let options = Some(ListNetworksOptions::<&str> {
            ..Default::default()
        });

        let docker_client = &n.docker;
        return Ok(ApiJsonResponse::success(docker_client.list_networks(options).await?).into());
    }

    Ok(ApiJsonResponse::empty_success().into())
}

pub async fn docker_volume_list_by_node(
    Query(query): Query<NodeDockerQueryRequest>,
) -> Result<Json<ApiJsonResponse<VolumeListResponse>>, ApiError> {
    info!("docker network list: {:?}", query);
    let n = node_manager().get_node(&query.node_name).await?;

    if let Some(n) = n {
        let options = Some(ListVolumesOptions::<&str> {
            ..Default::default()
        });

        let docker_client = &n.docker;
        return Ok(ApiJsonResponse::success(docker_client.list_volumes(options).await?).into());
    }

    Ok(ApiJsonResponse::empty_success().into())
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
