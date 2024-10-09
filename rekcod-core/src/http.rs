use axum::response::{IntoResponse, Response};
use hyper::StatusCode;
use tracing::error;

pub struct ApiError(anyhow::Error);

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        error!("Manager went wrong: {:?}", self.0);
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Manager went wrong because service inner error"),
        )
            .into_response()
    }
}

impl<E> From<E> for ApiError
where
    E: Into<anyhow::Error>,
{
    fn from(err: E) -> Self {
        Self(err.into())
    }
}
