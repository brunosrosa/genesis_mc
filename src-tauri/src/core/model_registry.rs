use std::collections::HashSet;
use std::env;
use std::fs::{self, File};
use std::path::{Path, PathBuf};
use memmap2::MmapOptions;
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use walkdir::WalkDir;
use crate::core::engine_trait::{EngineCascade, EngineSupportLevel, TopologyFeatures, FileFormat, AttentionType, RopeScalingType};

/// Profundidade máxima rígida de navegação (Marco 4.10.1 ETAPA 2).
/// Limite de 4 níveis previne recursão acidental em árvores de modelo
/// profundamente aninhadas e mantém a varredura O(n) sobre modelos reais.
pub const MAX_MODEL_WALK_DEPTH: usize = 4;

/// Iterator seguro para varredura de diretórios de modelos.
///
/// Impondo `max_depth(MAX_MODEL_WALK_DEPTH)` e detectando explicitamente
/// symlinks circulares via `fs::canonicalize` + `HashSet` de paths canônicos
/// visitados. Substitui o uso direto de `WalkDir::into_iter().flatten()`
/// que silenciosamente dropa erros (incluindo loops de symlink).
///
/// Comportamento fail-soft: erros de I/O são logados e o item é pulado.
pub struct SafeModelWalk {
    inner: walkdir::IntoIter,
    visited_canonical: HashSet<PathBuf>,
    root_canonical: Option<PathBuf>,
}

impl SafeModelWalk {
    pub fn new(root: &Path) -> Self {
        // Tenta canocalizar a raiz; se falhar (path inexistente), usa o
        // path absoluto como fallback.
        let root_canonical = fs::canonicalize(root).ok();
        Self {
            inner: WalkDir::new(root)
                .max_depth(MAX_MODEL_WALK_DEPTH)
                .follow_links(false)
                .into_iter(),
            visited_canonical: HashSet::new(),
            root_canonical,
        }
    }
}

impl Iterator for SafeModelWalk {
    type Item = walkdir::DirEntry;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            let entry = match self.inner.next() {
                Some(Ok(e)) => e,
                Some(Err(e)) => {
                    // Log explícito do erro (não silent drop) e continua.
                    eprintln!("[SOULS-WALK] pulando entrada com erro: {e}");
                    continue;
                }
                None => return None,
            };
            let path = entry.path();
            // Detecção de symlink loop: canocaliza e checa se já visitamos.
            // Em Windows, `fs::canonicalize` resolve o symlink e devolve o
            // destino real. Se o destino for igual à raiz canônica ou a um
            // path já visitado, é um loop → skip.
            if path.is_symlink() {
                if let Ok(canonical) = fs::canonicalize(path) {
                    if let Some(ref root) = self.root_canonical {
                        if canonical == *root {
                            eprintln!("[SOULS-WALK] pulando symlink loop para raiz: {}", path.display());
                            continue;
                        }
                    }
                    if !self.visited_canonical.insert(canonical) {
                        eprintln!("[SOULS-WALK] pulando symlink circular: {}", path.display());
                        continue;
                    }
                }
            }
            return Some(entry);
        }
    }
}

