use rekcod_core::{
    api::req::RegisterNodeRequest, client::get_client, constants::REKCOD_SERVER_PREFIX_PATH,
};
use tokio_util::sync::CancellationToken;
use tracing::error;

use crate::config;

pub(crate) async fn register_node(cancel: CancellationToken) -> anyhow::Result<()> {
    loop {
        tokio::select! {
            _ = cancel.cancelled() => {
                break;
            }
            _ = tokio::time::sleep(std::time::Duration::from_secs(10)) => {
                let config = config::rekcod_agent_config();

                // register node
                let url = format!(
                    "http://{}{}/node/register",
                    config.master_host, REKCOD_SERVER_PREFIX_PATH
                );

                let my_local_ip = local_ip_address::local_ip().map(|s| s.to_string()).unwrap_or("127.0.0.1".to_string());
                let sys = crate::job::sys::sys_info_global();
                let req = RegisterNodeRequest {
                    name: my_local_ip.clone(),
                    host_name: sys.host_name.clone().unwrap_or("unknown".to_string()),
                    ip: my_local_ip.clone(),
                    port: config.api_port,
                    token: "".to_string(),
                    version: "".to_string(),
                    arch: sys.cpu_arch.clone().unwrap_or("unknown".to_string()),
                    os: sys.system_name.clone().unwrap_or("unknown".to_string()),
                    os_version: sys.os_version.clone().unwrap_or("unknown".to_string()),
                    os_long_version: sys.long_os_version.clone().unwrap_or("unknown".to_string()),
                    os_kernel: sys.kernel_version.clone().unwrap_or("unknown".to_string()),
                    status: true,
                };
                if let Err(e) = get_client()?.post(url).json(&req).send().await {
                    error!("register node error: {:?}", e);
                }

            }
        }
    }

    Ok(())
}
