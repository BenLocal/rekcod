use once_cell::sync::OnceCell;

use crate::{AgentArgs, ServerArgs};

static REKCOD_CONFIG: OnceCell<RekcodConfig> = OnceCell::new();

pub(crate) fn rekcod_config() -> &'static RekcodConfig {
    REKCOD_CONFIG.get().expect("pls init rekcod config first")
}

pub(crate) fn init_rekcod_config(config: RekcodConfig) {
    REKCOD_CONFIG
        .set(config)
        .expect("config can only be set once");
}

#[derive(Debug, PartialEq, Eq)]
pub enum RekcodServerType {
    Server,
    Agent,
}

#[derive(Debug)]
pub struct RekcodConfig {
    pub server_type: RekcodServerType,
    pub api_port: u16,
}

impl From<ServerArgs> for RekcodConfig {
    fn from(args: ServerArgs) -> Self {
        Self {
            server_type: RekcodServerType::Server,
            api_port: args.port,
        }
    }
}

impl From<AgentArgs> for RekcodConfig {
    fn from(args: AgentArgs) -> Self {
        Self {
            server_type: RekcodServerType::Agent,
            api_port: args.port,
        }
    }
}
