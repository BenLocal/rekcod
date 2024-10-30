use std::{collections::HashMap, path::Path};

use axum::{
    body::Body,
    middleware,
    response::Response,
    routing::{get, post},
    Json, Router,
};
use futures::TryStreamExt as _;
use http_range::HttpRange;
use hyper::{HeaderMap, StatusCode};
use rekcod_core::{
    api::resp::{ApiJsonResponse, SystemInfoResponse},
    auth::token_auth,
    http::ApiError,
};
use serde::{Deserialize, Serialize};
use tokio::{
    fs::File,
    io::{AsyncReadExt as _, AsyncSeekExt as _, BufWriter},
};
use tokio_util::io::{ReaderStream, StreamReader};

use crate::{config, job::sys::sys_info};

pub fn routers() -> Router {
    Router::new()
        .route("/upload", get(upload_file))
        .route("/download", post(download_file))
        .route("/download_range", get(download_range_file))
        .route("/shell", post(shell_stream))
        .route("/sys", get(get_sys_info))
        .route("/", get(|| async { "rekcod.agent agent" }))
        .layer(middleware::from_fn(token_auth))
}

#[derive(Deserialize, Serialize, Debug)]
pub struct ShellRequest {
    pub env: Option<HashMap<String, String>>,
    pub run: String,

    /// bash command
    /// example:
    ///    - bash
    ///    - sh
    ///    - pwsh
    /// default is bash
    pub bash: Option<String>,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct DownloadRequest {
    pub path: String,
}

async fn download_range_file(headers: HeaderMap) -> Result<Response, ApiError> {
    let file_path = headers
        .get("file_path")
        .ok_or(anyhow::anyhow!("No file_path in headers"))?
        .to_str()
        .map_err(|err| anyhow::anyhow!(err))?;

    if !tokio::fs::try_exists(file_path).await? {
        return Err(anyhow::anyhow!("can not find file: {}", file_path).into());
    }

    let mime = mime_guess::from_path(file_path).first_or_octet_stream();

    let file_path = Path::new(&file_path);
    let file_size = file_path.metadata()?.len();

    let ranges = headers
        .get("Range")
        .and_then(|range| range.to_str().ok())
        .and_then(|range| HttpRange::parse(range, file_size).ok());

    let range = match ranges {
        Some(mut ranges) => match ranges.len() {
            0 => None,
            1 => Some(ranges.remove(0)),
            _ => return Err(anyhow::anyhow!("Multiple ranges are not supported").into()),
        },
        None => None,
    };

    match range {
        Some(range) => {
            let start = range.start;

            let mut file = File::open(file_path).await?;
            file.seek(std::io::SeekFrom::Start(start))
                .await
                .map_err(|err| anyhow::anyhow!(err))?;

            let body = axum::body::Body::from_stream(ReaderStream::new(file.take(range.length)));

            Ok(Response::builder()
                .status(StatusCode::OK)
                .header(axum::http::header::CONTENT_TYPE, mime.as_ref())
                .header("Content-Length", range.length)
                .header(
                    "ContentRange",
                    format!("bytes {}-{}/{}", start, start + range.length - 1, file_size),
                )
                .body(body)?)
        }
        None => {
            let file = File::open(file_path).await?;
            let body = axum::body::Body::from_stream(ReaderStream::new(file));

            Ok(Response::builder()
                .status(StatusCode::OK)
                .header(axum::http::header::CONTENT_TYPE, mime.as_ref())
                .header("Content-Length", file_size)
                .body(body)?)
        }
    }
}

async fn download_file(Json(req): Json<DownloadRequest>) -> Result<Response, ApiError> {
    if !tokio::fs::try_exists(&req.path).await? {
        return Err(anyhow::anyhow!("can not find file: {}", req.path).into());
    }

    let file = tokio::fs::File::open(&req.path)
        .await
        .map_err(|e| anyhow::anyhow!(e))?;

    let body = axum::body::Body::from_stream(ReaderStream::new(file));

    let mime = mime_guess::from_path(&req.path).first_or_octet_stream();

    return Ok(Response::builder()
        .status(StatusCode::OK)
        .header(axum::http::header::CONTENT_TYPE, mime.as_ref())
        .body(body)?);
}

async fn upload_file(headers: HeaderMap, body: Body) -> Result<String, ApiError> {
    let file = headers
        .get("file_name")
        .ok_or(anyhow::anyhow!("No file_name in headers"))?
        .to_str()
        .map_err(|err| anyhow::anyhow!(err))?;

    let file_base = headers
        .get("file_base")
        .map(|f| Some(f.to_str()))
        .unwrap_or(None)
        .transpose()?;

    let config = config::rekcod_agent_config();
    let base_path = std::path::Path::new(&config.data_path);

    async {
        let base_dir = if let Some(file_base) = file_base {
            base_path.join(file_base)
        } else {
            base_path.to_path_buf()
        };

        if !base_dir.exists() {
            std::fs::create_dir_all(&base_dir)?;
        }

        let body_with_io_error = body
            .into_data_stream()
            .map_err(|err| std::io::Error::new(std::io::ErrorKind::Other, err));
        let body_reader = StreamReader::new(body_with_io_error);
        futures::pin_mut!(body_reader);

        // Create the file. `File` implements `AsyncWrite`.
        let path = base_dir.join(file);
        let mut file = BufWriter::new(File::create(&path).await?);

        // Copy the body into the file.
        tokio::io::copy(&mut body_reader, &mut file).await?;

        let flie = path.to_str().unwrap_or("");
        Ok::<_, std::io::Error>(flie.to_string())
    }
    .await
    .map_err(|err| anyhow::anyhow!(err).into())
}

async fn shell_stream(Json(req): Json<ShellRequest>) -> Result<Response, ApiError> {
    let bash = req.bash.unwrap_or("bash".to_string());
    // need check bash is exists
    let mut cmd = tokio::process::Command::new(bash);

    // args
    cmd.arg("-c").arg(req.run);

    // env
    if let Some(env) = req.env {
        for (key, value) in env {
            cmd.env(key, value);
        }
    }

    let mut cmd = cmd
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()?;

    let stdout = cmd.stdout.take().expect("Failed to get stdout");
    let stderr = cmd.stderr.take().expect("Failed to get stderr");
    let stdout_stream = tokio_util::io::ReaderStream::new(stdout);
    let stderr_stream = tokio_util::io::ReaderStream::new(stderr);
    let merged_stream = futures::stream::select(stdout_stream, stderr_stream);

    let body = axum::body::Body::from_stream(merged_stream.map_ok(|chunk| chunk.to_vec()));
    return Ok(Response::builder()
        .status(StatusCode::OK)
        .header(axum::http::header::CONTENT_TYPE, "application/octet-stream")
        .body(body)?);
}

async fn get_sys_info() -> Result<Json<ApiJsonResponse<SystemInfoResponse>>, ApiError> {
    Ok(ApiJsonResponse::success(sys_info().into()).into())
}
