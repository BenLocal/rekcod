use axum::{extract::Request, http::HeaderValue, middleware::Next, response::Response};
use hyper::StatusCode;
use once_cell::sync::OnceCell;

use crate::constants::TOEKN_HEADER_KEY;

static REKCOD_TOKEN: OnceCell<String> = OnceCell::new();

pub fn get_token() -> &'static str {
    REKCOD_TOKEN.get().expect("pls init rekcod token first")
}

pub fn header_value_token() -> HeaderValue {
    HeaderValue::from_static(get_token())
}

pub fn set_token(token: String) {
    REKCOD_TOKEN.set(token).expect("token can only be set once");
}

pub async fn token_auth(req: Request, next: Next) -> Result<Response, StatusCode> {
    let token = get_token();
    let auth_header = req
        .headers()
        .get(TOEKN_HEADER_KEY)
        .and_then(|header| header.to_str().ok());

    match auth_header {
        Some(auth_header) if auth_header == token => Ok(next.run(req).await),
        _ => Err(StatusCode::UNAUTHORIZED),
    }
}
