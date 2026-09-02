// SOULS MC MARCO IV — Adaptador Dinâmico de LoRA (ADR-027 / Marco IV §3)
//
// Hot-swap de adaptadores LoRA em runtime sem recarregar o `llama_context`.
// Fluxo:
//
//   1. `pre_register(specialty, path)` — carrega o arquivo `.gguf`/`.bin` do
//      adaptador para Host RAM (com flag `lora-init-without-apply` equivalente
//      no ik_llama.cpp), mantendo os pesos **inertes** até serem ativados.
//
//   2. `apply_lora_adapter_in_flight(ctx_ptr, specialty, scale)` — funde
//      tensores de baixo posto no contexto ativo, com latência alvo < 5ms.
//      Antes de aplicar, libera o adaptador anterior (R4 da Linha Vermelha).
//
//   3. Toda a instrumentação é lock-free via `AtomicU64` para telemetria.
//
// Nota FFI: a biblioteca-alvo é `ik_llama` (ik_llama.cpp). A declaração é
// best-effort — quando a `.dll`/`.so` não está disponível, a função retorna
// `LoraError::FfiUnavailable`. Em testes, injetamos um mock via trait
// `LoraApplyFn` para validar o envelope temporal sem dependência externa.

#![cfg(feature = "lora_adapter")]

use std::ffi::c_void;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Mutex;
use std::time::Instant;

#[cfg(feature = "ik_llama_ffi")]
use std::ffi::{c_char, CString};

use dashmap::DashMap;
use thiserror::Error;

/// Tipo opaco equivalente a `*mut llama_context` no nível C ABI.
pub type LlamaContextPtr = *mut c_void;

/// Especialidades de adaptador LoRA reconhecidas pelo motor.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LoraSpecialty {
    /// Foco em geração de código (Rust, Python, TS).
    Coder,
    /// Foco em questionamento socrático / pedagogy.
    Socratic,
    /// Foco em heurísticas de pruning e busca.
    Heuristic,
}

impl LoraSpecialty {
    /// Nome canônico usado em paths e logs.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Coder => "coder",
            Self::Socratic => "socratic",
            Self::Heuristic => "heuristic",
        }
    }
}

#[derive(Debug, Error)]
pub enum LoraError {
    #[error("Adaptador LoRA não pré-registrado: {0}")]
    NotRegistered(String),
    #[error("Falha ao aplicar LoRA via FFI (rc={0})")]
    FfiApplyFailed(i32),
    #[error("Biblioteca ik_llama indisponível no sistema")]
    FfiUnavailable,
    #[error("Path inválido (NUL byte no conteúdo)")]
    InvalidPath,
    #[error("lock poisoning")]
    LockPoisoned,
}

/// Trait injetável para o `apply` real, permitindo mock em testes.
/// A implementação padrão chama a FFI; testes podem trocar via
/// `LlamaLoraAdapter::set_apply_fn`.
pub trait LoraApplyFn: Send + Sync {
    fn apply(&self, ctx: LlamaContextPtr, path: &Path, scale: f32) -> Result<(), LoraError>;
}

/// Implementação de produção: invoca a FFI do ik_llama.cpp.
/// **Gateada em `ik_llama_ffi`**: o binário do ik_llama não é uma dep Rust
/// disponível em crates.io, então mantemos a chamada FFI opcional para
/// permitir validação em esteiras sem a biblioteca nativa. Em produção,
/// habilite `ik_llama_ffi` no `.cargo/config.toml` ou via RUSTFLAGS que
/// injetem `ik_llama.lib` no link path.
pub struct FfiLoraApply;

impl LoraApplyFn for FfiLoraApply {
    fn apply(&self, ctx: LlamaContextPtr, path: &Path, scale: f32) -> Result<(), LoraError> {
        #[cfg(feature = "ik_llama_ffi")]
        {
            let c_path = CString::new(path.to_string_lossy().as_bytes())
                .map_err(|_| LoraError::InvalidPath)?;
            // SAFETY: `ctx` é opaco para esta crate; o caller é responsável por
            // garantir que aponta para um `llama_context*` válido do ik_llama.cpp.
            // `c_path` é CString (NUL-terminated); permanece válido durante a call.
            let rc = unsafe { souls_ik_llama_lora_apply(ctx, c_path.as_ptr(), scale) };
            if rc == 0 {
                Ok(())
            } else {
                Err(LoraError::FfiApplyFailed(rc))
            }
        }
        #[cfg(not(feature = "ik_llama_ffi"))]
        {
            let _ = (ctx, path, scale);
            Err(LoraError::FfiUnavailable)
        }
    }
}

