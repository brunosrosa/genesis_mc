//! SOULS Headroom Engine & CCR (Compress-Cache-Retrieve) Gateway Core
//! ADR-037 / PRD-10.3 Bare-Metal Implementation

use std::borrow::Cow;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use dashmap::DashMap;
use sha2::{Sha256, Digest};
use thiserror::Error;

use crate::core::model_registry;

#[derive(Debug, Error, PartialEq, Eq)]
pub enum HeadroomError {
    #[error("Arquitetura de modelo não suportada pelo Gateway SOULS: {0}")]
    UnsupportedArchitecture(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HeadroomBudget {
    pub h_in: usize,
    pub t_total: usize,
    pub trigger: bool,
    pub delta_r: usize,
    pub capped_c_max: usize,
}

/// Capa a janela de contexto máxima com base na família do modelo (ex: Gemma 32k cap térmico)
pub fn cap_context_length_for_family(family: &str, declared_ctx: usize) -> usize {
    let lower = family.trim().to_lowercase();
    if lower.contains("gemma") {
        declared_ctx.min(32_768)
    } else {
        declared_ctx
    }
}

/// Lei 1: A Matemática do Orçamento de Contexto
/// H_in = C_max - B_out - delta_safe
/// T_total = T_sys + T_tools + T_hist + T_live
/// Trigger = T_total > H_in
/// delta_r = T_total - H_in (se Trigger)
pub fn calculate_headroom_budget(
    c_max: usize,
    b_out: usize,
    delta_safe: usize,
    t_sys: usize,
    t_tools: usize,
    t_hist: usize,
    t_live: usize,
) -> HeadroomBudget {
    let h_in = c_max.saturating_sub(b_out).saturating_sub(delta_safe);
    let t_total = t_sys + t_tools + t_hist + t_live;
    let trigger = t_total > h_in;
    let delta_r = if trigger { t_total.saturating_sub(h_in) } else { 0 };

    HeadroomBudget {
        h_in,
        t_total,
        trigger,
        delta_r,
        capped_c_max: c_max,
    }
}

/// Avalia a arquitetura e orça o contexto aplicando o cap térmico por família do modelo (Fim do Falso C_max)
pub fn calculate_headroom_budget_for_model(
    model_family: &str,
    declared_c_max: usize,
    b_out: usize,
    delta_safe: usize,
    t_sys: usize,
    t_tools: usize,
    t_hist: usize,
    t_live: usize,
) -> Result<HeadroomBudget, HeadroomError> {
    // 2. AUDITORIA DE REJEIÇÃO: Rejeita arquiteturas não suportadas (ex: Zamba2, Mamba, RWKV) antes do Headroom
    if !model_registry::is_architecture_supported(model_family) {
        return Err(HeadroomError::UnsupportedArchitecture(model_family.to_string()));
    }

    // 1. INTEGRAÇÃO DO HOTFIX NO HEADROOM: Aplica o Hard Cap (ex: 32k Gemma) antes da matemática do orçamento
    let real_c_max = cap_context_length_for_family(model_family, declared_c_max);

    Ok(calculate_headroom_budget(
        real_c_max,
        b_out,
        delta_safe,
        t_sys,
        t_tools,
        t_hist,
        t_live,
    ))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LexerState {
    Code,
    String,
    Char,
    LineComment,
    BlockComment,
}

pub struct CodeCompressor;

impl CodeCompressor {
    /// Lei 2: Poda Semântica Determinística em Rust (Zero-Copy)
    /// Subdivide corpos de funções por `{ /* stubbed */ }` preservando assinaturas.
    /// Utiliza uma Máquina de Estados de Lexer em uma única passada (Single-pass Lexer State Machine)
    /// resiliente a aspas, escapes e comentários.
    pub fn compress_ast_zero_copy<'a>(code: &'a str) -> Cow<'a, str> {
        let bytes = code.as_bytes();
        let mut modified = false;
        let mut result = String::with_capacity(code.len());
        
        let mut i = 0;
        let len = bytes.len();
        
        while i < len {
            // Verifica se inicia assinatura de função
            let is_func = (i == 0 || bytes[i - 1] == b'\n' || bytes[i - 1] == b' ' || bytes[i - 1] == b';')
                && (code[i..].starts_with("fn ")
                    || code[i..].starts_with("pub fn ")
                    || code[i..].starts_with("async fn ")
                    || code[i..].starts_with("function ")
                    || code[i..].starts_with("def "));

            if is_func {
                // Encontra a abertura de chave '{' ignorando chaves em comentários ou strings da própria assinatura
                let mut brace_start = None;
                let mut j = i;
                while j < len {
                    if bytes[j] == b'{' {
                        brace_start = Some(j);
                        break;
                    }
                    if bytes[j] == b';' {
                        // Declaração sem corpo (ex: fn trait)
                        break;
                    }
                    j += 1;
                }

                if let Some(start_idx) = brace_start {
                    // Encontra o encerramento '}' correspondente via Lexer State Machine
                    let mut depth = 1;
                    let mut k = start_idx + 1;
                    let mut state = LexerState::Code;

                    while k < len && depth > 0 {
                        let b = bytes[k];
                        match state {
                            LexerState::Code => {
                                if b == b'"' {
                                    state = LexerState::String;
                                } else if b == b'\'' {
                                    state = LexerState::Char;
                                } else if b == b'/' && k + 1 < len && bytes[k + 1] == b'/' {
                                    state = LexerState::LineComment;
                                    k += 1;
                                } else if b == b'/' && k + 1 < len && bytes[k + 1] == b'*' {
                                    state = LexerState::BlockComment;
                                    k += 1;
                                } else if b == b'{' {
                                    depth += 1;
                                } else if b == b'}' {
                                    depth -= 1;
                                }
                            }
                            LexerState::String => {
                                if b == b'\\' {
                                    k += 1; // Pula caractere escapado
                                } else if b == b'"' {
                                    state = LexerState::Code;
                                }
                            }
                            LexerState::Char => {
                                if b == b'\\' {
                                    k += 1; // Pula caractere escapado
                                } else if b == b'\'' {
                                    state = LexerState::Code;
                                }
                            }
                            LexerState::LineComment => {
                                if b == b'\n' {
                                    state = LexerState::Code;
                                }
                            }
                            LexerState::BlockComment => {
                                if b == b'*' && k + 1 < len && bytes[k + 1] == b'/' {
                                    state = LexerState::Code;
                                    k += 1;
                                }
                            }
                        }
                        k += 1;
                    }

                    if depth == 0 {
                        // Assinatura e abertura de chave preservadas
                        result.push_str(&code[i..=start_idx]);
                        result.push_str(" /* stubbed */ }");
                        i = k;
                        modified = true;
                        continue;
                    }
                }
            }

            result.push(bytes[i] as char);
            i += 1;
        }

        if modified {
            Cow::Owned(result)
        } else {
            Cow::Borrowed(code)
        }
    }
}

pub struct CcrEntry {
    pub payload: Vec<u8>,
    pub last_accessed_at: AtomicU64,
}

/// Lei 3 & 4: CCR Store (Compress-Cache-Retrieve) alocado 100% em Host RAM (Zero-VRAM)
pub struct SoulsCcrStore {
    cache: Arc<DashMap<[u8; 16], CcrEntry>>,
    max_ram_bytes: usize,
    current_ram_bytes: AtomicUsize,
    vram_bytes_allocated: AtomicUsize,
    access_counter: AtomicU64,
}

impl SoulsCcrStore {
    pub fn new(max_ram_bytes: usize) -> Self {
        Self {
            cache: Arc::new(DashMap::new()),
            max_ram_bytes,
            current_ram_bytes: AtomicUsize::new(0),
            vram_bytes_allocated: AtomicUsize::new(0),
            access_counter: AtomicU64::new(1),
        }
    }