/// Helper de conveniência: itera sobre entradas válidas de modelos.
pub fn safe_walk_models(root: &Path) -> SafeModelWalk {
    SafeModelWalk::new(root)
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct ModelArchitectureParams {
    pub block_count: u32,         // n_layer
    pub embedding_length: u32,    // n_embd
    pub head_count: u32,          // n_head
    pub head_count_kv: u32,       // n_head_kv
    pub feed_forward_length: u32, // n_ff
    pub rope_scaling_attn_factor: Option<f32>,
    pub chat_template: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[allow(non_camel_case_types)]
pub enum KvQuantPrecision {
    F16,
    Q8_0,
    Q4_K,
    Q4_0,
    Custom(f32),
}

impl KvQuantPrecision {
    pub fn bytes_per_element(&self) -> f32 {
        match self {
            KvQuantPrecision::F16 => 2.0,
            KvQuantPrecision::Q8_0 => 1.0,
            KvQuantPrecision::Q4_K | KvQuantPrecision::Q4_0 => 0.5,
            KvQuantPrecision::Custom(b) => *b,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VramProfileEstimate {
    pub weights_vram_bytes: u64,
    pub kv_cache_vram_bytes: u64,
    pub compute_scratch_vram_bytes: u64,
    pub lora_overhead_vram_bytes: u64,
    pub total_estimated_vram_bytes: u64,
    pub max_supported_context: u64,
    pub fits_in_vram: bool,
    pub k_precision_bytes: f32,
    pub v_precision_bytes: f32,
    pub batch_size: u32,
    pub supports_flash_attention: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ModelMetadata {
    pub file_path: String,
    pub model_name: String,
    pub family: String,
    pub parameters: String,
    pub context_length: u64,
    pub quantization: String,
    pub capabilities: Vec<String>,
    pub file_size_bytes: u64,
    pub is_active: bool,
    pub tier1_passed: bool,
    pub success_rate_ema: f64,
    pub ema_latency_ms: f64,
    pub architecture: ModelArchitectureParams,
}

/// Calcula a estimativa termodinâmica de consumo de VRAM (pesos + KV cache assimétrico + scratch O(N)/O(N^2) + LoRA) em O(1), desmembrada de forma agnóstica para qualquer limite dinâmico de VRAM.
#[allow(clippy::too_many_arguments)]
pub fn estimate_vram_thermodynamics(
    meta: &ModelMetadata,
    target_context_len: u64,
    offload_ratio: f32,
    k_precision_bytes: f32,
    v_precision_bytes: f32,
    batch_size: u32,
    supports_flash_attention: bool,
    active_lora_overhead_bytes: u64,
    available_vram_bytes: u64,
) -> VramProfileEstimate {
    let weights_vram = (meta.file_size_bytes as f32 * offload_ratio.clamp(0.0, 1.0)) as u64;

    let arch = &meta.architecture;
    let layers = if arch.block_count > 0 { arch.block_count as u64 } else { 32 };
    let head_count = if arch.head_count > 0 { arch.head_count as u64 } else { 16 };
    let head_count_kv = if arch.head_count_kv > 0 { arch.head_count_kv as u64 } else { head_count };
    let embd = if arch.embedding_length > 0 { arch.embedding_length as u64 } else { 2560 };
    let head_dim = embd / head_count;

    let effective_batch = if batch_size > 0 { batch_size as u64 } else { 1 };
    let total_kv_bytes_per_elem = k_precision_bytes + v_precision_bytes;

    // Formula Bare-Metal Assimétrica (ADR-027): 
    // layers * n_head_kv * head_dim * context_len * batch_size * (k_bytes + v_bytes)
    let kv_cache_vram = (layers as f32
        * (head_count_kv as f32)
        * (head_dim as f32)
        * (target_context_len as f32)
        * (effective_batch as f32)
        * total_kv_bytes_per_elem) as u64;

    let base_scratch = 256 * 1024 * 1024; // 256 MB base CUDA graph / activation buffers

    // Bifurcação Termodinâmica: Flash Attention O(N) tile-based vs Atenção Tradicional O(N^2)
    let ctx_scratch = if supports_flash_attention {
        // Flash Attention O(N): tile-based scratch
        target_context_len * head_count * 64 * effective_batch
    } else {
        // Atenção Tradicional O(N^2): Matriz de pontuação de atenção (QK^T em FP16 = 2 bytes)
        target_context_len * target_context_len * head_count * 2 * effective_batch
    };

    let compute_scratch = base_scratch + ctx_scratch;

    let total = weights_vram + kv_cache_vram + compute_scratch + active_lora_overhead_bytes;

    VramProfileEstimate {
        weights_vram_bytes: weights_vram,
        kv_cache_vram_bytes: kv_cache_vram,
        compute_scratch_vram_bytes: compute_scratch,
        lora_overhead_vram_bytes: active_lora_overhead_bytes,
        total_estimated_vram_bytes: total,
        max_supported_context: meta.context_length,
        fits_in_vram: total <= available_vram_bytes,
        k_precision_bytes,
        v_precision_bytes,
        batch_size: effective_batch as u32,
        supports_flash_attention,
    }
}

/// Conecta a topologia de hardware detectada dinamicamente ao calculador termodinâmico de VRAM.
#[allow(clippy::too_many_arguments)]
pub fn estimate_vram_for_topology(
    meta: &ModelMetadata,
    target_context_len: u64,
    offload_ratio: f32,
    k_precision_bytes: f32,
    v_precision_bytes: f32,
    batch_size: u32,
    supports_flash_attention: bool,
    active_lora_overhead_bytes: u64,
    topology: &crate::core::hardware_profiler::SystemTopology,
) -> VramProfileEstimate {
    estimate_vram_thermodynamics(
        meta,
        target_context_len,
        offload_ratio,
        k_precision_bytes,
        v_precision_bytes,
        batch_size,
        supports_flash_attention,
        active_lora_overhead_bytes,
        topology.vram_total_bytes,
    )
}

struct ByteCursor<'a> {
    data: &'a [u8],
    pos: usize,
}

impl<'a> ByteCursor<'a> {
    fn new(data: &'a [u8]) -> Self {
        Self { data, pos: 0 }
    }

    fn has_remaining(&self, count: usize) -> bool {
        self.pos.checked_add(count).is_some_and(|end| end <= self.data.len())
    }

    fn read_u32(&mut self) -> Option<u32> {
        if !self.has_remaining(4) {
            return None;
        }
        let bytes = &self.data[self.pos..self.pos + 4];
        self.pos += 4;
        let arr: [u8; 4] = bytes.try_into().ok()?;
        Some(u32::from_le_bytes(arr))
    }

    fn read_u64(&mut self) -> Option<u64> {
        if !self.has_remaining(8) {
            return None;
        }
        let bytes = &self.data[self.pos..self.pos + 8];
        self.pos += 8;
        let arr: [u8; 8] = bytes.try_into().ok()?;
        Some(u64::from_le_bytes(arr))
    }

    fn read_f32(&mut self) -> Option<f32> {
        if !self.has_remaining(4) {
            return None;
        }
        let bytes = &self.data[self.pos..self.pos + 4];
        self.pos += 4;
        let arr: [u8; 4] = bytes.try_into().ok()?;
        Some(f32::from_le_bytes(arr))
    }

    fn read_slice(&mut self, len: usize) -> Option<&'a [u8]> {
        if !self.has_remaining(len) {
            return None;
        }
        let slice = &self.data[self.pos..self.pos + len];
        self.pos += len;
        Some(slice)
    }

    fn read_string(&mut self) -> Option<String> {
        let len = self.read_u64()? as usize;
        let bytes = self.read_slice(len)?;
        Some(String::from_utf8_lossy(bytes).to_string())
    }

    fn skip_value(&mut self, val_type: u32) -> bool {
        match val_type {
            0 | 1 | 7 => self.read_slice(1).is_some(),
            2 | 3 => self.read_slice(2).is_some(),
            4..=6 => self.read_slice(4).is_some(),
            10..=12 => self.read_slice(8).is_some(),
            8 => self.read_string().is_some(),
            9 => {
                let elem_type = match self.read_u32() {
                    Some(t) => t,
                    None => return false,
                };
                let arr_len = match self.read_u64() {
                    Some(l) => l,
                    None => return false,
                };
                for _ in 0..arr_len {
                    if !self.skip_value(elem_type) {
                        return false;
                    }
                }
                true
            }
            _ => false,
        }
    }
}

use dashmap::DashMap;
use std::sync::LazyLock;

/// Cache transiliente L1/L2/L3 de metadados GGUF O(1) para erradicar o Double-I/O Bug (ADR-010 / ADR-027 / Marco 5.1.0)
pub struct GgufMetadataCache {
    cache: DashMap<String, ModelMetadata>,
}

impl GgufMetadataCache {
    pub fn new() -> Self {
        Self {
            cache: DashMap::new(),
        }
    }

    /// Obtém metadados do modelo via fluxo em 3 níveis (L1 RAM -> L2 SQLite WAL -> L3 single mmap).
    /// GARGALO ZERO DE SYSCALL: no L1 Hit, nenhuma chamada a `fs::metadata` ou `dunce::canonicalize` é realizada!
    pub fn get_or_parse(&self, file_path: &Path) -> Option<ModelMetadata> {
        let raw_key = file_path.to_string_lossy().to_string();

        // 1. L1 RAM Hit (O(1) em RAM sem nenhuma Syscall de sistema de arquivos)
        if let Some(entry) = self.cache.get(&raw_key) {
            return Some(entry.value().clone());
        }

        let canon_key = dunce::canonicalize(file_path)
            .map(|p| p.to_string_lossy().to_string())
            .ok();

        if let Some(ref ck) = canon_key {
            if let Some(entry) = self.cache.get(ck) {
                let meta = entry.value().clone();
                self.cache.insert(raw_key, meta.clone());
                return Some(meta);
            }
        }

        // 2. L2 SQLite Hit (souls_state.db WAL)
        if let Some(meta) = self.load_from_sqlite_l2(&raw_key, canon_key.as_deref()) {
            self.cache.insert(raw_key.clone(), meta.clone());
            if let Some(ck) = canon_key {
                if ck != raw_key {
                    self.cache.insert(ck, meta.clone());
                }
            }
            return Some(meta);
        }

        // 3. L3 Miss: Single Zero-Copy mmap do cabeçalho GGUF v3 via memmap2
        let parsed = parse_gguf_metadata_zero_copy_uncached(file_path)?;

        // Persiste na base relacional souls_state.db (L2) e hidrata o DashMap L1 em RAM
        self.persist_to_sqlite_l2(&parsed);
        self.cache.insert(raw_key.clone(), parsed.clone());
        if let Some(ck) = canon_key {
            if ck != raw_key {
                self.cache.insert(ck, parsed.clone());
            }
        }

        Some(parsed)
    }

    fn load_from_sqlite_l2(&self, raw_key: &str, canon_key: Option<&str>) -> Option<ModelMetadata> {
        let db_path = resolve_db_path();
        if !db_path.exists() {
            return None;
        }

        let conn = rusqlite::Connection::open(&db_path).ok()?;
        let _ = conn.busy_timeout(std::time::Duration::from_secs(2));

        let query_key = canon_key.unwrap_or(raw_key);

        let mut stmt = conn.prepare(
            "SELECT file_path, model_name, family, parameters, context_length, quantization, capabilities, file_size_bytes, is_active, topology_json
             FROM model_registry
             WHERE file_path = ?1 OR file_path = ?2"
        ).ok()?;

        let row = stmt.query_row(params![raw_key, query_key], |r| {
            let file_path: String = r.get(0)?;
            let model_name: String = r.get(1)?;
            let family: String = r.get(2)?;
            let parameters: String = r.get(3)?;
            let context_length: u64 = r.get::<_, i64>(4)? as u64;
            let quantization: String = r.get(5)?;
            let caps_raw: String = r.get(6)?;
            let file_size_bytes: u64 = r.get::<_, i64>(7)? as u64;
            let is_active: bool = r.get::<_, i32>(8)? != 0;
            let topology_raw: String = r.get(9)?;
            Ok((file_path, model_name, family, parameters, context_length, quantization, caps_raw, file_size_bytes, is_active, topology_raw))
        }).ok()?;

        let capabilities: Vec<String> = serde_json::from_str(&row.6).unwrap_or_default();
        let architecture: ModelArchitectureParams = if let Ok(tf) = serde_json::from_str::<TopologyFeatures>(&row.9) {
            ModelArchitectureParams {
                block_count: tf.block_count,
                embedding_length: tf.embedding_length,
                head_count: tf.head_count,
                head_count_kv: tf.head_count_kv,
                feed_forward_length: 0,
                rope_scaling_attn_factor: if tf.rope_scaling == RopeScalingType::Linear { Some(1.0) } else { None },
                chat_template: tf.chat_template.unwrap_or_default(),
            }
        } else {
            ModelArchitectureParams::default()
        };

        Some(ModelMetadata {
            file_path: row.0,
            model_name: row.1,
            family: row.2,
            parameters: row.3,
            context_length: row.4,
            quantization: row.5,
            capabilities,
            file_size_bytes: row.7,
            is_active: row.8,
            tier1_passed: false,
            success_rate_ema: 0.0,
            ema_latency_ms: 0.0,
            architecture,
        })
    }

    fn persist_to_sqlite_l2(&self, meta: &ModelMetadata) {
        let db_path = resolve_db_path();
        let conn = match init_model_registry_db(&db_path) {
            Ok(c) => c,
            Err(_) => return,
        };

        let caps_json = serde_json::to_string(&meta.capabilities).unwrap_or_else(|_| "[]".to_string());
        let tf = build_topology_features_from_meta(meta);
        let topology_json = serde_json::to_string(&tf).unwrap_or_else(|_| "{}".to_string());
        let actual_mod_type = infer_module_type(&meta.model_name, &meta.family);

        let _ = conn.execute(
            "INSERT INTO model_registry (file_path, model_name, family, parameters, context_length, quantization, capabilities, file_size_bytes, is_active, module_type, topology_json, last_seen)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, DATETIME('now'))
             ON CONFLICT(file_path) DO UPDATE SET
                model_name=excluded.model_name,
                family=excluded.family,
                parameters=excluded.parameters,
                context_length=excluded.context_length,
                quantization=excluded.quantization,
                capabilities=excluded.capabilities,
                file_size_bytes=excluded.file_size_bytes,
                is_active=excluded.is_active,
                module_type=excluded.module_type,
                topology_json=excluded.topology_json,
                last_seen=DATETIME('now');",
            params![
                meta.file_path,
                meta.model_name,
                meta.family,
                meta.parameters,
                meta.context_length as i64,
                meta.quantization,
                caps_json,
                meta.file_size_bytes as i64,
                if meta.is_active { 1 } else { 0 },
                actual_mod_type,
                topology_json,
            ],
        );
    }

    pub fn clear(&self) {
        self.cache.clear();
    }

    pub fn len(&self) -> usize {
        self.cache.len()
    }

    pub fn is_empty(&self) -> bool {
        self.cache.is_empty()
    }
}

impl Default for GgufMetadataCache {
    fn default() -> Self {
        Self::new()
    }
}

pub static GLOBAL_GGUF_METADATA_CACHE: LazyLock<GgufMetadataCache> = LazyLock::new(GgufMetadataCache::new);

/// Extrai metadados do cabeçalho GGUF em O(1) e Zero-Copy via cache RAM (sem mmap2 duplicado).
pub fn parse_gguf_metadata_zero_copy(file_path: &Path) -> Option<ModelMetadata> {
    GLOBAL_GGUF_METADATA_CACHE.get_or_parse(file_path)
}

/// Extrai metadados do cabeçalho GGUF em O(1) e Zero-Copy via memmap2 (sem carregar tensores em RAM).
pub fn parse_gguf_metadata_zero_copy_uncached(file_path: &Path) -> Option<ModelMetadata> {
    let f = File::open(file_path).ok()?;
    let meta = f.metadata().ok()?;
    let file_size = meta.len();
    if file_size < 24 {
        return None;
    }

    let mmap = unsafe { MmapOptions::new().map(&f).ok()? };
    if mmap.len() < 24 || &mmap[..4] != b"GGUF" {
        return None;
    }

    let mut cursor = ByteCursor::new(&mmap);
    cursor.pos = 4; // Skip magic

    let _version = cursor.read_u32()?;
    let _tensor_count = cursor.read_u64()?;
    let kv_count = cursor.read_u64()?;

    let mut family = String::new();
    let mut name = String::new();
    let mut chat_template = String::new();
    let mut context_length: u64 = 4096;
    let mut file_type_enum: Option<u32> = None;
    let mut arch_params = ModelArchitectureParams::default();

    for _ in 0..kv_count {
        let key = match cursor.read_string() {
            Some(k) => k,
            None => break,
        };
        let val_type = match cursor.read_u32() {
            Some(t) => t,
            None => break,
        };

        if key == "general.architecture" && val_type == 8 {
            if let Some(s) = cursor.read_string() {
                family = s;
            }
        } else if key == "general.name" && val_type == 8 {
            if let Some(s) = cursor.read_string() {
                name = s;
            }
        } else if (key == "tokenizer.chat_template" || key == "general.chat_template") && val_type == 8 {
            if let Some(s) = cursor.read_string() {
                chat_template = s;
            }
        } else if key.ends_with(".context_length") || key == "general.context_length" {
            if val_type == 4 || val_type == 2 {
                if let Some(val) = cursor.read_u32() {
                    context_length = val as u64;
                }
            } else if val_type == 10 {
                if let Some(val) = cursor.read_u64() {
                    context_length = val;
                }
            } else {
                cursor.skip_value(val_type);
            }
        } else if key.ends_with(".block_count") || key == "general.block_count" {
            if val_type == 4 || val_type == 2 {
                if let Some(val) = cursor.read_u32() {
                    arch_params.block_count = val;
                }
            } else if val_type == 10 {
                if let Some(val) = cursor.read_u64() {
                    arch_params.block_count = val as u32;
                }
            } else {
                cursor.skip_value(val_type);
            }
        } else if key.ends_with(".embedding_length") || key == "general.embedding_length" {
            if val_type == 4 || val_type == 2 {
                if let Some(val) = cursor.read_u32() {
                    arch_params.embedding_length = val;
                }
            } else if val_type == 10 {
                if let Some(val) = cursor.read_u64() {
                    arch_params.embedding_length = val as u32;
                }
            } else {
                cursor.skip_value(val_type);
            }
        } else if key.ends_with(".attention.head_count") {
            if val_type == 4 || val_type == 2 {
                if let Some(val) = cursor.read_u32() {
                    arch_params.head_count = val;
                }
            } else if val_type == 10 {
                if let Some(val) = cursor.read_u64() {
                    arch_params.head_count = val as u32;
                }
            } else {
                cursor.skip_value(val_type);
            }
        } else if key.ends_with(".attention.head_count_kv") {
            if val_type == 4 || val_type == 2 {
                if let Some(val) = cursor.read_u32() {
                    arch_params.head_count_kv = val;
                }
            } else if val_type == 10 {
                if let Some(val) = cursor.read_u64() {
                    arch_params.head_count_kv = val as u32;
                }
            } else {
                cursor.skip_value(val_type);
            }
        } else if key.ends_with(".feed_forward_length") {
            if val_type == 4 || val_type == 2 {
                if let Some(val) = cursor.read_u32() {
                    arch_params.feed_forward_length = val;
                }
            } else if val_type == 10 {
                if let Some(val) = cursor.read_u64() {
                    arch_params.feed_forward_length = val as u32;
                }
            } else {
                cursor.skip_value(val_type);
            }
        } else if key.ends_with(".rope.scaling.attn_factor") {
            if val_type == 6 || val_type == 5 || val_type == 4 {
                arch_params.rope_scaling_attn_factor = cursor.read_f32();
            } else {
                cursor.skip_value(val_type);
            }
        } else if key == "general.file_type" && (val_type == 4 || val_type == 2) {
            if let Some(val) = cursor.read_u32() {
                file_type_enum = Some(val);
            }
        } else {
            if !cursor.skip_value(val_type) {
                break;
            }
        }
    }

    let filename = file_path.file_name()?.to_string_lossy();
    let parent_dir = file_path
        .parent()
        .and_then(|p| p.file_name())
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_default();

    let raw_model_name = if !name.trim().is_empty() {
        name.trim().to_string()
    } else {
        file_path
            .file_stem()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_else(|| "UnknownModel".to_string())
    };

    let model_name = format_model_canonical_name(&raw_model_name, Some(&parent_dir));

    if family.is_empty() {
        family = infer_family(&filename, &model_name);
    }
    let parameters = infer_params(&parent_dir, &filename, &model_name);
    let quantization = file_type_enum
        .map(file_type_to_quant_str)
        .unwrap_or_else(|| "GGUF".to_string());

    let capabilities = infer_capabilities(&family, &model_name, &filename, &chat_template);
    arch_params.chat_template = chat_template;

    Some(ModelMetadata {
        file_path: file_path.to_string_lossy().to_string(),
        model_name,
        family,
        parameters,
        context_length,
        quantization,
        capabilities,
        file_size_bytes: file_size,
        is_active: true,
        tier1_passed: false,
        success_rate_ema: 0.0,
        ema_latency_ms: 0.0,
        architecture: arch_params,
    })
}

/// Extrai metadados do cabeçalho Safetensors em O(1) e Zero-Copy via mmap.
pub fn parse_safetensors_metadata_zero_copy(file_path: &Path) -> Option<ModelMetadata> {
    let f = File::open(file_path).ok()?;
    let meta = f.metadata().ok()?;
    let file_size = meta.len();
    if file_size < 8 {
        return None;
    }

    let mmap = unsafe { MmapOptions::new().map(&f).ok()? };
    if mmap.len() < 8 {
        return None;
    }
    let header_bytes: [u8; 8] = mmap[..8].try_into().ok()?;
    let header_len = u64::from_le_bytes(header_bytes) as usize;
    if mmap.len() < 8 + header_len {
        return None;
    }

    let json_bytes = &mmap[8..8 + header_len];
    let val: serde_json::Value = serde_json::from_slice(json_bytes).ok()?;
    let filename = file_path.file_name()?.to_string_lossy();
    let parent_dir = file_path
        .parent()
        .and_then(|p| p.file_name())
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_default();

    let model_name = filename.to_string();
    let family = infer_family(&filename, "");
    let parameters = infer_params(&parent_dir, &filename, "");
    let quantization = "BF16/F16".to_string();

    let mut context_length = 4096;
    if let Some(meta_val) = val.get("__metadata__") {
        if let Some(ctx) = meta_val.get("max_position_embeddings") {
            if let Some(c) = ctx.as_u64() {
                context_length = c;
            }
        }
    }

    let capabilities = infer_capabilities(&family, &model_name, &filename, "");

    Some(ModelMetadata {
        file_path: file_path.to_string_lossy().to_string(),
        model_name,
        family,
        parameters,
        context_length,
        quantization,
        capabilities,
        file_size_bytes: file_size,
        is_active: true,
        tier1_passed: false,
        success_rate_ema: 0.0,
        ema_latency_ms: 0.0,
        architecture: ModelArchitectureParams::default(),
    })
}

fn file_type_to_quant_str(ft: u32) -> String {
    match ft {
        0 => "F32",
        1 => "F16",
        2 => "Q4_0",
        3 => "Q4_1",
        7 => "Q8_0",
        8 => "Q5_0",
        9 => "Q5_1",
        10 => "Q2_K",
        11 => "Q3_K_S",
        12 => "Q3_K_M",
        13 => "Q3_K_L",
        14 => "Q4_K_S",
        15 => "Q4_K_M",
        16 => "Q5_K_S",
        17 => "Q5_K_M",
        18 => "Q6_K",
        28 => "BF16",
        _ => "GGUF_CUSTOM",
    }
    .to_string()
}

fn infer_family(filename: &str, name: &str) -> String {
    let combined = format!("{} {}", filename, name).to_lowercase();
    if combined.contains("qwen3.5") || combined.contains("qwen-3.5") {
        "Qwen3.5".to_string()
    } else if combined.contains("qwen") {
        "Qwen".to_string()
    } else if combined.contains("llama") {
        "Llama".to_string()
    } else if combined.contains("granite") {
        "Granite".to_string()
    } else if combined.contains("phi") {
        "Phi".to_string()
    } else {
        "Generic".to_string()
    }
}

fn infer_params(parent_dir: &str, filename: &str, name: &str) -> String {
    let combined = format!("{} {} {}", parent_dir, filename, name);
    let re_b = regex::Regex::new(r"(?i)\b(\d+(?:\.\d+)?)\s*[bB]\b").ok();
    if let Some(re) = re_b {
        if let Some(cap) = re.captures(&combined) {
            return format!("{}B", &cap[1]);
        }
    }
    "Unknown".to_string()
}

fn infer_capabilities(arch: &str, model_name: &str, filename: &str, chat_template: &str) -> Vec<String> {
    let mut caps = Vec::new();
    let combined = format!("{} {} {}", arch, model_name, filename).to_lowercase();
    let tpl_lower = chat_template.to_lowercase();

    if tpl_lower.contains("tools") || tpl_lower.contains("tool_call") || combined.contains("coder") {
        caps.push("TOOL_CALLING".to_string());
    }
    if caps.is_empty() {
        caps.push("BASE".to_string());
    }
    caps
}

pub fn resolve_db_path() -> PathBuf {
    let cwd = env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    let root = if cwd.ends_with("src-tauri") {
        cwd.parent().unwrap_or(&cwd).to_path_buf()
    } else {
        cwd
    };
    root.join(".souls_data").join("souls_heuristic_vault.db")
}

pub fn init_model_registry_db(db_path: &Path) -> Result<Connection, String> {
    if let Some(parent) = db_path.parent() {
        let _ = fs::create_dir_all(parent);
    }

    let conn = Connection::open(db_path)
        .map_err(|e| format!("Falha ao abrir souls_heuristic_vault.db: {e}"))?;

    let _ = conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA busy_timeout=5000;");

    conn.execute(
        "CREATE TABLE IF NOT EXISTS model_registry (
            file_path TEXT PRIMARY KEY,
            model_name TEXT NOT NULL DEFAULT '',
            family TEXT NOT NULL DEFAULT '',
            parameters TEXT NOT NULL DEFAULT '',
            context_length INTEGER NOT NULL DEFAULT 4096,
            quantization TEXT NOT NULL DEFAULT '',
            capabilities TEXT NOT NULL DEFAULT '[]',
            file_size_bytes INTEGER NOT NULL DEFAULT 0,
            is_active INTEGER NOT NULL DEFAULT 1,
            tier1_passed INTEGER NOT NULL DEFAULT 0,
            success_rate_ema REAL NOT NULL DEFAULT 0.0,
            ema_latency_ms REAL NOT NULL DEFAULT 0.0,
            module_type TEXT NOT NULL DEFAULT 'PRIMARY_LLM',
            last_seen TEXT NOT NULL DEFAULT (DATETIME('now'))
        );",
        [],
    )
    .map_err(|e| format!("Falha ao criar tabela model_registry: {e}"))?;

    // Migration idempotente para adicionar colunas em model_registry se não existirem
    let _ = conn.execute("ALTER TABLE model_registry ADD COLUMN module_type TEXT NOT NULL DEFAULT 'PRIMARY_LLM'", []);
    let _ = conn.execute("ALTER TABLE model_registry ADD COLUMN visual_projector_path TEXT DEFAULT NULL", []);
    let _ = conn.execute("ALTER TABLE model_registry ADD COLUMN engine_type TEXT NOT NULL DEFAULT 'llama_cpp'", []);
    let _ = conn.execute("ALTER TABLE model_registry ADD COLUMN topology_json TEXT NOT NULL DEFAULT '{}'", []);
    let _ = conn.execute("ALTER TABLE model_registry ADD COLUMN ttft_ms REAL DEFAULT 0.0", []);
    let _ = conn.execute("ALTER TABLE model_registry ADD COLUMN tpot_ms REAL DEFAULT 0.0", []);
    let _ = conn.execute("ALTER TABLE model_registry ADD COLUMN vram_peak_mb REAL DEFAULT 0.0", []);
    let _ = conn.execute("ALTER TABLE model_registry ADD COLUMN e3_score REAL DEFAULT 0.0", []);
    // MARCO III — Disjuntor de saúde contra crash FFI do Vanguard Worker
    // (ADR-010). Colunas preenchidas por `disable_model_in_sqlite` quando o
    // subprocesso C++ do llama-cpp quebra (exit !=0, broken pipe, std::terminate).
    let _ = conn.execute("ALTER TABLE model_registry ADD COLUMN deactivated_at INTEGER DEFAULT 0", []);
    let _ = conn.execute("ALTER TABLE model_registry ADD COLUMN deactivation_reason TEXT DEFAULT NULL", []);

    // Criação da tabela de telemetria da arena se não existir
    conn.execute(
        "CREATE TABLE IF NOT EXISTS arena_telemetry (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            file_path TEXT NOT NULL,
            prompt_id TEXT NOT NULL,
            ttft_ms REAL NOT NULL DEFAULT 0.0,
            tpot_ms REAL NOT NULL DEFAULT 0.0,
            vram_peak_mb REAL NOT NULL DEFAULT 0.0,
            json_success INTEGER NOT NULL DEFAULT 0,
            e3_score REAL NOT NULL DEFAULT 0.0,
            created_at TEXT NOT NULL DEFAULT (DATETIME('now'))
        );",
        [],
    )
    .map_err(|e| format!("Falha ao criar tabela arena_telemetry: {e}"))?;

    Ok(conn)
}

/// Helper para categorizar a função do arquivo GGUF
pub fn infer_module_type(filename: &str, family: &str) -> &'static str {
    let lower_fn = filename.to_lowercase();
    let lower_fam = family.trim().to_lowercase();
    if lower_fn.contains("mmproj") || lower_fam == "clip" {
        "VISION_PROJECTOR"
    } else if lower_fn.contains("mtp") {
        "MTP_ADAPTER"
    } else if lower_fn.contains("bitnet") || lower_fn.contains("i2_s") || lower_fn.contains("i1_s") {
        "SPECIALIZED_QUANT"
    } else {
        "PRIMARY_LLM"
    }
}

pub fn build_topology_features_from_meta(meta: &ModelMetadata) -> TopologyFeatures {
    let lower_family = meta.family.to_lowercase();
    let attention_type = if lower_family.contains("moe")
        || (meta.architecture.head_count_kv > 0 && meta.architecture.head_count > meta.architecture.head_count_kv * 4)
    {
        AttentionType::MixtureOfExperts
    } else if meta.architecture.head_count_kv > 0 && meta.architecture.head_count != meta.architecture.head_count_kv {
        AttentionType::GroupedQuery
    } else if lower_family.contains("mamba") || lower_family.contains("rwkv") {
        AttentionType::StateSpaceModel
    } else {
        AttentionType::MultiHead
    };

    let rope_scaling = if meta.architecture.rope_scaling_attn_factor.is_some() {
        RopeScalingType::Linear
    } else {
        RopeScalingType::None
    };

    TopologyFeatures {
        family_raw: meta.family.clone(),
        file_format: if meta.file_path.to_lowercase().ends_with(".gguf") {
            FileFormat::Gguf
        } else if meta.file_path.to_lowercase().ends_with(".safetensors") {
            FileFormat::Safetensors
        } else {
            FileFormat::Unknown(meta.file_path.clone())
        },
        attention_type,
        rope_scaling,
        block_count: meta.architecture.block_count,
        head_count: meta.architecture.head_count,
        head_count_kv: meta.architecture.head_count_kv,
        embedding_length: meta.architecture.embedding_length,
        context_length: meta.context_length,
        chat_template: if !meta.architecture.chat_template.is_empty() {
            Some(meta.architecture.chat_template.clone())
        } else {
            None
        },
        ..Default::default()
    }
}

/// Valida se a arquitetura lida do metadado GGUF é suportada pelo motor bare-metal do SOULS via EngineCascade.
pub fn is_architecture_supported(arch: &str) -> bool {
    let lower = arch.trim().to_lowercase();

    // Gate estrutural: engines agnósticos do V4 (ex: PulpLele/Burn/Ort) não podem
    // reclassificar arquiteturas state-space como "suportadas" para o chassi llama/headroom.
    if matches!(lower.as_str(), "rwkv" | "zamba2" | "mamba" | "mamba-ssm") {
        return false;
    }

    let cascade = EngineCascade::new();
    let tf = TopologyFeatures {
        family_raw: arch.to_string(),
        file_format: FileFormat::Gguf,
        ..Default::default()
    };

    let dummy_path = Path::new("Cargo.toml");
    let (engine_id, level) = cascade.probe_best_engine(dummy_path, &tf);
    engine_id != "unsupported" && !matches!(level, EngineSupportLevel::Unsupported(_))
}

/// Sincroniza os modelos locais para a tabela `model_registry`.
/// Regra de Ouro (Garbage Collection): Modelos que sumiram do disco sofrem SOFT DELETION (`is_active = 0`). DELETE físico é proibido!
pub fn sync_local_models_to_registry(conn: &Connection, models_dir: &Path) -> Result<usize, String> {
    let mut scanned_paths = Vec::new();
    let cascade = EngineCascade::new();

    for entry in safe_walk_models(models_dir) {
        let path = entry.path();
        if path.is_file() {
            if let Some(ext) = path.extension() {
                if ext.to_string_lossy().to_lowercase() == "gguf" {
                    let filename = path
                        .file_name()
                        .map(|n| n.to_string_lossy().to_lowercase())
                        .unwrap_or_default();

                    if let Some(m) = parse_gguf_metadata_zero_copy(path) {
                        let actual_mod_type = infer_module_type(&filename, &m.family);

                        if actual_mod_type == "VISION_PROJECTOR" || m.family.trim().to_lowercase() == "clip" {
                            let proj_path_str = path.to_string_lossy().to_string();
                            let _ = conn.execute(
                                "UPDATE model_registry SET is_active = 0, module_type = 'VISION_PROJECTOR' WHERE file_path = ?1",
                                params![proj_path_str],
                            );
                            if let Some(parent_dir) = path.parent() {
                                let parent_str = parent_dir.to_string_lossy().to_string();
                                let _ = conn.execute(
                                    "UPDATE model_registry SET visual_projector_path = ?1 WHERE file_path LIKE ?2 || '%' AND module_type = 'PRIMARY_LLM'",
                                    params![proj_path_str, parent_str],
                                );
                            }
                            continue;
                        }

                        scanned_paths.push(m.file_path.clone());
                        let caps_json = serde_json::to_string(&m.capabilities).unwrap_or_else(|_| "[]".to_string());
                        let tf = build_topology_features_from_meta(&m);
                        let (engine_id, support_level) = cascade.probe_best_engine(path, &tf);
                        let is_active_val = if engine_id != "unsupported" && !matches!(support_level, EngineSupportLevel::Unsupported(_)) { 1 } else { 0 };
                        let topology_json = serde_json::to_string(&tf).unwrap_or_else(|_| "{}".to_string());

                        let res = conn.execute(
                            "INSERT INTO model_registry (file_path, model_name, family, parameters, context_length, quantization, capabilities, file_size_bytes, is_active, module_type, engine_type, topology_json, last_seen)
                             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, DATETIME('now'))
                             ON CONFLICT(file_path) DO UPDATE SET
                                model_name=excluded.model_name,
                                family=excluded.family,
                                parameters=excluded.parameters,
                                context_length=excluded.context_length,
                                quantization=excluded.quantization,
                                capabilities=excluded.capabilities,
                                file_size_bytes=excluded.file_size_bytes,
                                is_active=excluded.is_active,
                                module_type=excluded.module_type,
                                engine_type=excluded.engine_type,
                                topology_json=excluded.topology_json,
                                last_seen=DATETIME('now');",
                            params![
                                m.file_path,
                                m.model_name,
                                m.family,
                                m.parameters,
                                m.context_length as i64,
                                m.quantization,
                                caps_json,
                                m.file_size_bytes as i64,
                                is_active_val,
                                actual_mod_type,
                                engine_id,
                                topology_json,
                            ],
                        );

                        if let Err(e) = res {
                            tracing::error!("Falha ao registrar modelo no SQLite: {e}");
                        }
                    }
                }
            }
        }
    }


    // SOFT DELETION: Marca modelos ausentes do disco como is_active = 0 sem DELETE físico
    let mut stmt = conn
        .prepare("SELECT file_path FROM model_registry WHERE is_active = 1")
        .map_err(|e| format!("Falha ao preparar consulta SQL: {e}"))?;

    let rows = stmt
        .query_map([], |row| row.get::<_, String>(0))
        .map_err(|e| format!("Falha ao iterar sobre registros ativos: {e}"))?;

    let mut missing_paths = Vec::new();
    for row in rows.flatten() {
        if !Path::new(&row).exists() {
            missing_paths.push(row);
        }
    }

    for missing in &missing_paths {
        let _ = conn.execute(
            "UPDATE model_registry SET is_active = 0, last_seen = DATETIME('now') WHERE file_path = ?1",
            params![missing],
        );
        tracing::warn!("Modelo ausente do disco marcado como inativo (is_active = 0): {}", missing);
    }

    Ok(scanned_paths.len())
}

