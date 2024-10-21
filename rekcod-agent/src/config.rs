use once_cell::sync::OnceCell;
use rekcod_core::obj::RekcodType;

#[derive(Debug, Clone)]
pub struct RekcodAgentConfig {
    pub data_path: String,
    pub config_path: String,
    pub master_host: String,
    pub api_port: u16,
    pub typ: RekcodType,
}

static REKCOD_CONFIG: OnceCell<RekcodAgentConfig> = OnceCell::new();

pub(crate) fn rekcod_agent_config() -> &'static RekcodAgentConfig {
    REKCOD_CONFIG
        .get()
        .expect("pls init rekcod agent config first")
}

pub fn init_rekcod_agent_config(config: RekcodAgentConfig) {
    REKCOD_CONFIG
        .set(config)
        .expect("config can only be set once");
}
