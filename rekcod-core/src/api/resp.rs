use serde::{Deserialize, Serialize};
use tabled::Tabled;

#[serde_with::skip_serializing_none]
#[derive(Serialize, Deserialize, Default, Debug)]
pub struct ApiJsonResponse<T>
where
    T: Sized + Serialize + Send + Sync,
{
    msg: String,
    code: i32,
    data: Option<T>,
}

impl<T> ApiJsonResponse<T>
where
    T: Sized + Serialize + Send + Sync,
{
    pub fn msg(&self) -> &str {
        &self.msg
    }

    pub fn code(&self) -> i32 {
        self.code
    }

    pub fn data(&self) -> Option<&T> {
        self.data.as_ref()
    }
}

#[allow(dead_code)]
impl<T> ApiJsonResponse<T>
where
    T: Sized + Serialize + Send + Sync,
{
    pub fn empty_success() -> Self {
        Self {
            msg: "".to_string(),
            code: 0,
            data: None::<T>,
        }
    }

    pub fn success(d: T) -> Self {
        Self {
            msg: "".to_string(),
            code: 0,
            data: Some(d),
        }
    }

    pub fn success_optional(d: Option<T>) -> Self {
        Self {
            msg: "".to_string(),
            code: 0,
            data: d,
        }
    }

    pub fn empty_error(code: i32, msg: &str) -> Self {
        Self {
            msg: msg.to_string(),
            code: code,
            data: None::<T>,
        }
    }
}

#[derive(Serialize, Deserialize, Default)]
pub struct SystemInfoResponse {
    /// cpu usage in percent
    pub cpu_usage: f32,
    /// cpu count
    pub cpu_count: u32,
    /// available memory bytes
    pub mem_available: u64,
    /// total memory bytes
    pub mem_total: u64,
    /// memory usage in percent
    pub mem_usage: f32,
    /// memory free bytes
    pub mem_free: u64,
    /// memory used bytes
    pub mem_used: u64,

    pub disks: Vec<SystemDiskInfo>,
    pub networks: Vec<SystemNetworkInfo>,

    pub system_name: Option<String>,
    pub kernel_version: Option<String>,
    pub os_version: Option<String>,
    pub long_os_version: Option<String>,
    pub host_name: Option<String>,
    pub cpu_arch: Option<String>,
}

#[derive(Serialize, Deserialize, Default)]
pub struct SystemDiskInfo {
    pub name: String,
    /// disk free bytes
    pub free: u64,
    /// disk total bytes
    pub total: u64,
    pub mount: String,
    pub removeable: bool,
}

#[derive(Serialize, Deserialize, Default)]
pub struct SystemNetworkInfo {
    pub name: String,
    pub ips: Vec<String>,
    pub mac: String,
    pub total_out: u64,
    pub total_in: u64,
}

#[derive(Serialize, Deserialize, Default, Tabled, Debug, Clone)]
#[tabled(rename_all = "UPPERCASE")]
pub struct NodeItemResponse {
    pub name: String,
    pub host_name: String,
    pub ip: String,
    pub port: u16,
    pub version: String,
    pub arch: String,
    pub os: String,
    pub os_version: String,
    pub os_kernel: String,
    pub status: bool,
}