#[allow(clippy::too_many_arguments)]
pub fn record_arena_telemetry(
    conn: &Connection,
    file_path: &str,
    prompt_id: &str,
    ttft_ms: f64,
    tpot_ms: f64,
    vram_peak_mb: f64,
    json_success: bool,
    e3_score: f64,
) -> Result<(), String> {
    let success_val = if json_success { 1 } else { 0 };
    conn.execute(
        "INSERT INTO arena_telemetry (file_path, prompt_id, ttft_ms, tpot_ms, vram_peak_mb, json_success, e3_score)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        params![file_path, prompt_id, ttft_ms, tpot_ms, vram_peak_mb, success_val, e3_score],
    )
    .map_err(|e| format!("Falha ao gravar arena_telemetry: {e}"))?;
    Ok(())
}

/// Atualiza o resultado da avaliação Tier 1 diretamente no banco SQLite SSOT.
#[allow(clippy::too_many_arguments)]
pub fn update_tier1_result(
    conn: &Connection,
    file_path: &str,
    success_rate: f64,
    avg_latency_ms: f64,
    passed: bool,
    ttft_ms: f64,
    tpot_ms: f64,
    vram_peak_mb: f64,
    e3_score: f64,
) -> Result<(), String> {
    let tier1_passed_val = if passed { 1 } else { 0 };
    conn.execute(
        "UPDATE model_registry 
         SET tier1_passed = ?1, 
             success_rate_ema = ?2, 
             ema_latency_ms = ?3, 
             ttft_ms = ?4, 
             tpot_ms = ?5, 
             vram_peak_mb = ?6, 
             e3_score = ?7, 
             last_seen = DATETIME('now')
         WHERE file_path = ?8",
        params![
            tier1_passed_val,
            success_rate,
            avg_latency_ms,
            ttft_ms,
            tpot_ms,
            vram_peak_mb,
            e3_score,
            file_path
        ],
    )
    .map_err(|e| format!("Falha ao atualizar model_registry: {e}"))?;
    Ok(())
}

