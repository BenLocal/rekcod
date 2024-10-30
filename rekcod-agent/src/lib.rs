use std::{
    pin::Pin,
    task::{Context, Poll},
};

use axum::{
    body::Body,
    extract::{Request, State},
    response::{IntoResponse as _, Response},
    routing::any,
    Router,
};

use job::{register, sys};
use pin_project_lite::pin_project;
use rekcod_core::constants::{DOCKER_PROXY_PATH, REKCOD_AGENT_PREFIX_PATH};

use tokio::io::{copy_bidirectional, AsyncRead, AsyncWrite, ReadBuf};
use tokio_util::sync::CancellationToken;

use crate::docker::DockerProxyInterface;
use docker::DockerProxyClient;
use hyper::{upgrade::Upgraded, StatusCode};

mod agent;
pub mod config;
mod docker;
mod job;

pub fn routers() -> Router {
    let client = DockerProxyClient::new();

    Router::new()
        .route(
            &format!("{}/*path", DOCKER_PROXY_PATH),
            any(docker_proxy_handler),
        )
        .with_state(client)
        .nest(REKCOD_AGENT_PREFIX_PATH, agent::routers())
}

async fn docker_proxy_handler(
    State(client): State<DockerProxyClient>,
    req: Request,
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

    let c = match client {
        #[cfg(unix)]
        DockerProxyClient::Unix(c) => c,
        #[cfg(windows)]
        DockerProxyClient::Windows(c) => c,
    };

    // Check for WebSocket upgrade request
    let is_websocket = req
        .headers()
        .get(hyper::header::CONNECTION)
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_lowercase().contains("upgrade"))
        .unwrap_or(false);

    let uri = c
        .uri(path_query)
        .map_err(|_| StatusCode::BAD_REQUEST)?
        .clone();
    if is_websocket {
        let (mut parts, body) = req.into_parts();
        let cache_req = Request::from_parts(parts.clone(), Body::empty());
        parts.uri = uri;
        let proxy_req = Request::from_parts(parts, body);

        let response = c
            .request(proxy_req)
            .await
            .map_err(|_| StatusCode::BAD_REQUEST)?;
        if response.status() == StatusCode::SWITCHING_PROTOCOLS {
            tokio::spawn(async move {
                let mut out = AsyncUpgraded::new(hyper::upgrade::on(cache_req).await.unwrap());
                let mut inv = AsyncUpgraded::new(hyper::upgrade::on(response).await.unwrap());
                let _ = copy_bidirectional(&mut inv, &mut out).await;
            });
            Ok(StatusCode::SWITCHING_PROTOCOLS.into_response())
        } else {
            Ok(response.into_response())
        }
    } else {
        let (mut parts, body) = req.into_parts();
        parts.uri = uri;
        let proxy_req = Request::from_parts(parts, body);
        Ok(c.request(proxy_req)
            .await
            .map_err(|_| StatusCode::BAD_REQUEST)?
            .into_response())
    }
}

pub async fn init(cancel: CancellationToken) -> anyhow::Result<()> {
    let cancel_clone = cancel.clone();
    tokio::spawn(async move {
        let cancel_clone_end = cancel_clone.clone();
        if let Err(e) = register::register_node(cancel_clone).await {
            println!("agent register error: {}", e);
            cancel_clone_end.cancel();
        }
    });

    let cancel_clone = cancel.clone();
    tokio::spawn(async move {
        let cancel_clone_end = cancel_clone.clone();
        if let Err(e) = sys::sys_monitor(cancel_clone).await {
            println!("sys monitor error: {}", e);
            cancel_clone_end.cancel();
        }
    });

    Ok(())
}

pin_project! {
    #[derive(Debug)]
    pub(crate) struct AsyncUpgraded {
        #[pin]
        inner: Upgraded,
    }
}

impl AsyncUpgraded {
    pub(crate) fn new(upgraded: Upgraded) -> Self {
        Self { inner: upgraded }
    }
}

impl AsyncRead for AsyncUpgraded {
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        read_buf: &mut ReadBuf<'_>,
    ) -> Poll<std::io::Result<()>> {
        let n = {
            let mut hbuf = hyper::rt::ReadBuf::new(read_buf.initialize_unfilled());
            match hyper::rt::Read::poll_read(self.project().inner, cx, hbuf.unfilled()) {
                Poll::Ready(Ok(())) => hbuf.filled().len(),
                other => return other,
            }
        };
        read_buf.advance(n);

        Poll::Ready(Ok(()))
    }
}

impl AsyncWrite for AsyncUpgraded {
    fn poll_write(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<Result<usize, std::io::Error>> {
        hyper::rt::Write::poll_write(self.project().inner, cx, buf)
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), std::io::Error>> {
        hyper::rt::Write::poll_flush(self.project().inner, cx)
    }

    fn poll_shutdown(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<Result<(), std::io::Error>> {
        hyper::rt::Write::poll_shutdown(self.project().inner, cx)
    }
}
