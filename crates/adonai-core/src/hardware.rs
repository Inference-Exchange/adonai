use serde::{Deserialize, Serialize};
use sysinfo::{Disks, System};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HardwareProfile {
    pub platform: HostPlatform,
    pub cpu_brand: String,
    pub physical_core_count: Option<usize>,
    pub memory: MemoryProfile,
    pub storage: Vec<StorageProfile>,
    pub accelerators: Vec<Accelerator>,
    pub network: NetworkProfile,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HostPlatform {
    pub os: String,
    pub os_version: Option<String>,
    pub kernel_version: Option<String>,
    pub architecture: String,
    pub hostname: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MemoryProfile {
    pub total_bytes: u64,
    pub available_bytes: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StorageProfile {
    pub name: String,
    pub mount_point: String,
    pub total_bytes: u64,
    pub available_bytes: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NetworkProfile {
    pub default_exposure: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Accelerator {
    pub kind: AcceleratorKind,
    pub name: String,
    pub evidence: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum AcceleratorKind {
    AppleMetal,
    NvidiaCuda,
    AmdRocm,
    CpuOnly,
    Unknown,
}

pub fn scan_hardware() -> HardwareProfile {
    let mut system = System::new_all();
    system.refresh_all();

    let cpu_brand = system
        .cpus()
        .first()
        .map(|cpu| cpu.brand().trim().to_owned())
        .filter(|brand| !brand.is_empty())
        .unwrap_or_else(|| "unknown".to_owned());

    let storage = Disks::new_with_refreshed_list()
        .iter()
        .map(|disk| StorageProfile {
            name: disk.name().to_string_lossy().into_owned(),
            mount_point: disk.mount_point().to_string_lossy().into_owned(),
            total_bytes: disk.total_space(),
            available_bytes: disk.available_space(),
        })
        .collect();

    HardwareProfile {
        platform: HostPlatform {
            os: System::name().unwrap_or_else(|| std::env::consts::OS.to_owned()),
            os_version: System::os_version(),
            kernel_version: System::kernel_version(),
            architecture: std::env::consts::ARCH.to_owned(),
            hostname: System::host_name(),
        },
        cpu_brand,
        physical_core_count: System::physical_core_count(),
        memory: MemoryProfile {
            total_bytes: system.total_memory(),
            available_bytes: non_zero(system.available_memory()),
        },
        storage,
        accelerators: detect_accelerators(),
        network: NetworkProfile {
            default_exposure: "loopback-only".to_owned(),
        },
    }
}

fn non_zero(value: u64) -> Option<u64> {
    if value == 0 { None } else { Some(value) }
}

fn detect_accelerators() -> Vec<Accelerator> {
    let mut accelerators = Vec::new();

    if cfg!(target_os = "macos") && cfg!(target_arch = "aarch64") {
        accelerators.push(Accelerator {
            kind: AcceleratorKind::AppleMetal,
            name: "Apple Silicon Metal".to_owned(),
            evidence: "target_os=macos,target_arch=aarch64".to_owned(),
        });
    }

    if accelerators.is_empty() {
        accelerators.push(Accelerator {
            kind: AcceleratorKind::CpuOnly,
            name: "CPU".to_owned(),
            evidence: "no supported accelerator detected by first-pass scanner".to_owned(),
        });
    }

    accelerators
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scan_returns_platform_and_memory() {
        let profile = scan_hardware();

        assert!(!profile.platform.os.is_empty());
        assert!(!profile.platform.architecture.is_empty());
        assert!(profile.memory.total_bytes > 0);
        assert!(!profile.accelerators.is_empty());
    }
}
