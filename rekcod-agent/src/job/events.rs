use std::{collections::HashMap, time::Duration};

use futures::StreamExt as _;
use rekcod_core::docker::local_connect;
use tokio_util::sync::CancellationToken;

pub(crate) async fn docker_event_monitor(cancel: CancellationToken) -> anyhow::Result<()> {
    let docker = local_connect();
    let interval = 5;
    loop {
        tokio::select! {
            _ = cancel.cancelled() => {
                break;
            }
            _ = tokio::time::sleep(Duration::from_secs(interval)) => {
                let option = bollard::system::EventsOptions::<String> {
                    since: Some(chrono::DateTime::to_utc(
                        &(chrono::Utc::now() - Duration::from_secs(interval)),
                    )),
                    until: Some(chrono::Utc::now()),
                    filters: HashMap::new(),
                };
                let mut events = docker.events(Some(option));
                while let Some(_event) = events.next().await {
                    //info!("event: {:?}", event);
                }
            }
        }
    }

    Ok(())
}
