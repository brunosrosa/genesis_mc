//! Context parameters ([`LlamaContextParams`]) over ik's `llama_context_params`.

use std::num::NonZeroU32;

use ik_llama_cpp_sys as sys;

/// The kind of context to create.
///
/// Mirrors `llama-cpp-2`'s `context::params::LlamaContextType`. ik has no
/// `llama_context_type` field — instead it toggles MTP via a `bool mtp`, so
/// this shim maps [`LlamaContextType::Mtp`] onto that flag.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum LlamaContextType {
    /// Default decoder context.
    Default,
    /// Multi-token-prediction (NextN) draft context.
    Mtp,
}

/// Parameters controlling a [`crate::LlamaContext`].
///
/// Starts from `llama_context_default_params()`. Note ik keeps a `seed` field in
/// the context params (stock removed it) and uses a `bool flash_attn` plus the
/// MTP fields `mtp` / `mtp_op_type`. There is no `ctx_type` in ik.
#[derive(Debug, Clone)]
pub struct LlamaContextParams {
    pub(crate) params: sys::llama_context_params,
}

impl Default for LlamaContextParams {
    fn default() -> Self {
        Self {
            params: unsafe { sys::llama_context_default_params() },
        }
    }
}

impl LlamaContextParams {
    /// Context size (tokens); `None` means "take it from the model" (0).
    ///
    /// Takes `Option<NonZeroU32>` to match `llama-cpp-2`.
    #[must_use]
    pub fn with_n_ctx(mut self, n_ctx: Option<NonZeroU32>) -> Self {
        self.params.n_ctx = n_ctx.map_or(0, NonZeroU32::get);
        self
    }

    /// Logical batch size.
    #[must_use]
    pub fn with_n_batch(mut self, n_batch: u32) -> Self {
        self.params.n_batch = n_batch;
        self
    }

    /// Physical (micro) batch size.
    #[must_use]
    pub fn with_n_ubatch(mut self, n_ubatch: u32) -> Self {
        self.params.n_ubatch = n_ubatch;
        self
    }

    /// Maximum number of sequences (distinct recurrent states).
    ///
    /// `llama-cpp-2`'s NextN fork exposes this as `n_rs_seq`; ik's equivalent is
    /// `n_seq_max`, which this sets.
    #[must_use]
    pub fn with_n_rs_seq(mut self, n_rs_seq: u32) -> Self {
        self.params.n_seq_max = n_rs_seq;
        self
    }

    /// Select the context kind. [`LlamaContextType::Mtp`] enables ik's MTP path
    /// (equivalent to [`Self::with_mtp(true)`](Self::with_mtp)).
    #[must_use]
    pub fn with_context_type(mut self, context_type: LlamaContextType) -> Self {
        self.params.mtp = matches!(context_type, LlamaContextType::Mtp);
        self
    }

    /// RNG seed (ik retains this in the context params).
    #[must_use]
    pub fn with_seed(mut self, seed: u32) -> Self {
        self.params.seed = seed;
        self
    }

    /// Threads used for generation.
    #[must_use]
    pub fn with_n_threads(mut self, n_threads: u32) -> Self {
        self.params.n_threads = n_threads;
        self
    }

    /// Threads used for batch/prompt processing.
    #[must_use]
    pub fn with_n_threads_batch(mut self, n_threads_batch: u32) -> Self {
        self.params.n_threads_batch = n_threads_batch;
        self
    }

    /// Enable flash attention.
    #[must_use]
    pub fn with_flash_attn(mut self, flash_attn: bool) -> Self {
        self.params.flash_attn = flash_attn;
        self
    }

    /// Activate the MTP path (requires a model loaded with `.with_mtp(true)`).
    #[must_use]
    pub fn with_mtp(mut self, mtp: bool) -> Self {
        self.params.mtp = mtp;
        self
    }

    /// Produce embeddings on decode (sets `params.embeddings`).
    ///
    /// Required for embedding and reranker models; usually paired with
    /// [`Self::with_pooling_type`].
    #[must_use]
    pub fn with_embeddings(mut self, embeddings: bool) -> Self {
        self.params.embeddings = embeddings;
        self
    }

    /// Set the pooling strategy for embeddings (sets `params.pooling_type`).
    ///
    /// Takes the raw `sys::llama_pooling_type` (e.g. `LLAMA_POOLING_TYPE_MEAN`,
    /// `LLAMA_POOLING_TYPE_CLS`, or `LLAMA_POOLING_TYPE_LAST`) to avoid
    /// introducing a new enum.
    #[must_use]
    pub fn with_pooling_type(mut self, pooling_type: sys::llama_pooling_type) -> Self {
        self.params.pooling_type = pooling_type;
        self
    }

    /// Access the raw params (advanced/escape hatch).
    #[must_use]
    pub fn as_raw(&self) -> &sys::llama_context_params {
        &self.params
    }

    // ========================================================================
    // SOULS MC Marco IV shims — parity with `llama-cpp-2` for TurboQuant + RoPE.
    // The ik wrapper upstream exposes neither K/V cache quant types nor RoPE
    // scaling knobs. We map them onto the C fields that already exist in
    // `llama_context_params` (verified in `ik_llama.cpp/include/llama.h`).
    // ========================================================================

