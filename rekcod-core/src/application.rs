use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize)]
pub struct Application {
    pub name: String,
    pub description: String,
    pub version: String,
}
