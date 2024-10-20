use once_cell::sync::OnceCell;
use rekcod_core::constants::REKCOD_SERVER_PREFIX_PATH;

static REKCOD_CLI_CONFIG: OnceCell<RekcodCliConfig> = OnceCell::new();

pub(crate) fn rekcod_cli_config() -> &'static RekcodCliConfig {
    REKCOD_CLI_CONFIG
        .get()
        .expect("pls init rekcod cli config first")
}

pub(crate) fn init_rekcod_cli_config(config: RekcodCliConfig) {
    REKCOD_CLI_CONFIG
        .set(config)
        .expect("config can only be set once");
}

#[derive(Debug)]
pub struct RekcodCliConfig {
    pub host: String,
}

impl RekcodCliConfig {
    pub fn http_server_host(&self) -> String {
        format!("http://{}{}", self.host, REKCOD_SERVER_PREFIX_PATH)
    }
}
