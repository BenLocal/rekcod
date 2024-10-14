use std::sync::Arc;

use bollard::{BollardRequest, Docker};
use tracing::info;

pub fn rekcod_connect<S>(
    client_addr: Option<S>,
    path_prefix: &str,
    timeout: u64,
) -> anyhow::Result<Docker>
where
    S: Into<String>,
{
    let http_connector = hyper_util::client::legacy::connect::HttpConnector::new();
    let mut client_builder =
        hyper_util::client::legacy::Client::builder(hyper_util::rt::TokioExecutor::new());
    client_builder.pool_max_idle_per_host(0);
    let http_client = Arc::new(client_builder.build(http_connector));

    let path_prefix = Arc::new(path_prefix.to_owned());
    let docker = Docker::connect_with_custom_transport(
        move |req: BollardRequest| {
            let http_client = Arc::clone(&http_client);
            let path_prefix = Arc::clone(&path_prefix);
            Box::pin(async move {
                let (mut p, b) = req.into_parts();
                // let _prev = p.headers.insert("host", host);
                let mut uri = p.uri.into_parts();
                uri.path_and_query = uri
                    .path_and_query
                    .map(|paq| {
                        info!("proxy docker request url: {:?}", paq);
                        hyper::http::uri::PathAndQuery::try_from(format!(
                            "{}{}",
                            path_prefix,
                            paq.as_str()
                        ))
                    })
                    .transpose()
                    .map_err(bollard::errors::Error::from)?;
                p.uri = uri.try_into().map_err(bollard::errors::Error::from)?;
                let req = BollardRequest::from_parts(p, b);
                http_client
                    .request(req)
                    .await
                    .map_err(bollard::errors::Error::from)
            })
        },
        client_addr,
        timeout,
        bollard::API_DEFAULT_VERSION,
    )?;

    Ok(docker)
}
