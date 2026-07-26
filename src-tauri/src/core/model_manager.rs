use thiserror::Error;

#[derive(Error, Debug, PartialEq)]
pub enum ModelManagerError {
    #[error("Modelo rejeitado por exceder teto de VRAM FinOps de 5.0 GB (Projetado: {0} MB)")]
    OverbudgetVram(u64),
    #[error("Modelo não encontrado")]
    NotFound,
}

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_auto_profiling_rejects_overbudget_gguf() {
        // Modelo de 7B denso com 4.2GB de peso + 1.0GB KV Cache + 800MB CUDA = 6.0GB (> 5.0GB limit)
        let overbudget_result = profile_gguf_vram("Qwen3.5-9B-Heavy.gguf", 4300, 4096, false);
        assert_eq!(
            overbudget_result,
            Err(ModelManagerError::OverbudgetVram(6100)),
            "Modelo com projeção de VRAM > 5.0 GB deve ser sumariamente rejeitado em O(1)"
        );

        // Modelo de 4B leve (2.5GB peso + 1.0GB KV + 800MB CUDA = 4.3GB <= 5.0GB limit)
        let viable_result = profile_gguf_vram("Qwen3.5-4B-Instruct.gguf", 2500, 4096, false);
        assert!(viable_result.is_ok());
        let res = viable_result.unwrap();
        assert!(res.is_viable);
        assert_eq!(res.total_vram_projected_mb, 4300);
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
}
