use axum::Json;
use rekcod_core::{
    api::{
        req::{NodeInfoRequest, NodeListRequest},
        resp::{ApiJsonResponse, NodeItemResponse},
    },
    http::ApiError,
};

use crate::node::node_manager;

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
