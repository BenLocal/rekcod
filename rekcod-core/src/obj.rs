use std::fmt::Display;

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

#[derive(Debug, Clone)]
pub enum NodeStatus {
    Online,
    Offline,
}

impl Display for NodeStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            NodeStatus::Online => write!(f, "online"),
            NodeStatus::Offline => write!(f, "offline"),
        }
    }
}

impl<T> From<T> for NodeStatus
where
    T: AsRef<str>,
{
    fn from(status: T) -> Self {
        match status.as_ref() {
            "online" => NodeStatus::Online,
            "offline" => NodeStatus::Offline,
            _ => NodeStatus::Offline,
        }
    }
}
