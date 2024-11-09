use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Application {
    pub name: String,
    pub description: String,
    pub version: Option<String>,
    pub values: Option<serde_yaml::Value>,
}