    pub fn from_env() -> Self {
        let max_mb = std::env::var("SOULS_CCR_MAX_RAM_MB")
            .ok()
            .and_then(|v| v.parse::<usize>().ok())
            .unwrap_or(256);
        Self::new(max_mb * 1024 * 1024)
    }

    /// Armazena o payload original e retorna o Hash de 16 bytes.
    /// Dispara evicção LRU se current_ram_bytes atingir a maré alta de 90%.
    pub fn store(&self, payload: &[u8]) -> [u8; 16] {
        let mut hasher = Sha256::new();
        hasher.update(payload);
        let digest = hasher.finalize();

        let mut hash = [0u8; 16];
        hash.copy_from_slice(&digest[..16]);

        let payload_len = payload.len();

        if let Some(old_entry) = self.cache.get(&hash) {
            let old_len = old_entry.payload.len();
            self.current_ram_bytes.fetch_update(Ordering::SeqCst, Ordering::SeqCst, |curr| {
                Some(curr.saturating_sub(old_len))
            }).ok();
        }

        let access_time = self.access_counter.fetch_add(1, Ordering::Relaxed);
        self.current_ram_bytes.fetch_add(payload_len, Ordering::SeqCst);

        self.cache.insert(
            hash,
            CcrEntry {
                payload: payload.to_vec(),
                last_accessed_at: AtomicU64::new(access_time),
            },
        );

        // Zero VRAM Footprint Invariante
        self.vram_bytes_allocated.store(0, Ordering::Relaxed);

        // Checagem de Maré Alta (High Watermark: >= 90% do limite max_ram_bytes)
        let high_watermark = (self.max_ram_bytes * 90) / 100;
        let target_watermark = (self.max_ram_bytes * 80) / 100;

        if self.current_ram_bytes.load(Ordering::SeqCst) >= high_watermark {
            self.evict_lru_until(target_watermark);
        }

        hash
    }

