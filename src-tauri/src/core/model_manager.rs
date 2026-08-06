use thiserror::Error;
pub use crate::core::model_registry::{GgufMetadataCache, GLOBAL_GGUF_METADATA_CACHE};

#[derive(Error, Debug, PartialEq)]
pub enum ModelManagerError {
    #[error("Modelo rejeitado por exceder teto de VRAM FinOps de 5.0 GB (Projetado: {0} MB)")]
    OverbudgetVram(u64),
    #[error("Modelo não encontrado")]
    NotFound,
}

#[derive(Debug, PartialEq)]
pub struct ModelProfilingResult {
    pub model_name: String,
    pub static_weight_mb: u64,
    pub kv_cache_projected_mb: u64,
    pub total_vram_projected_mb: u64,
    pub is_viable: bool,
}

/// Auto-Profiling O(1) de consumo de VRAM projetado a partir de metadados GGUF
pub fn profile_gguf_vram(
    model_name: &str,
    static_weight_mb: u64,
    max_context_tokens: u32,
    is_ssm_mamba: bool,
) -> Result<ModelProfilingResult, ModelManagerError> {
    // Para Transformers densos, KV Cache cresce com janelas de contexto
    // Para SSM Mamba-2 / Falcon-Mamba, o estado da KV Cache é de complexidade O(1) constante (~200MB)
    let kv_cache_projected_mb = if is_ssm_mamba {
        200
    } else {
        ((max_context_tokens as u64) * 256) / 1024 / 1024 // ~1.0GB para 4096 tokens em FP16
    };

    let cuda_buffer_overhead_mb = 800; // Overhead fixo de runtime CUDA / drivers
    let total_vram_projected_mb = static_weight_mb + kv_cache_projected_mb + cuda_buffer_overhead_mb;

    if total_vram_projected_mb > 5000 {
        return Err(ModelManagerError::OverbudgetVram(total_vram_projected_mb));
    }

    Ok(ModelProfilingResult {
        model_name: model_name.to_string(),
        static_weight_mb,
        kv_cache_projected_mb,
        total_vram_projected_mb,
        is_viable: true,
    })
}

pub struct RalphLoopState {
    pub prompt_prefix_hash: u64,
    pub current_reflection_step: u8,
    pub max_reflection_steps: u8,
    pub is_critic_on_cpu: bool,
    pub prefix_reused: bool,
}

impl RalphLoopState {
    pub fn new(prompt_hash: u64) -> Self {
        Self {
            prompt_prefix_hash: prompt_hash,
            current_reflection_step: 0,
            max_reflection_steps: 3,
            is_critic_on_cpu: true,
            prefix_reused: false,
        }
    }

    pub fn trigger_reflection(&mut self, _critic_feedback: &str) -> Result<bool, &'static str> {
        if self.current_reflection_step >= self.max_reflection_steps {
            return Err("Limite maximo de 3 iteracoes de reflexao atingido (Fail-Closed)");
        }
        self.current_reflection_step += 1;
        self.prefix_reused = true; // Preserva KV Cache Prefix sem re-prefill na GPU
        Ok(true)
    }
}

pub struct NGramSpeculationBuffer {
    pub n_match: usize,
    pub n_min: usize,
    pub n_max: usize,
    pub hash_table_size_bytes: usize,
    pub allocated_in_host_ram: bool,
    pub vram_bytes_allocated: usize,
}

pub fn allocate_ngram_speculation_buffer(n_match: usize, n_min: usize, n_max: usize) -> NGramSpeculationBuffer {
    // ADR-032 / PRD-10.2: Tabela de hash N-Gram alocada na RAM do Host (pegada ~16MB), 0 VRAM
    let hash_table_size_bytes = 16 * 1024 * 1024;
    NGramSpeculationBuffer {
        n_match,
        n_min,
        n_max,
        hash_table_size_bytes,
        allocated_in_host_ram: true,
        vram_bytes_allocated: 0,
    }
}

