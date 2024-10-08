use axum::Router;
use hyperlocal::UnixConnector;

const DOCKER_PROXY_PATH: &'static str = "/proxy.docker";

type UnixSocketClient = hyper_util::client::legacy::Client<UnixConnector, Body>;

pub fn routers() -> Router {
    let client: UnixSocketClient =
        hyper_util::client::legacy::Client::builder(TokioExecutor::new()).build(UnixConnector);

    Router::new()
        .route(
            &format!("{}/*path", DOCKER_PROXY_PATH),
            any(docker_proxy_handler),
        )
        .with_state(client)
}

async fn docker_proxy_handler(
    State(client): State<UnixSocketClient>,
    mut req: Request,
) -> Result<Response, StatusCode> {
    let path = req.uri().path();
    let path_query = req
        .uri()
        .path_and_query()
        .map(|v| {
            let without_prefix = v.as_str().strip_prefix(DOCKER_PROXY_PATH).unwrap_or(path);
            without_prefix
        })
        .unwrap_or(path);

    *req.uri_mut() = hyperlocal::Uri::new("/var/run/docker.sock", path_query).into();

    Ok(client
        .request(req)
        .await
        .map_err(|_| StatusCode::BAD_REQUEST)?
        .into_response())
}
