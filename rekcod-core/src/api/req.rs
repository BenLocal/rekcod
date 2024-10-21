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