/// Verifica se o modelo já foi testado no SQLite SSOT para garantir idempotência.
pub fn check_already_evaluated(model_id: &str, conn: &Connection) -> bool {
    let mut stmt = match conn.prepare(
        "SELECT tier1_passed, success_rate_ema, ema_latency_ms 
         FROM model_registry 
         WHERE file_path = ?1 OR model_name = ?1",
    ) {
        Ok(stmt) => stmt,
        Err(_) => return false,
    };

    let mut rows = match stmt.query(params![model_id]) {
        Ok(rows) => rows,
        Err(_) => return false,
    };

    if let Ok(Some(row)) = rows.next() {
        let tier1_passed: i32 = row.get(0).unwrap_or(0);
        let success_rate: f64 = row.get(1).unwrap_or(0.0);
        let latency: f64 = row.get(2).unwrap_or(0.0);
        tier1_passed > 0 || success_rate > 0.0 || latency > 0.0
    } else {
        false
    }
}


/// Consulta os modelos locais que foram APROVADOS no Tier 1 e estão ATIVOS em disco.
pub fn fetch_approved_tier1_models(conn: &Connection) -> Result<Vec<PathBuf>, String> {
    let mut stmt = conn
        .prepare("SELECT file_path FROM model_registry WHERE tier1_passed = 1 AND is_active = 1")
        .map_err(|e| format!("Falha ao preparar consulta de modelos aprovados: {e}"))?;

    let rows = stmt
        .query_map([], |row| row.get::<_, String>(0))
        .map_err(|e| format!("Falha ao mapear modelos aprovados: {e}"))?;

    let mut paths = Vec::new();
    for r in rows.flatten() {
        paths.push(PathBuf::from(r));
    }

    Ok(paths)
}