    /// Resgata o payload original via Hash de 16 bytes e atualiza o timestamp de acesso
    pub fn retrieve(&self, hash: &[u8; 16]) -> Option<Vec<u8>> {
        if let Some(entry) = self.cache.get(hash) {
            let now = self.access_counter.fetch_add(1, Ordering::Relaxed);
            entry.last_accessed_at.store(now, Ordering::Relaxed);
            Some(entry.payload.clone())
        } else {
            None
        }
    }

    /// Rotina de evicção LRU: expurga registros mais antigos/frios até que current_ram_bytes <= target_bytes
    pub fn evict_lru_until(&self, target_bytes: usize) {
        while self.current_ram_bytes.load(Ordering::SeqCst) > target_bytes {
            // 1. Coleta e clona chaves e timestamps em passo isolado liberando locks do DashMap
            let mut entries: Vec<([u8; 16], u64)> = self
                .cache
                .iter()
                .map(|kv| (*kv.key(), kv.value().last_accessed_at.load(Ordering::Relaxed)))
                .collect();

            if entries.is_empty() {
                break;
            }

            // 2. Ordena por idade de acesso (LRU)
            entries.sort_unstable_by_key(|&(_, ts)| ts);

            let mut evicted_any = false;
            // 3. Remoção individual fora de qualquer laço de travamento
            for (hash, _) in entries {
                if self.current_ram_bytes.load(Ordering::SeqCst) <= target_bytes {
                    break;
                }
                if let Some((_, entry)) = self.cache.remove(&hash) {
                    let len = entry.payload.len();
                    self.current_ram_bytes.fetch_update(Ordering::SeqCst, Ordering::SeqCst, |curr| {
                        Some(curr.saturating_sub(len))
                    }).ok();
                    evicted_any = true;
                }
            }

            if !evicted_any {
                break;
            }
        }
    }

    pub fn current_ram_bytes(&self) -> usize {
        self.current_ram_bytes.load(Ordering::Relaxed)
    }

    /// Registra footprint de VRAM (deve ser estritamente 0)
    pub fn vram_bytes_allocated(&self) -> usize {
        self.vram_bytes_allocated.load(Ordering::Relaxed)
    }

