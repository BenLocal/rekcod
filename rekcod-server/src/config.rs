use once_cell::sync::OnceCell;

#[derive(Debug)]
pub struct RekcodServerConfig {
    pub db_url: String,
    pub config_path: String,
    pub data_path: String,
    pub api_port: u16,
    pub dashboard: bool,
    pub dashboard_base_url: Option<String>,
}

static REKCOD_CONFIG: OnceCell<RekcodServerConfig> = OnceCell::new();

pub(crate) fn rekcod_server_config() -> &'static RekcodServerConfig {
    REKCOD_CONFIG
        .get()
        .expect("pls init rekcod server config first")
}

pub fn init_rekcod_server_config(config: RekcodServerConfig) {
    REKCOD_CONFIG
        .set(config)
        .expect("config can only be set once");
}