/// Coleta modelos locais RECURSIVAMENTE via WalkDir ignorando projetores multimodais (mmproj).
pub fn collect_local_models(models_dir: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();
    for entry in safe_walk_models(models_dir) {
        let path = entry.path();
        if path.is_file() {
            if let Some(ext) = path.extension() {
                if ext.to_string_lossy().to_lowercase() == "gguf" {
                    let filename = path
                        .file_name()
                        .map(|n| n.to_string_lossy().to_lowercase())
                        .unwrap_or_default();
                    if filename.contains("mmproj") {
                        continue;
                    }
                    if let Some(meta) = parse_gguf_metadata_zero_copy(path) {
                        if meta.family.trim().to_lowercase() == "clip" {
                            continue;
                        }
                        if infer_module_type(&filename, &meta.family) == "PRIMARY_LLM" {
                            files.push(path.to_path_buf());
                        }
                    } else if infer_module_type(&filename, "") == "PRIMARY_LLM" {
                        files.push(path.to_path_buf());
                    }
                }
            }
        }
    }
    files
}

/// Resolve dinamicamente o caminho absoluto para o modelo GGUF do Avaliador Epistêmico
/// (Gemma 4 E2B / Phi-4-mini) a partir da tabela `model_registry` no SQLite
/// (`souls_state.db` / `souls_heuristic_vault.db`) ou por varredura local.
pub fn resolve_epistemic_model_path() -> Option<PathBuf> {
    let db_path = resolve_db_path();
    if let Ok(conn) = Connection::open_with_flags(&db_path, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY) {
        let stmt = conn.prepare(
            "SELECT file_path FROM model_registry 
             WHERE (family LIKE '%gemma%' OR family LIKE '%phi%' OR file_path LIKE '%gemma%' OR file_path LIKE '%phi%')
               AND is_active = 1
             ORDER BY tier1_passed DESC, file_path DESC LIMIT 1"
        ).ok();
        if let Some(mut s) = stmt {
            if let Ok(mut rows) = s.query([]) {
                if let Ok(Some(row)) = rows.next() {
                    if let Ok(p_str) = row.get::<_, String>(0) {
                        let path = PathBuf::from(p_str);
                        if path.exists() {
                            return Some(path);
                        }
                    }
                }
            }
        }
    }
    // Fallback: varredura em diretórios conhecidos de modelos
    let cwd = env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    let root = if cwd.ends_with("src-tauri") {
        cwd.parent().unwrap_or(&cwd).to_path_buf()
    } else {
        cwd
    };
    let models_dirs = [
        root.join(".souls_data").join("models"),
        root.join("models"),
    ];
    for dir in &models_dirs {
        if dir.exists() {
            let files = collect_local_models(dir);
            for f in files {
                let name = f.file_name().map(|n| n.to_string_lossy().to_lowercase()).unwrap_or_default();
                if name.contains("gemma") || name.contains("phi") || name.contains("e2b") {
                    return Some(f);
                }
            }
        }
    }
    None
}