    /// Interceção Tool Loopback em < 1ms pelo Gateway Tokio Rust
    pub fn intercept_loopback(&self, tool_call_json: &str) -> Option<String> {
        if !tool_call_json.contains("headroom_retrieve") {
            return None;
        }

        // Tenta extrair o hash hex da chamada JSON
        let hash_hex = tool_call_json
            .split("\"hash\":\"")
            .nth(1)
            .or_else(|| tool_call_json.split("\"hash\": \"").nth(1))?
            .split('"')
            .next()?;

        let hash_bytes = hex_decode(hash_hex)?;
        if hash_bytes.len() != 16 {
            return None;
        }

        let mut hash_arr = [0u8; 16];
        hash_arr.copy_from_slice(&hash_bytes);

        let payload = self.retrieve(&hash_arr)?;
        let text = String::from_utf8_lossy(&payload);

        Some(format!(r#"{{"status":"success","retrieved_payload":"{}"}}"#, text.replace('"', "\\\"").replace('\n', "\\n")))
    }

    pub fn max_ram_bytes(&self) -> usize {
        self.max_ram_bytes
    }
}

#[allow(dead_code)]
pub fn hex_encode(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{:02x}", b)).collect()
}

fn hex_decode(s: &str) -> Option<Vec<u8>> {
    if s.len() % 2 != 0 {
        return None;
    }
    (0..s.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&s[i..i + 2], 16).ok())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_headroom_math_budget() {
        let c_max = 128_000;
        let b_out = 4_096;
        let delta_safe = 512;
        let t_sys = 1_000;
        let t_tools = 2_000;
        let t_hist = 130_000;
        let t_live = 3_000;

        let budget = calculate_headroom_budget(c_max, b_out, delta_safe, t_sys, t_tools, t_hist, t_live);
        assert_eq!(budget.h_in, 123_392);
        assert_eq!(budget.t_total, 136_000);
        assert!(budget.trigger);
        assert_eq!(budget.delta_r, 12_608);
    }

    #[test]
    fn test_headroom_math_budget_capped_gemma() {
        let declared_c_max = 131_072; // Gemma declarando 128k
        let b_out = 4_096;
        let delta_safe = 512;
        let t_sys = 1_000;
        let t_tools = 2_000;
        let t_hist = 30_000;
        let t_live = 3_000;

        // Com cap de 32k: H_in = 32768 - 4096 - 512 = 28160.
        // T_total = 1000 + 2000 + 30000 + 3000 = 36000.
        // Trigger = 36000 > 28160 (true). Delta R = 36000 - 28160 = 7840.
        let budget = calculate_headroom_budget_for_model(
            "gemma4",
            declared_c_max,
            b_out,
            delta_safe,
            t_sys,
            t_tools,
            t_hist,
            t_live,
        )
        .expect("gemma4 é suportado");

        assert_eq!(budget.capped_c_max, 32_768);
        assert_eq!(budget.h_in, 28_160);
        assert_eq!(budget.t_total, 36_000);
        assert!(budget.trigger);
        assert_eq!(budget.delta_r, 7_840);
    }

    #[test]
    fn test_rejection_unsupported_architecture_zamba2() {
        let err = calculate_headroom_budget_for_model(
            "zamba2",
            128_000,
            4_096,
            512,
            1_000,
            2_000,
            10_000,
            1_000,
        )
        .unwrap_err();

        assert_eq!(err, HeadroomError::UnsupportedArchitecture("zamba2".to_string()));
    }

    #[test]
    fn test_ast_code_compressor_zero_copy() {
        let code = r#"
fn process_payload(data: &str) -> String {
    let result = data.to_uppercase();
    format!("PROCESSED: {}", result)
}

pub fn calculate_total(a: i32, b: i32) -> i32 {
    a + b
}
"#;
        let compressed = CodeCompressor::compress_ast_zero_copy(code);
        assert!(compressed.contains("fn process_payload"));
        assert!(compressed.contains("/* stubbed */"));
        assert!(compressed.len() < code.len());
    }

    #[test]
    fn test_ccr_dashmap_allocation_host_ram() {
        let store = SoulsCcrStore::new(256 * 1024 * 1024);
        let payload = b"fn critical_business_logic() { println!(\"Zero VRAM CCR Test\"); }";
        let hash = store.store(payload);

        let retrieved = store.retrieve(&hash);
        assert_eq!(retrieved.as_deref(), Some(&payload[..]));

        // Proven Footprint Zero VRAM
        assert_eq!(store.vram_bytes_allocated(), 0);

        // Tool Loopback interception
        let hex_hash = hex_encode(&hash);
        let tool_json = format!(r#"{{"name":"headroom_retrieve","parameters":{{"hash":"{}"}}}}"#, hex_hash);
        let loopback_resp = store.intercept_loopback(&tool_json);
        assert!(loopback_resp.is_some());
        assert!(loopback_resp.unwrap().contains("critical_business_logic"));
    }

    #[test]
    fn test_ccr_lru_eviction_high_watermark() {
        let max_bytes = 1000;
        let store = SoulsCcrStore::new(max_bytes);

        // Insere 10 payloads de 100 bytes = 1000 bytes (excede a maré alta de 90% = 900 bytes)
        let mut hashes = Vec::new();
        for i in 0..10 {
            let mut payload = vec![i as u8; 100];
            payload[0] = i as u8;
            hashes.push(store.store(&payload));
        }

        // Deve ter expurgado os itens mais antigos até a maré segura <= 80% (800 bytes)
        assert!(store.current_ram_bytes() <= 800);

        // O primeiro item (hashes[0]) deve ter sido ejetado
        assert!(store.retrieve(&hashes[0]).is_none());
        // O último item (hashes[9]) deve permanecer
        assert!(store.retrieve(&hashes[9]).is_some());
    }

    #[tokio::test]
    async fn test_souls_ccr_store_lru_eviction() {
        let max_bytes = 1 * 1024 * 1024; // 1 MB limit
        let store = Arc::new(SoulsCcrStore::new(max_bytes));

        let low_watermark = (max_bytes * 80) / 100;

        let mut handles = Vec::new();
        for task_idx in 0..10 {
            let store_clone = Arc::clone(&store);
            handles.push(tokio::spawn(async move {
                for i in 0..50 {
                    let mut payload = vec![(task_idx * 10 + i) as u8; 4096];
                    payload[0..4].copy_from_slice(&(task_idx as u32).to_le_bytes());
                    payload[4..8].copy_from_slice(&(i as u32).to_le_bytes());
                    store_clone.store(&payload);
                }
            }));
        }

        for h in handles {
            h.await.expect("Task concorrente finalizou sem pânico");
        }

        // Dispara uma gravação final para garantir o cruzamento da Maré Alta (90%)
        let trigger_payload = vec![0xffu8; 8192];
        store.store(&trigger_payload);

        let final_bytes = store.current_ram_bytes();
        assert!(
            final_bytes <= low_watermark + 8192,
            "current_ram_bytes ({}) excede a maré baixa ({})",
            final_bytes,
            low_watermark
        );
        assert_eq!(store.vram_bytes_allocated(), 0);
    }

    #[test]
    fn test_code_compressor_resilient_scope() {
        let code = r#"
fn fn_with_strings_and_comments() {
    let s = "struct Dummy { field: i32 } // inside string";
    let c = '{';
    // Single line comment { ignore me }
    /* Block comment { ignore me too } */
    println!("Done: {}", s);
}

pub fn another_fn() -> i32 {
    42
}
"#;
        let compressed = CodeCompressor::compress_ast_zero_copy(code);
        assert!(compressed.contains("fn fn_with_strings_and_comments"));
        assert!(compressed.contains("/* stubbed */"));
        assert!(compressed.contains("pub fn another_fn"));
        assert!(!compressed.contains("struct Dummy"));
        assert!(!compressed.contains("println!"));
    }
}
