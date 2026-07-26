use serde::{Deserialize, Serialize};
use sysinfo::System;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemTopology {
    pub gpu_name: String,
    pub vram_total_bytes: u64,
    pub ram_total_bytes: u64,
    pub is_dedicated_gpu: bool,
}

impl Default for SystemTopology {
    fn default() -> Self {
        Self {
            gpu_name: "Generic / Shared Memory".to_string(),
            vram_total_bytes: 4 * 1024 * 1024 * 1024, // 4 GB default safe fallback
            ram_total_bytes: 16 * 1024 * 1024 * 1024, // 16 GB default safe fallback
            is_dedicated_gpu: false,
        }
    }
}

/// Detecta a topologia de hardware (RAM + VRAM da dGPU ou Memória Unificada) em tempo O(1) com zero overhead e tratamento Fail-Safe.
pub fn detect_system_topology() -> SystemTopology {
    let mut sys = System::new();
    sys.refresh_memory();
    let ram_total = sys.total_memory();

    // 1. Tenta a detecção nativa de dGPU via NVML (NVIDIA)
    #[cfg(feature = "llama_backend")]
    if let Ok(nvml) = nvml_wrapper::Nvml::init() {
        if let Ok(device) = nvml.device_by_index(0) {
            let gpu_name = device.name().unwrap_or_else(|_| "NVIDIA GPU".to_string());
            if let Ok(mem_info) = device.memory_info() {
                return SystemTopology {
                    gpu_name,
                    vram_total_bytes: mem_info.total,
                    ram_total_bytes: ram_total,
                    is_dedicated_gpu: true,
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
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_system_topology_fail_safe() {
        let topo = detect_system_topology();
        assert!(topo.vram_total_bytes > 0);
    }
}
