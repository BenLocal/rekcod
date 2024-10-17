use hyper::header;

use crate::auth::{get_token, TOEKN_HEADER_KEY};

pub fn get_client() -> anyhow::Result<reqwest::Client> {
    let mut headers = header::HeaderMap::new();
    headers.insert(
        TOEKN_HEADER_KEY,
        header::HeaderValue::from_static(get_token()),
    );

    Ok(reqwest::Client::builder()
        .default_headers(headers)
        .build()?)
}