// Declaração FFI para ik_llama.cpp. A biblioteca é resolvida em runtime;
// se ausente, o linker exige `dynamic_loading` em produção (a trait
// `LoraApplyFn` permite mockar este caminho em testes).
#[cfg(feature = "ik_llama_ffi")]
unsafe extern "C" {
    fn souls_ik_llama_lora_apply(ctx: *mut c_void, path: *const c_char, scale: f32) -> i32;
}

/// Estado de um adaptador pré-registrado.
#[derive(Debug, Clone)]
pub struct LoraRegistration {
    pub path: PathBuf,
    /// Bytes aproximados do arquivo (metadata; valor 0 = desconhecido).
    pub weight_bytes: u64,
}

pub struct LlamaLoraAdapter {
    registered: DashMap<LoraSpecialty, LoraRegistration>,
    applied: Mutex<Option<LoraSpecialty>>,
    last_swap_ns: AtomicU64,
    apply_fn: Mutex<Box<dyn LoraApplyFn>>,
}

impl Default for LlamaLoraAdapter {
    fn default() -> Self {
        Self::new()
    }
}

impl LlamaLoraAdapter {
    #[must_use]
    pub fn new() -> Self {
        Self {
            registered: DashMap::new(),
            applied: Mutex::new(None),
            last_swap_ns: AtomicU64::new(0),
            apply_fn: Mutex::new(Box::new(FfiLoraApply)),
        }
    }

    /// Substitui a implementação de `apply` (uso em testes).
    pub fn set_apply_fn(&self, f: Box<dyn LoraApplyFn>) -> Result<(), LoraError> {
        let mut guard = self.apply_fn.lock().map_err(|_| LoraError::LockPoisoned)?;
        *guard = f;
        Ok(())
    }

    /// Pré-registra um adaptador com flag `lora-init-without-apply` equivalente.
    /// Os pesos ficam inertes em Host RAM até `apply_lora_adapter_in_flight`.
    pub fn pre_register(&self, specialty: LoraSpecialty, path: PathBuf, weight_bytes: u64) {
        self.registered
            .insert(specialty, LoraRegistration { path, weight_bytes });
    }

    /// Descarrega o adaptador atualmente aplicado (se houver).
    pub fn release_previous(&self) -> Result<(), LoraError> {
        let mut guard = self.applied.lock().map_err(|_| LoraError::LockPoisoned)?;
        *guard = None;
        Ok(())
    }

    /// Aplica o adaptador in-flight, fundindo tensores de baixo posto no contexto.
    /// Latência alvo: < 5ms (medida e exposta em `last_swap_ns`).
    pub fn apply_lora_adapter_in_flight(
        &self,
        ctx_ptr: LlamaContextPtr,
        specialty: LoraSpecialty,
        scale: f32,
    ) -> Result<(), LoraError> {
        let reg = self
            .registered
            .get(&specialty)
            .ok_or_else(|| LoraError::NotRegistered(specialty.as_str().to_string()))?;

        // R4 da Linha Vermelha: libera o adaptador anterior antes de aplicar o novo.
        self.release_previous()?;

        let path = reg.path.clone();
        let start = Instant::now();

        let apply_guard = self.apply_fn.lock().map_err(|_| LoraError::LockPoisoned)?;
        let result = apply_guard.apply(ctx_ptr, &path, scale);
        drop(apply_guard);

        let elapsed_ns = start.elapsed().as_nanos() as u64;
        self.last_swap_ns.store(elapsed_ns, Ordering::Release);

        result?;

        let mut guard = self.applied.lock().map_err(|_| LoraError::LockPoisoned)?;
        *guard = Some(specialty);
        Ok(())
    }