    /// Set the K cache quantisation type (TurboQuant K-half).
    #[must_use]
    pub fn with_type_k(mut self, type_k: KvCacheType) -> Self {
        self.params.type_k = type_k.as_ggml_type();
        self
    }

    /// Set the V cache quantisation type (TurboQuant V-Q4_K).
    #[must_use]
    pub fn with_type_v(mut self, type_v: KvCacheType) -> Self {
        self.params.type_v = type_v.as_ggml_type();
        self
    }

    /// Set the RoPE scaling type (e.g. linear, YaRN).
    #[must_use]
    pub fn with_rope_scaling_type(mut self, scaling_type: RopeScalingType) -> Self {
        self.params.rope_scaling_type = scaling_type.as_sys();
        self
    }

    /// Set the RoPE frequency scaling factor (1.0 = no scaling).
    #[must_use]
    pub fn with_rope_freq_scale(mut self, factor: f32) -> Self {
        self.params.rope_freq_scale = factor;
        self
    }

    /// Set the YaRN attention factor (only meaningful when
    /// [`Self::with_rope_scaling_type`] is [`RopeScalingType::Yarn`]).
    #[must_use]
    pub fn with_yarn_attn_factor(mut self, factor: f32) -> Self {
        // ik's `llama_context_params` retains a `yarn_attn_factor` field
        // (the upstream llama.cpp dropped it). C shim via setattr.
        self.params.yarn_attn_factor = factor;
        self
    }
}

// ============================================================================
// SOULS MC Marco IV — `KvCacheType` enum (parity with `llama-cpp-2`)
// ============================================================================

/// K/V cache quantisation type. Maps onto `ggml_type`.
///
/// The TurboQuant pair used by SOULS MC is K=[`KvCacheType::F16`] (preserves
/// RoPE precision) + V=[`KvCacheType::Q4_K`] (compresses 4x). For 32k ctx on
/// the RTX 2060m this lands around ~800MB VRAM, vs ~1.6GB at F16/F16.
#[allow(non_camel_case_types)] // ggml uses `Q4_K`; we mirror the C identifier.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum KvCacheType {
    /// F16 (default; K-half for TurboQuant).
    F16,
    /// F32 (full precision, debug only).
    F32,
    /// Q4_0 (4-bit, 32-element block).
    Q4_0,
    /// Q4_K (4-bit k-quant, 256-element super-block; V-half for TurboQuant).
    Q4_K,
    /// Q5_0 (5-bit, 32-element block).
    Q5_0,
    /// Q5_1 (5-bit, 32-element block).
    Q5_1,
    /// Q8_0 (8-bit, 32-element block; SOULS MC fallback for non-256-aligned
    /// `n_embd_head_v` to avoid panics in the C-FFI).
    Q8_0,
}

impl KvCacheType {
    /// Map onto the raw `ggml_type` C enum from `ggml.h`.
    #[must_use]
    pub const fn as_ggml_type(self) -> sys::ggml_type {
        match self {
            Self::F16 => sys::GGML_TYPE_F16,
            Self::F32 => sys::GGML_TYPE_F32,
            Self::Q4_0 => sys::GGML_TYPE_Q4_0,
            Self::Q4_K => sys::GGML_TYPE_Q4_K,
            Self::Q5_0 => sys::GGML_TYPE_Q5_0,
            Self::Q5_1 => sys::GGML_TYPE_Q5_1,
            Self::Q8_0 => sys::GGML_TYPE_Q8_0,
        }
    }
}

impl Default for KvCacheType {
    fn default() -> Self {
        Self::F16
    }
}

// ============================================================================
// SOULS MC Marco IV — `RopeScalingType` enum (parity with `llama-cpp-2`)
// ============================================================================

/// RoPE scaling strategy. Maps onto `enum llama_rope_scaling_type`.
///
/// See `llama.h` line 250-256 for the canonical enum (NONE=0, LINEAR=1, YARN=2).
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum RopeScalingType {
    /// No RoPE scaling (default).
    None,
    /// Linear interpolation (compresses position range linearly).
    Linear,
    /// YaRN (yet another RoPE extensioN).
    Yarn,
    /// Unspecified / pass-through. The C side treats `0xFF` (or any value the
    /// upstream does not name) as a no-op; SOULS MC uses this for
    /// forward-compat with upstream llama.cpp additions like `MaxPos`.
    Unspecified,
}

impl RopeScalingType {
    /// Map onto the raw `llama_rope_scaling_type` C enum.
    #[must_use]
    pub const fn as_sys(self) -> sys::llama_rope_scaling_type {
        match self {
            Self::None => sys::LLAMA_ROPE_SCALING_TYPE_NONE,
            Self::Linear => sys::LLAMA_ROPE_SCALING_TYPE_LINEAR,
            Self::Yarn => sys::LLAMA_ROPE_SCALING_TYPE_YARN,
            // ik's bindgen emits `_UNSPECIFIED = -1` (see `llama.h:251`), so we
            // forward to that variant rather than collapsing to `NONE`. Keeps
            // forward-compat with upstream additions like `MaxPos`.
            Self::Unspecified => sys::LLAMA_ROPE_SCALING_TYPE_UNSPECIFIED,
        }
    }
}

impl Default for RopeScalingType {
    fn default() -> Self {
        Self::None
    }
}