/// Carrega modelos aprovados a partir do relatório fallback de texto Tier 1.
pub fn load_approved_tier1_models(report_path: &Path) -> Vec<PathBuf> {
    let mut approved = Vec::new();
    let file = match File::open(report_path) {
        Ok(f) => f,
        Err(_) => return approved,
    };

    use std::io::BufRead;
    let reader = std::io::BufReader::new(file);
    let mut current_path: Option<PathBuf> = None;

    for line_res in reader.lines() {
        let line = match line_res {
            Ok(l) => l,
            Err(_) => continue,
        };
        let trimmed = line.trim();
        if trimmed.starts_with("Path:") {
            let path_str = trimmed.trim_start_matches("Path:").trim();
            current_path = Some(PathBuf::from(path_str));
        } else if trimmed.starts_with("Status:") && trimmed.contains("[APROVADO PARA TIER 2]") {
            if let Some(p) = current_path.take() {
                approved.push(p);
            }
        }
    }

    approved
}

pub fn format_model_canonical_name(filename_or_path: &str, parent_dir: Option<&str>) -> String {
    let path = Path::new(filename_or_path);
    let raw_stem = path
        .file_stem()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| filename_or_path.to_string());

    // 1. Extração da Quantização
    let quant_regex = regex::Regex::new(r"(?i)(Q[0-9]_[K01]_[MSL01]|Q[0-9]_[01K]|IQ[0-9]_[MSL]|BF16|F16|F32|i2_s)").unwrap();
    let quant_str = if let Some(mat) = quant_regex.find(&raw_stem) {
        mat.as_str().to_uppercase()
    } else if let Some(mat) = quant_regex.find(filename_or_path) {
        mat.as_str().to_uppercase()
    } else {
        "GGUF".to_string()
    };

    let stem_no_quant = if quant_str != "GGUF" {
        let re_quant = regex::Regex::new(&format!(r"(?i)[-_\s]*\b{}\b[-_\s]*", regex::escape(&quant_str))).unwrap();
        re_quant.replace_all(&raw_stem, "-").to_string()
    } else {
        raw_stem.clone()
    };

    // 2. Extração do Criador / Publisher
    let creators = [
        ("nvidia", "NVIDIA"),
        ("unsloth", "Unsloth"),
        ("lmstudio-community", "LMStudio"),
        ("lmstudio", "LMStudio"),
        ("ankitai", "AnkitAI"),
        ("hauhaucs", "HauhauCS"),
        ("jackrong", "Jackrong"),
        ("mradermacher", "Mradermacher"),
        ("nimbus-labs", "Nimbus Labs"),
        ("nimbus", "Nimbus Labs"),
        ("owao", "Owao"),
        ("tencent", "Tencent"),
        ("zero-point-ai", "Zero-Point AI"),
        ("zero-point", "Zero-Point AI"),
        ("flyingfishinwater", "Flyingfishinwater"),
        ("jica98", "Jica98"),
        ("microsoft", "Microsoft"),
        ("meta", "Meta"),
        ("google", "Google"),
        ("mistralai", "Mistral"),
        ("mistral", "Mistral"),
        ("qwen", "Qwen"),
        ("deepseek-ai", "DeepSeek"),
        ("deepseek", "DeepSeek"),
    ];

    let stem_lower = stem_no_quant.to_lowercase();
    let parent_lower = parent_dir.unwrap_or_default().to_lowercase();

    let mut found_creator: Option<&str> = None;
    let mut creator_token_in_stem: Option<&str> = None;

    for &(key, display_name) in &creators {
        if stem_lower.contains(key) {
            found_creator = Some(display_name);
            creator_token_in_stem = Some(key);
            break;
        }
    }

    if found_creator.is_none() {
        for &(key, display_name) in &creators {
            if parent_lower.contains(key) {
                found_creator = Some(display_name);
                break;
            }
        }
    }

    let creator = found_creator.unwrap_or("Local");

    // 3. Limpeza e Formatação do Nome Base do Modelo
    let mut cleaned_stem = stem_no_quant;
    if let Some(token) = creator_token_in_stem {
        let re_token = regex::Regex::new(&format!(r"(?i)[-_\s]*\b{}\b[-_\s]*", regex::escape(token))).unwrap();
        cleaned_stem = re_token.replace_all(&cleaned_stem, "-").to_string();
    }

    cleaned_stem = cleaned_stem.replace(['_', '.'], " ");

    let parts: Vec<&str> = cleaned_stem
        .split('-')
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .collect();

    let mut formatted_parts = Vec::new();
    for part in parts {
        let words: Vec<&str> = part.split_whitespace().collect();
        let mut formatted_words = Vec::new();
        for word in words {
            let lower = word.to_lowercase();
            if lower == "gguf" || lower == "bin" {
                continue;
            }
            if lower == "nvidia" {
                formatted_words.push("NVIDIA".to_string());
            } else if lower == "phi" {
                formatted_words.push("Phi".to_string());
            } else if word.chars().all(|c| c.is_ascii_uppercase()) && word.len() <= 4 {
                formatted_words.push(word.to_string());
            } else if word.chars().any(|c| c.is_numeric()) && word.chars().any(|c| c.is_alphabetic()) {
                formatted_words.push(capitalize_alphanumeric(word));
            } else {
                formatted_words.push(to_title_case(word));
            }
        }
        if !formatted_words.is_empty() {
            formatted_parts.push(formatted_words.join(" "));
        }
    }

    let mut base_model = formatted_parts.join(" ");
    base_model = base_model
        .replace("Phi 4", "Phi-4")
        .replace("Nemotron 3", "Nemotron-3")
        .replace("Hy MT2", "Hy-MT2")
        .replace("Hy Mt2", "Hy-MT2")
        .replace("HY MT2", "Hy-MT2")
        .trim()
        .to_string();

    if base_model.is_empty() {
        base_model = "Base Model".to_string();
    }

    format!("{} - {} ({})", creator, base_model, quant_str)
}

