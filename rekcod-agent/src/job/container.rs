use std::{
    collections::HashMap,
    time::{self, SystemTime},
};

use bollard::container::ListContainersOptions;
use rekcod_core::docker::local_connect;
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;
use tracing::{error, info};

struct DebounceState {
    pub timeout: SystemTime,
}

impl DebounceState {
    fn expire(&self) -> bool {
        let current = time::SystemTime::now();
        let exp = match current.duration_since(self.timeout) {
            Ok(d) => d,
            Err(_) => {
                return false;
            }
        };
        exp.as_secs() > 60
    }
}

#[derive(Debug)]
struct ContainerInfo {
    pub id: String,
    pub name: String,
}

pub(crate) async fn docker_health_monitor(cancel: CancellationToken) -> anyhow::Result<()> {
    let docker = local_connect();
    let mut remaining: HashMap<String, DebounceState> = HashMap::new();
    let (debounce_tx, mut debounce_rx) = mpsc::channel::<ContainerInfo>(100);
    loop {
        tokio::select! {
            _ = cancel.cancelled() => {
                break;
            }
            Some(container) = debounce_rx.recv() => {
                match remaining.remove(&container.id) {
                    Some(mut state) => {
                        if !state.expire() {
                            remaining.insert(container.id.clone(), state);
                            continue;
                        } else {
                            state.timeout = time::SystemTime::now();
                        }
                        remaining.insert(container.id.clone(), state);
                    },
                    None => {
                        remaining.insert(container.id.clone(), DebounceState {
                            timeout: time::SystemTime::now(),
                        });
                    }
                };

                // restart docker container
                info!("restarting container({}): {}", &container.name, &container.id);
                match docker.restart_container(&container.id, None).await {
                    Ok(_) => {
                        info!("restarted container({}): {}", &container.name, &container.id);
                    },
                    Err(e) => {
                        error!("restart container({}): {} error: {}", &container.name, &container.id, e);
                    }
                };

                // clear timeout for container
                remaining.retain(|_, d| !d.expire());
            }
            _ = tokio::time::sleep(std::time::Duration::from_secs(10)) => {
                let mut filters = HashMap::new();
                // filters.insert("health", vec!["unhealthy"]);
                filters.insert("status", vec!["exited"]);

                let options = Some(ListContainersOptions {
                    all: true,
                    filters,
                    ..Default::default()
                });
                match docker.list_containers(options).await
                {
                    Ok(containers) => {
                        for container in containers {
                            if let (Some(container_id), Some(names)) = (container.id, container.names) {
                                let name = names.iter().next().map_or("".to_string(), |n| n.to_string());
                                debounce_tx.send(
                                    ContainerInfo {
                                        id: container_id,
                                        name: name,
                                    }
                                ).await?;
                            }
                        }
                    },
                    Err(e) => {
                        error!("restart loop in list containers error: {}", e);
                    }
                }
            }
        }
    }

    Ok(())
}
