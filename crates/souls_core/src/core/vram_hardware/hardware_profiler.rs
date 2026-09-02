use serde::{Deserialize, Serialize};
use sysinfo::{Disks, System};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CpuInstructionSet {
    Avx512,
    Avx2,
    Neon,
    Base,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemTopology {
    pub gpu_name: String,
    pub vram_total_bytes: u64,
    pub ram_total_bytes: u64,
    pub is_dedicated_gpu: bool,
    pub primary_simd_extension: CpuInstructionSet,
    pub is_nvme_ssd: bool,
    /// Telemetria real da largura de banda PCIe em GB/s extraída do hardware/NVML (ex: 15.75 GB/s para Gen3x16, 31.51 GB/s para Gen4x16).
    /// Se a leitura física falhar ou o dispositivo for memória compartilhada/integrada, retorna `None` (Zero Chutes).
    pub pcie_bandwidth_estimated_gbps: Option<f32>,
}

impl Default for SystemTopology {
    fn default() -> Self {
        Self {
            gpu_name: "Generic / Shared Memory".to_string(),
            vram_total_bytes: 4 * 1024 * 1024 * 1024, // 4 GB default safe fallback
            ram_total_bytes: 16 * 1024 * 1024 * 1024, // 16 GB default safe fallback
            is_dedicated_gpu: false,
            primary_simd_extension: CpuInstructionSet::Base,
            is_nvme_ssd: false,
            pcie_bandwidth_estimated_gbps: None,
        }
    }
}

/// Detecta a extensão vetorial primária (SIMD) suportada pela CPU de forma agnóstica (x86_64 / ARM / AArch64).
pub fn detect_primary_simd() -> CpuInstructionSet {
    #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
    {
        if std::is_x86_feature_detected!("avx512f") {
            return CpuInstructionSet::Avx512;
        }
        if std::is_x86_feature_detected!("avx2") {
            return CpuInstructionSet::Avx2;
        }
    }

    #[cfg(any(target_arch = "arm", target_arch = "aarch64"))]
    {
        #[cfg(target_arch = "aarch64")]
        {
            if std::arch::is_aarch64_feature_detected!("neon") {
                return CpuInstructionSet::Neon;
            }
        }
        #[cfg(not(target_arch = "aarch64"))]
        {
            #[cfg(target_feature = "neon")]
            return CpuInstructionSet::Neon;
        }
    }

    CpuInstructionSet::Base
}

/// Calcule a largura de banda teórica real do PCIe com base na Geração e Vias (Width).
pub fn calculate_pcie_bandwidth_gbps(gen: u32, width: u32) -> Option<f32> {
    if width == 0 {
        return None;
    }
    let gbps_per_lane = match gen {
        1 => 0.250,
        2 => 0.500,
        3 => 0.9846,
        4 => 1.9692,
        5 => 3.9384,
        6 => 7.8768,
        _ => return None,
    };
    Some(gbps_per_lane * (width as f32))
}

/// Detecta a topologia de hardware (RAM, VRAM, SIMD primário, NVMe e largura de banda PCIe física)
/// em tempo O(1) com zero overhead e tratamento Fail-Safe.
pub fn detect_system_topology() -> SystemTopology {
    let mut sys = System::new();
    sys.refresh_memory();
    let ram_total = sys.total_memory();

    let primary_simd_extension = detect_primary_simd();

    let disks = Disks::new_with_refreshed_list();
    let is_nvme_ssd = disks.iter().any(|disk| {
        let name_upper = disk.name().to_string_lossy().to_uppercase();
        let kind_str = format!("{:?}", disk.kind()).to_uppercase();
        name_upper.contains("NVME")
            || kind_str.contains("SSD")
            || matches!(disk.kind(), sysinfo::DiskKind::SSD)
    });

    // 1. Tenta a detecção nativa de dGPU via NVML (NVIDIA)
    #[cfg(feature = "llama_backend")]
    if let Ok(nvml) = nvml_wrapper::Nvml::init() {
        if let Ok(device) = nvml.device_by_index(0) {
            let gpu_name = device.name().unwrap_or_else(|_| "NVIDIA GPU".to_string());
            if let Ok(mem_info) = device.memory_info() {
                let pcie_bandwidth_estimated_gbps = {
                    let cur_gen = device.current_pcie_link_gen().ok();
                    let cur_width = device.current_pcie_link_width().ok();
                    let max_gen = device.max_pcie_link_gen().ok();
                    let max_width = device.max_pcie_link_width().ok();

                    let gen = cur_gen.or(max_gen);
                    let width = cur_width.or(max_width);

                    match (gen, width) {
                        (Some(g), Some(w)) => calculate_pcie_bandwidth_gbps(g, w),
                        _ => None,
                    }
                };

                return SystemTopology {
                    gpu_name,
                    vram_total_bytes: mem_info.total,
                    ram_total_bytes: ram_total,
                    is_dedicated_gpu: true,
                    primary_simd_extension,
                    is_nvme_ssd,
                    pcie_bandwidth_estimated_gbps,
                };
            }
        }
    }

    // 2. Fallback Fail-Safe: dGPU não detectada -> alocação em Memória Unificada / Compartilhada (50% da RAM)
    let fallback_vram = if ram_total > 0 { ram_total / 2 } else { 4 * 1024 * 1024 * 1024 };

    SystemTopology {
        gpu_name: "Host System (Unified/Shared Memory)".to_string(),
        vram_total_bytes: fallback_vram,
        ram_total_bytes: ram_total,
        is_dedicated_gpu: false,
        primary_simd_extension,
        is_nvme_ssd,
        pcie_bandwidth_estimated_gbps: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_system_topology_fail_safe() {
        let topo = detect_system_topology();
        assert!(topo.vram_total_bytes > 0);
        assert!(topo.ram_total_bytes > 0);

        let simd = detect_primary_simd();
        assert_eq!(topo.primary_simd_extension, simd);

        let default_topo = SystemTopology::default();
        assert_eq!(default_topo.primary_simd_extension, CpuInstructionSet::Base);
        assert!(!default_topo.is_nvme_ssd);
        assert_eq!(default_topo.pcie_bandwidth_estimated_gbps, None);
    }

    #[test]
    fn test_pcie_bandwidth_calculation() {
        let gen3_x16 = calculate_pcie_bandwidth_gbps(3, 16);
        assert!(gen3_x16.is_some());
        assert!((gen3_x16.unwrap() - 15.75).abs() < 0.1);

        let gen4_x16 = calculate_pcie_bandwidth_gbps(4, 16);
        assert!(gen4_x16.is_some());
        assert!((gen4_x16.unwrap() - 31.51).abs() < 0.1);

        assert_eq!(calculate_pcie_bandwidth_gbps(0, 16), None);
        assert_eq!(calculate_pcie_bandwidth_gbps(3, 0), None);
    }
}