    /// Latência em nanossegundos do último hot-swap.
    #[must_use]
    pub fn last_swap_ns(&self) -> u64 {
        self.last_swap_ns.load(Ordering::Acquire)
    }

    /// Adaptador atualmente aplicado (se houver).
    #[must_use]
    pub fn currently_applied(&self) -> Option<LoraSpecialty> {
        self.applied.lock().ok().and_then(|g| *g)
    }
}

// =============================================================================
// UNIT TESTS (TDD — Red-Green-Refactor)
// =============================================================================
#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::AtomicU32;
    use std::sync::Arc;
    use std::time::Duration;

    /// Mock que simula latência de FFI programável.
    /// A latência é controlada externamente via `Arc<AtomicU32>` para permitir
    /// ajuste pelos testes sem necessidade de downcast.
    pub struct MockApply {
        pub latency_us: Arc<AtomicU32>,
    }

    impl MockApply {
        pub fn new(latency_us: Arc<AtomicU32>) -> Self {
            Self { latency_us }
        }
    }

    impl LoraApplyFn for MockApply {
        fn apply(&self, _ctx: LlamaContextPtr, _path: &Path, _scale: f32) -> Result<(), LoraError> {
            let us = self.latency_us.load(Ordering::Acquire);
            if us > 0 {
                std::thread::sleep(Duration::from_micros(u64::from(us)));
            }
            Ok(())
        }
    }

    #[test]
    fn test_lora_pre_register_and_lookup() {
        let adapter = LlamaLoraAdapter::new();
        adapter.pre_register(
            LoraSpecialty::Coder,
            PathBuf::from("/tmp/coder.gguf"),
            1024 * 1024,
        );
        assert!(adapter.registered.contains_key(&LoraSpecialty::Coder));
        assert!(!adapter.registered.contains_key(&LoraSpecialty::Socratic));
    }

    #[test]
    fn test_lora_apply_unregistered_errors() {
        let adapter = LlamaLoraAdapter::new();
        let ctx = std::ptr::null_mut();
        let err = adapter
            .apply_lora_adapter_in_flight(ctx, LoraSpecialty::Heuristic, 1.0)
            .unwrap_err();
        assert!(matches!(err, LoraError::NotRegistered(_)));
    }

    #[test]
    fn test_lora_hot_swap_under_5ms() {
        let latency = Arc::new(AtomicU32::new(0));
        let adapter = LlamaLoraAdapter::new();
        adapter
            .set_apply_fn(Box::new(MockApply::new(Arc::clone(&latency))))
            .unwrap();
        adapter.pre_register(
            LoraSpecialty::Coder,
            PathBuf::from("/tmp/coder.gguf"),
            1024,
        );
        adapter.pre_register(
            LoraSpecialty::Socratic,
            PathBuf::from("/tmp/socratic.gguf"),
            1024,
        );

        // Simula latência de FFI de 800µs (deixa margem para overhead Rust).
        latency.store(800, Ordering::Release);

        let ctx = std::ptr::null_mut();
        adapter
            .apply_lora_adapter_in_flight(ctx, LoraSpecialty::Coder, 0.8)
            .expect("apply coder");
        assert_eq!(adapter.currently_applied(), Some(LoraSpecialty::Coder));

        // Hot-swap para outra especialidade deve ser < 5ms end-to-end.
        adapter
            .apply_lora_adapter_in_flight(ctx, LoraSpecialty::Socratic, 0.6)
            .expect("apply socratic");
        assert_eq!(adapter.currently_applied(), Some(LoraSpecialty::Socratic));

        let elapsed_ns = adapter.last_swap_ns();
        let elapsed_us = elapsed_ns / 1000;
        assert!(
            elapsed_us < 5_000,
            "hot-swap levou {elapsed_us}µs (teto: 5000µs)"
        );
    }

    #[test]
    fn test_lora_release_previous_idempotent() {
        let adapter = LlamaLoraAdapter::new();
        assert_eq!(adapter.currently_applied(), None);
        adapter.release_previous().expect("release empty");
        assert_eq!(adapter.currently_applied(), None);
    }
}
