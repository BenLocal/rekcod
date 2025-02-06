use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct ApplicationTmpl {
    pub name: String,
    pub description: String,
    pub version: Option<String>,
    pub qa: Option<Vec<ApplicationTmplQaItem>>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct ApplicationTmplQaItem {
    pub id: String,
    pub name: String,
    pub label: String,
    #[serde(rename = "type")]
    pub typ: String,
    pub default_value: Option<String>,
}
