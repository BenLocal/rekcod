use axum::{routing::get, Router};

pub fn routers() -> Router {
    Router::new().route("/", get(|| async { "rekcod.agent agent" }))
}