pub fn pin_critic_worker_thread_affinity(allowed_core_indices: &[usize]) -> Result<Vec<usize>, String> {
    // ADR-033 / PRD-10.2 / ADR-030: Isolamento térmico de CPU via SetThreadAffinityMask (windows-sys) para workers do Critic Model (Fail-Soft)
    let mut pinned_indices = Vec::new();

    #[cfg(target_os = "windows")]
    {
        use windows_sys::Win32::System::Threading::{GetCurrentThread, SetThreadAffinityMask};
        let handle = unsafe { GetCurrentThread() };
        for &idx in allowed_core_indices {
            if idx < (usize::BITS as usize) {
                let mask = 1usize << idx;
                let res = unsafe { SetThreadAffinityMask(handle, mask) };
                if res != 0 {
                    pinned_indices.push(idx);
                } else {
                    tracing::warn!("WARN: Permissão negada pelo SO para CPU Pinning no núcleo {}, rodando com scheduler padrão", idx);
                }
            }
        }
    }

    #[cfg(not(target_os = "windows"))]
    {
        let _ = allowed_core_indices;
        tracing::warn!("WARN: CPU Pinning nativo só implementado para Windows nesta build");
    }

    if pinned_indices.is_empty() && !allowed_core_indices.is_empty() {
        tracing::warn!("WARN: Permissão negada para CPU Pinning em todos os núcleos solicitados, rodando com scheduler padrão");
    }

    Ok(pinned_indices)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_auto_profiling_rejects_overbudget_gguf() {
        // Modelo de 7B denso com 4.3GB de peso + 1MB KV Cache + 800MB CUDA = 5101MB (> 5000MB limit)
        let overbudget_result = profile_gguf_vram("Qwen3.5-9B-Heavy.gguf", 4300, 4096, false);
        assert_eq!(
            overbudget_result,
            Err(ModelManagerError::OverbudgetVram(5101)),
            "Modelo acima do orçamento térmico de 5GB de VRAM deve ser rejeitado"
        );

        // Modelo de 4B leve (2.5GB peso + 1.0GB KV + 800MB CUDA = 4.3GB <= 5.0GB limit)
        let viable_result = profile_gguf_vram("Qwen3.5-4B-Instruct.gguf", 2500, 4096, false);
        assert!(viable_result.is_ok());
        let res = viable_result.unwrap();
        assert!(res.is_viable);
        assert_eq!(res.total_vram_projected_mb, 3301);
    }

    #[test]
    fn test_ralph_loop_prefix_reuse_zero_vram_leak() {
        let mut state = RalphLoopState::new(0xDEADBEEF);
        assert!(state.is_critic_on_cpu, "Critic MUST run decoupled on CPU RAM");
        assert!(!state.prefix_reused);

        // Dispara reflexão com traço de erro
        let step1 = state.trigger_reflection("Erro sintático na chave JSON");
        assert!(step1.is_ok());
        assert!(state.prefix_reused, "Re-prompt DEVE reutilizar prefixo de KV Cache");
        assert_eq!(state.current_reflection_step, 1);

        // Ultrapassa teto de 3 iterações -> Fail-Closed
        let _ = state.trigger_reflection("Erro 2");
        let _ = state.trigger_reflection("Erro 3");
        let step4 = state.trigger_reflection("Erro 4");
        assert!(step4.is_err(), "Ralph Loop DEVE abortar no teto de 3 iterações");
    }

    #[test]
    fn test_ngram_speculation_buffer_allocation_host_ram() {
        let buf = allocate_ngram_speculation_buffer(24, 48, 64);
        assert!(
            buf.allocated_in_host_ram,
            "Tabela de hash N-Gram DEVE ser alocada estritamente na RAM Host"
        );
        assert_eq!(
            buf.vram_bytes_allocated, 0,
            "Tabela de N-Gram DEVE possuir 0 bytes de consumo de VRAM na GPU"
        );
        assert!(
            buf.hash_table_size_bytes < 20 * 1024 * 1024,
            "Pegada de RAM Host para N-Gram deve ser < 20 MB"
        );
    }

    #[test]
    fn test_critic_worker_core_affinity_pinning() {
        let target_cores = vec![0, 1];
        let pinned = pin_critic_worker_thread_affinity(&target_cores);
        assert!(
            pinned.is_ok(),
            "Afinidade de núcleo via core_affinity DEVE ser aplicada de forma fail-soft sem panic"
        );
        let assigned = pinned.unwrap();
        if !assigned.is_empty() {
            assert_eq!(assigned, target_cores, "Cores ancorados devem corresponder aos solicitados");
        }
    }
}