fn to_title_case(s: &str) -> String {
    let mut c = s.chars();
    match c.next() {
        None => String::new(),
        Some(f) => f.to_uppercase().collect::<String>() + c.as_str().to_lowercase().as_str(),
    }
}

fn capitalize_alphanumeric(s: &str) -> String {
    let mut result = String::new();
    for chunk in s.split_inclusive(|c: char| !c.is_alphanumeric()) {
        let mut chars = chunk.chars();
        if let Some(first) = chars.next() {
            result.push(first.to_ascii_uppercase());
            for rest in chars {
                result.push(rest);
            }
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn test_safe_walk_models_respects_max_depth_4() {
        // Cria árvore: root/level1/level2/level3/level4/level5 (deep demais)
        // e verifica que level5 (depth=5) NÃO é visitado.
        let tmp = std::env::temp_dir().join(format!("souls_walk_test_{}", std::process::id()));
        let _ = fs::remove_dir_all(&tmp);
        let deep = tmp.join("l1/l2/l3/l4/l5/deep.gguf");
        fs::create_dir_all(deep.parent().unwrap()).unwrap();
        fs::write(&deep, b"fake_gguf").unwrap();
        // Cria também arquivo raso no nível 1 (deve ser encontrado).
        let shallow = tmp.join("shallow.gguf");
        fs::write(&shallow, b"fake_gguf_shallow").unwrap();

        let mut found = Vec::new();
        for entry in safe_walk_models(&tmp) {
            let p = entry.path();
            if p.extension().and_then(|e| e.to_str()) == Some("gguf") {
                found.push(p.file_name().unwrap().to_owned());
            }
        }
        let _ = fs::remove_dir_all(&tmp);

        // Arquivo raso deve ser encontrado.
        assert!(
            found.iter().any(|n| n == "shallow.gguf"),
            "shallow.gguf (depth=1) deve ser encontrado, encontrados: {found:?}"
        );
        // Arquivo profundo NÃO deve ser encontrado (depth=6).
        assert!(
            !found.iter().any(|n| n == "deep.gguf"),
            "deep.gguf (depth=6) NÃO deve ser encontrado com max_depth=4, encontrados: {found:?}"
        );
    }

    #[test]
    fn test_safe_walk_models_handles_nonexistent_dir_without_panic() {
        // Diretório inexistente → iterator vazio, sem panic.
        let bogus = std::env::temp_dir().join("souls_walk_does_not_exist_xyz_12345");
        let count = safe_walk_models(&bogus).count();
        assert_eq!(count, 0, "diretório inexistente deve dar iterator vazio");
    }

    #[test]
    fn test_safe_walk_models_skips_broken_symlink() {
        // Cria symlink quebrado (aponta para path inexistente) e verifica
        // que o walk não panica nem trava.
        // Em Windows, criar symlink requer privilégio de admin (ERROR 1314);
        // se a criação falhar por privilégio, pulamos o teste (não é
        // defeito de produção: produção tipicamente tem Developer Mode ON
        // ou roda como admin, mas ambiente de teste pode não ter).
        let tmp = std::env::temp_dir().join(format!("souls_walk_broken_{}", std::process::id()));
        let _ = fs::remove_dir_all(&tmp);
        fs::create_dir_all(&tmp).unwrap();
        let broken = tmp.join("broken_link.gguf");
        let symlink_ok = {
            #[cfg(unix)]
            {
                std::os::unix::fs::symlink("/nonexistent/target", &broken).is_ok()
            }
            #[cfg(windows)]
            {
                std::os::windows::fs::symlink_file(r"C:\nonexistent\target.gguf", &broken).is_ok()
            }
        };
        if !symlink_ok {
            let _ = fs::remove_dir_all(&tmp);
            eprintln!("[SKIP] symlink creation requires admin/developer mode; skipping broken-symlink test");
            return;
        }
        let count = safe_walk_models(&tmp).count();
        let _ = fs::remove_dir_all(&tmp);
        // O walk não pode panicar. O symlink quebrado pode ou não aparecer
        // dependendo do OS; o que importa é que o iterator termina.
        assert!(count <= 1, "iterator deve terminar sem panic, count = {count}");
    }
    #[test]
    fn test_format_model_canonical_name() {
        let input1 = "Phi-4-mini-instruct-unsloth-Q4_K_M.gguf";
        let formatted1 = format_model_canonical_name(input1, None);
        assert_eq!(formatted1, "Unsloth - Phi-4 Mini Instruct (Q4_K_M)");

        let input2 = "NVIDIA-Nemotron-3-Nano-4B-Q4_K_M.gguf";
        let formatted2 = format_model_canonical_name(input2, Some("lmstudio-community"));
        assert_eq!(formatted2, "NVIDIA - Nemotron-3 Nano 4B (Q4_K_M)");

        let input3 = "Hy-MT2-7B-Q4_K_M.gguf";
        let formatted3 = format_model_canonical_name(input3, Some("tencent"));
        assert_eq!(formatted3, "Tencent - Hy-MT2 7B (Q4_K_M)");
    }

    fn dummy_qwen_metadata() -> ModelMetadata {
        ModelMetadata {
            file_path: "test_qwen.gguf".to_string(),
            model_name: "Qwen3.5-4B-Q4_K_M".to_string(),
            family: "Qwen3.5".to_string(),
            parameters: "4B".to_string(),
            context_length: 262144,
            quantization: "Q4_K_M".to_string(),
            capabilities: vec!["TOOL_CALLING".to_string()],
            file_size_bytes: 2_700_000_000, // ~2.51 GiB
            is_active: true,
            tier1_passed: true,
            success_rate_ema: 1.0,
            ema_latency_ms: 100.0,
            architecture: ModelArchitectureParams {
                block_count: 32,
                embedding_length: 2560,
                head_count: 16,
                head_count_kv: 4, // GQA 4:1
                feed_forward_length: 9216,
                rope_scaling_attn_factor: None,
                chat_template: String::new(),
            },
        }
    }

    #[test]
    fn test_vram_thermodynamics_gqa_asymmetric_kv_and_batch() {
        let meta = dummy_qwen_metadata();
        let ctx = 30_000u64;
        let lora_bytes = 64 * 1024 * 1024; // 64 MB LoRA
        let vram_6gb = 6 * 1024 * 1024 * 1024;

        // Symmetric FP16 K (2.0) + FP16 V (2.0) = 4.0 bytes per element per head_dim token
        let est_f16 = estimate_vram_thermodynamics(&meta, ctx, 1.0, 2.0, 2.0, 1, true, 0, vram_6gb);

        // Asymmetric K in FP16 (2.0 bytes) and V in Q4_K (0.5 bytes) = 2.5 bytes total per element
        let est_asymmetric = estimate_vram_thermodynamics(&meta, ctx, 1.0, 2.0, 0.5, 1, true, 0, vram_6gb);

        // Symmetric Q4_K (0.5 + 0.5 = 1.0 byte total per element) with LoRA overhead
        let est_q4_lora = estimate_vram_thermodynamics(&meta, ctx, 1.0, 0.5, 0.5, 1, true, lora_bytes, vram_6gb);

        // Batch size 2 test with asymmetric K/V (2.0 + 0.5 = 2.5 bytes)
        let est_batch2 = estimate_vram_thermodynamics(&meta, ctx, 1.0, 2.0, 0.5, 2, true, 0, vram_6gb);

        // head_dim = 2560 / 16 = 160
        // FP16 KV (b=1): 32 layers * 4 heads_kv * 160 head_dim * 30000 ctx * (2.0 + 2.0) = 2,457,600,000 bytes
        assert_eq!(est_f16.kv_cache_vram_bytes, 2_457_600_000);

        // Asymmetric KV (K=2.0, V=0.5, b=1): 32 * 4 * 160 * 30000 * 2.5 = 1,536,000,000 bytes (~1.43 GB)
        assert_eq!(est_asymmetric.kv_cache_vram_bytes, 1_536_000_000);

        // Symmetric Q4_K KV (K=0.5, V=0.5, b=1): 32 * 4 * 160 * 30000 * 1.0 = 614,400,000 bytes (~0.57 GB)
        assert_eq!(est_q4_lora.kv_cache_vram_bytes, 614_400_000);
        assert_eq!(est_q4_lora.lora_overhead_vram_bytes, lora_bytes);

        // Batch 2 Asymmetric (b=2): 1,536,000,000 * 2 = 3,072,000,000 bytes (~2.86 GB)
        assert_eq!(est_batch2.kv_cache_vram_bytes, 3_072_000_000);

        // Check if Q4_K with 64MB LoRA fits in dynamic 6GB VRAM limit
        assert!(est_q4_lora.fits_in_vram);
    }

    #[test]
    fn test_vram_thermodynamics_preventive_oom_without_flash_attention() {
        let meta = dummy_qwen_metadata();
        let ctx = 30_000u64;
        let lora_bytes = 64 * 1024 * 1024; // 64 MB LoRA
        let vram_6gb = 6 * 1024 * 1024 * 1024;

        // Flash Attention DISABLED (FA = false): Scratch escala O(N^2)
        // 30,000^2 * 16 heads * 2 bytes = 28,800,000,000 bytes (~28.8 GB scratch!)
        let est_no_fa = estimate_vram_thermodynamics(&meta, ctx, 1.0, 0.5, 0.5, 1, false, lora_bytes, vram_6gb);

        // Flash Attention ENABLED (FA = true): Scratch escala O(N)
        // 30,000 * 16 * 64 = 30,720,000 bytes (~30.7 MB scratch)
        let est_with_fa = estimate_vram_thermodynamics(&meta, ctx, 1.0, 0.5, 0.5, 1, true, lora_bytes, vram_6gb);

        // Sem Flash Attention em contexto longo (30k tokens), a alocação estoura o limite de 6GB VRAM preventivamente
        assert!(!est_no_fa.fits_in_vram);
        assert!(est_no_fa.total_estimated_vram_bytes > 30_000_000_000);

        // Com Flash Attention ativado, o modelo cabe confortavelmente na VRAM dinamicamente fornecida
        assert!(est_with_fa.fits_in_vram);
        assert!(est_with_fa.total_estimated_vram_bytes < 4_000_000_000);
    }

    #[test]
    fn test_vram_thermodynamics_system_topology_integration() {
        let meta = dummy_qwen_metadata();
        let ctx = 30_000u64;
        let lora_bytes = 64 * 1024 * 1024; // 64 MB LoRA

        let mock_topology = crate::core::hardware_profiler::SystemTopology {
            gpu_name: "NVIDIA GeForce RTX 2060".to_string(),
            vram_total_bytes: 6 * 1024 * 1024 * 1024,
            ram_total_bytes: 16 * 1024 * 1024 * 1024,
            is_dedicated_gpu: true,
            primary_simd_extension: crate::core::hardware_profiler::CpuInstructionSet::Avx2,
            is_nvme_ssd: true,
            pcie_bandwidth_estimated_gbps: Some(15.75),
        };

        let est = estimate_vram_for_topology(
            &meta,
            ctx,
            1.0,
            0.5,
            0.5,
            1,
            true,
            lora_bytes,
            &mock_topology,
        );

        assert!(est.fits_in_vram);
        assert_eq!(
            est.total_estimated_vram_bytes,
            2_700_000_000 + 614_400_000 + (256 * 1024 * 1024 + 30_000 * 16 * 64) + lora_bytes
        );
    }

    #[test]
    fn test_collect_local_models_respects_max_depth_5() {
        let temp_dir = tempfile::tempdir().unwrap();
        let root = temp_dir.path();

        // Marco 4.10.1 — ETAPA 2: max_depth=4 (não 5).
        // Profundidade 4: root / d1 / d2 / d3 / model_l4.gguf
        let l4_dir = root.join("d1").join("d2").join("d3");
        std::fs::create_dir_all(&l4_dir).unwrap();
        std::fs::write(l4_dir.join("model_l4.gguf"), b"GGUF").unwrap();

        // Profundidade 5 (acima de max_depth 4): root / d1 / d2 / d3 / d4 / model_l5.gguf
        let l5_dir = l4_dir.join("d4");
        std::fs::create_dir_all(&l5_dir).unwrap();
        std::fs::write(l5_dir.join("model_l5.gguf"), b"GGUF").unwrap();

        let models = collect_local_models(root);
        assert!(
            models.iter().any(|p| p.file_name().unwrap() == "model_l4.gguf"),
            "model_l4.gguf (depth=4) deve ser encontrado, modelos: {:?}",
            models
        );
        assert!(
            !models.iter().any(|p| p.file_name().unwrap() == "model_l5.gguf"),
            "model_l5.gguf (depth=5) NAO deve ser encontrado com max_depth=4"
        );
    }

    #[test]
    fn test_gguf_cached_metadata_reads() {
        let temp_dir = tempfile::tempdir().unwrap();
        let fake_gguf = temp_dir.path().join("stress_test_model.gguf");

        // GGUF header mínimo válido (24 bytes): "GGUF" + version(3) + tensor_count(0) + kv_count(0)
        let mut header = vec![b'G', b'G', b'U', b'F'];
        header.extend_from_slice(&3u32.to_le_bytes());
        header.extend_from_slice(&0u64.to_le_bytes());
        header.extend_from_slice(&0u64.to_le_bytes());
        std::fs::write(&fake_gguf, &header).unwrap();

        // 1ª leitura: mmap/disk -> hidrata DashMap L1
        let first = parse_gguf_metadata_zero_copy(&fake_gguf);
        assert!(first.is_some(), "1ª leitura de metadados GGUF deve suceder via mmap");
        let first_meta = first.unwrap();

        let initial_cache_len = GLOBAL_GGUF_METADATA_CACHE.len();
        assert!(initial_cache_len >= 1, "Cache L1 DashMap deve possuir ao menos 1 elemento");

        // 9 leituras subsequentes: devem resolver em O(1) no DashMap L1 em RAM sem novos mmaps
        for i in 2..=10 {
            let subsequent = parse_gguf_metadata_zero_copy(&fake_gguf);
            assert!(subsequent.is_some(), "Leitura {}/10 deve retornar resultado do cache L1", i);
            assert_eq!(
                subsequent.unwrap(),
                first_meta,
                "Leitura {}/10 deve ser idêntica ao metadado inicial",
                i
            );
        }

        // Deleta o arquivo físico em disco para PROVAR que as leituras 11..15 continuam resolvendo da RAM!
        std::fs::remove_file(&fake_gguf).unwrap();

        for i in 11..=15 {
            let ram_hit = parse_gguf_metadata_zero_copy(&fake_gguf);
            assert!(
                ram_hit.is_some(),
                "Leitura {}/15 (pós-remoção física do arquivo) DEVE resolver do L1 RAM DashMap",
                i
            );
            assert_eq!(ram_hit.unwrap(), first_meta);
        }
    }
}





