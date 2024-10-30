use std::sync::Arc;

use axum::{
    body::Body,
    extract::{Path, Request, State},
    response::{IntoResponse, Response},
};
use hyper::{StatusCode, Uri};
use hyper_util::{client::legacy::connect::HttpConnector, rt::TokioExecutor};
use rekcod_core::{
    auth::header_value_token,
    constants::{REKCOD_AGENT_PREFIX_PATH, REKCOD_API_NODE_NAME_HEADER_KEY, TOEKN_HEADER_KEY},
};
use tracing::error;

use crate::node::node_manager;

pub type NodeProxyClient = hyper_util::client::legacy::Client<HttpConnector, Body>;

pub fn create_node_proxy_client() -> NodeProxyClient {
    hyper_util::client::legacy::Client::<(), ()>::builder(TokioExecutor::new())
        .build(HttpConnector::new())
}

pub async fn node_proxy_handler(
    State(ctx): State<Arc<NodeProxyClient>>,
    Path(_sub): Path<String>,
    mut req: Request,
) -> Result<Response, StatusCode> {
    let path = req.uri().path();
    let path_query = req
        .uri()
        .path_and_query()
        .map(|v| extract_path("/node/proxy", v.as_str()))
        .unwrap_or(path.to_string());

    let node_name = req
        .headers()
        .get(REKCOD_API_NODE_NAME_HEADER_KEY)
        .map(|x| x.to_str())
        .transpose()
        .map_err(|_| StatusCode::BAD_REQUEST)?
        .ok_or(StatusCode::BAD_REQUEST)?;

    let node = node_manager()
        .get_node(&node_name)
        .await
        .map_err(|e| {
            error!("node {} not found: {}", node_name, e);
            StatusCode::BAD_REQUEST
        })?
        .ok_or(StatusCode::BAD_REQUEST)?;

    let uri = format!(
        "http://{}:{}{}{}",
        node.node.ip, node.node.port, REKCOD_AGENT_PREFIX_PATH, path_query
    );

    *req.uri_mut() = Uri::try_from(uri).map_err(|_| StatusCode::BAD_REQUEST)?;
    (*req.headers_mut()).insert(TOEKN_HEADER_KEY, header_value_token());
    Ok(ctx
        .request(req)
        .await
        .map_err(|_| StatusCode::BAD_REQUEST)?
        .into_response())
}

fn extract_path(prefix: &str, input: &str) -> String {
    if let Some(index) = input.find(prefix) {
        let start_index = index + prefix.len();
        input[start_index..].to_string()
    } else {
        input.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_path() {
        assert_eq!(extract_path("/node/proxy", "/node/proxy"), "");
        assert_eq!(extract_path("/node/proxy", "/node/proxy/"), "/");
        assert_eq!(extract_path("/node/proxy", "/node/proxy/1"), "/1");
        assert_eq!(extract_path("/node/proxy", "/node/proxy/1/2"), "/1/2");
        assert_eq!(extract_path("/node/proxy", "/aaa/node/proxy/1/2"), "/1/2");
        assert_eq!(extract_path("/node/proxy", "/bbb/node/proxy/1/2"), "/1/2");
    }
}
