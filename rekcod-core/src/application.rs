use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Application {
    pub name: String,
    pub description: String,
    pub version: Option<String>,
    pub qa: Option<Vec<ApplicationQaItem>>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct ApplicationQaItem {
    pub id: String,
    pub name: String,
    pub label: String,
    #[serde(rename = "type")]
    pub typ: String,
    pub default_value: Option<String>,
}
