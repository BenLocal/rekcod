use serde::{Deserialize, Serialize};

#[derive(Debug, Clone)]
pub enum RekcodType {
    /// server and agent all in one
    Master,
    /// agent
    Agent,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RekcodCfg {
    pub host: String,
    pub token: String,
}
