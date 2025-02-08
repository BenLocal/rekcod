use tokio::time::{self, Duration};
use tokio_util::sync::CancellationToken;
use tracing::info;

use crate::node::manager::node_manager;

pub async fn monitor(cancel: CancellationToken) {
    info!("start monitor nodes");

    // first monitor should be init all nodes
    // unnecessary to check the result
    let _ = node_manager().get_all_nodes(true).await;

    loop {
        tokio::select! {
            _ = cancel.cancelled() => {
                info!("monitor nodes cancelled");
                break;
            }
            _ = time::sleep(Duration::from_secs(5)) => {
                let _ = node_manager().monitor_nodes().await;
            }
        }
    }
}
