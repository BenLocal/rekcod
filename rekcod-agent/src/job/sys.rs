use std::sync::Mutex;

use once_cell::sync::Lazy;
use rekcod_core::api::resp::{SystemDiskInfo, SystemInfoResponse, SystemNetworkInfo};
use sysinfo::{CpuRefreshKind, MemoryRefreshKind, RefreshKind, System};
use tokio_util::sync::CancellationToken;

static SYS_INFO: Lazy<Mutex<SysInfo>> = Lazy::new(|| Mutex::new(SysInfo::default()));
static SYS_INFO_GLOBAL: Lazy<GlobalSysInfo> = Lazy::new(|| GlobalSysInfo {
    system_name: System::name(),
    kernel_version: System::kernel_version(),
    os_version: System::os_version(),
    long_os_version: System::long_os_version(),
    host_name: System::host_name(),
    cpu_arch: Some(System::cpu_arch()),
});

pub(crate) fn sys_info() -> SysInfo {
    let cache = SYS_INFO.lock().unwrap();
    cache.clone()
}

pub(crate) fn sys_info_global() -> &'static GlobalSysInfo {
    &SYS_INFO_GLOBAL
}

#[derive(Debug, Default, Clone)]
pub(crate) struct SysInfo {
    pub cpu_usage: f32,
    pub cpu_count: u32,
    pub mem_available: u64,
    pub mem_total: u64,
    // mem usage x%
    pub mem_usage: f32,
    pub mem_free: u64,
    pub mem_used: u64,
    pub disks: Vec<SysDisk>,
    pub networks: Vec<SysNetwork>,
}

pub(crate) struct GlobalSysInfo {
    pub system_name: Option<String>,
    pub kernel_version: Option<String>,
    pub os_version: Option<String>,
    pub long_os_version: Option<String>,
    pub host_name: Option<String>,
    pub cpu_arch: Option<String>,
}

#[derive(Debug, Default, Clone)]
pub(crate) struct SysDisk {
    pub name: String,
    pub total: u64,
    pub free: u64,
    pub mount: String,
    pub removable: bool,
}

#[derive(Debug, Default, Clone)]
pub(crate) struct SysNetwork {
    pub name: String,
    pub ips: Vec<String>,
    pub mac: String,
    pub total_out: u64,
    pub total_in: u64,
}

impl Into<SystemInfoResponse> for SysInfo {
    fn into(self) -> SystemInfoResponse {
        let global = sys_info_global();

        SystemInfoResponse {
            cpu_usage: self.cpu_usage,
            mem_available: self.mem_available,
            mem_total: self.mem_total,
            mem_usage: self.mem_usage,
            mem_free: self.mem_free,
            mem_used: self.mem_used,
            cpu_count: self.cpu_count,
            system_name: global.system_name.clone(),
            kernel_version: global.kernel_version.clone(),
            os_version: global.os_version.clone(),
            long_os_version: global.long_os_version.clone(),
            host_name: global.host_name.clone(),
            cpu_arch: global.cpu_arch.clone(),
            disks: self.disks.iter().map(|x| x.into()).collect(),
            networks: self.networks.iter().map(|x| x.into()).collect(),
        }
    }
}

impl Into<SystemDiskInfo> for &SysDisk {
    fn into(self) -> SystemDiskInfo {
        SystemDiskInfo {
            name: self.name.clone(),
            free: self.free,
            total: self.total,
            mount: self.mount.clone(),
            removeable: self.removable,
        }
    }
}

impl Into<SystemNetworkInfo> for &SysNetwork {
    fn into(self) -> SystemNetworkInfo {
        SystemNetworkInfo {
            name: self.name.clone(),
            ips: self.ips.clone(),
            mac: self.mac.clone(),
            total_out: self.total_out,
            total_in: self.total_in,
        }
    }
}

pub(crate) async fn sys_monitor(cancel: CancellationToken) -> anyhow::Result<()> {
    let mut s = sysinfo::System::new();
    let mut disks = sysinfo::Disks::new();
    let mut networks = sysinfo::Networks::new();
    let rk = RefreshKind::nothing()
        .with_cpu(CpuRefreshKind::everything())
        .with_memory(MemoryRefreshKind::everything());
    loop {
        tokio::select! {
            _ = cancel.cancelled() => {
                break;
            }
            _ = tokio::time::sleep(std::time::Duration::from_secs(1)) => {
                s.refresh_specifics(rk);

                let mut sys_info = SYS_INFO.lock().unwrap();
                // cpu
                sys_info.cpu_usage = s.global_cpu_usage();
                sys_info.cpu_count = s.cpus().len() as u32;
                // mem
                sys_info.mem_available = s.available_memory();
                sys_info.mem_total = s.total_memory();
                sys_info.mem_usage = s.used_memory() as f32 / s.total_memory() as f32 * 100.0;
                sys_info.mem_free = s.free_memory();
                sys_info.mem_used = s.used_memory();

                if cfg!(target_os = "linux") {
                    disks.refresh(true);
                    sys_info.disks = disks.list().iter().map(|x| {
                        SysDisk {
                            name:x.name().to_string_lossy().to_string(),
                            total: x.total_space(),
                            free: x.available_space(),
                            mount: x.mount_point().to_string_lossy().to_string(),
                            removable: x.is_removable()
                        }
                    }).collect::<Vec<_>>();
                }


                networks.refresh(true);
                sys_info.networks = networks.list().iter().map(|(x, d)| {
                    SysNetwork {
                        name: x.to_string(),
                        ips: d.ip_networks().iter().map(|x| x.to_string()).collect(),
                        mac: d.mac_address().to_string(),
                        total_out: d.total_transmitted(),
                        total_in: d.total_received(),
                    }
                }).collect::<Vec<_>>();
            }
        }
    }

    Ok(())
}
