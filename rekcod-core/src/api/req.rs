use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Default, Debug)]
#[serde(default)]
pub struct RegisterNodeRequest {
    /// node name, should be unique
    pub name: String,
    /// linux uname or windows computer name
    pub host_name: String,
    /// ip
    pub ip: String,
    /// agent listen port
    pub port: u16,
    /// agent token
    pub token: String,
    /// agent version
    pub version: String,
    /// agent arch
    pub arch: String,
    /// agent os
    pub os: String,
    /// agent os version
    pub os_version: String,
    /// agent os long version
    pub os_long_version: String,
    /// agent os kernel
    pub os_kernel: String,
    /// agent os status true or false
    /// default: true
    /// if status is false, agent was not start
    /// if status is true, agent was start
    pub status: bool,
}

#[derive(Serialize, Deserialize, Default, Debug)]
#[serde(default)]
pub struct NodeListRequest {
    pub all: bool,
}

#[derive(Serialize, Deserialize, Default, Debug)]
#[serde(default)]
pub struct NodeInfoRequest {
    pub name: String,
}

#[derive(Serialize, Deserialize, Default, Debug)]
#[serde(default)]
pub struct NodeDockerQueryRequest {
    pub node_name: String,
}

#[derive(Serialize, Deserialize, Default, Debug)]
#[serde(default)]
pub struct NodeSysInfoRequest {
    pub name: String,
}

#[derive(Serialize, Deserialize, Default, Debug)]
#[serde(default)]
pub struct RenderTmplRequest {
    pub tmpl_context: String,
    pub tmpl_values: String,
}

#[derive(Serialize, Deserialize, Default, Debug)]
#[serde(default)]
pub struct AppDeployRequest {
    pub name: String,
    pub app_name: String,
    pub node_name: String,
    pub project: Option<String>,
    pub values: Option<String>,
    pub build: Option<bool>,
}

#[derive(Serialize, Deserialize, Default, Debug)]
#[serde(default)]
pub struct DockerImagePullAutoRequest {
    pub node_name: String,
    pub image_name: String,
}

#[derive(Serialize, Deserialize, Default, Debug, Clone)]
pub struct EnvRequest {
    pub values: String,
}
