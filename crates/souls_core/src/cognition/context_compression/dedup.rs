// SOULS V6 — MARCO 5.5.0: Compressor e Cache Host RAM (dedup.rs)
//
// Implementa a deduplicação CCR reversível e lossless na RAM do Host (Zero VRAM).
// - TRAVA DE LINHAS: Blocos com 5 ou menos linhas NÃO sofrem compressão. Blocos > 5 sofrem compressão.
// - CACHE ZERO-VRAM: `CCR_HOST_RAM_CACHE` encapsulando `DashMap<u64, String>`.
// - HASH u64: `DefaultHasher` do Rust std. Formatação hex {:x}.
// - MARCADOR: `[SOULS CCR: X linhas comprimidas. Para recuperar os dados integrais brutos, invoque a ferramenta headroom_retrieve(hash="<hash_hex>")]`

use std::collections::hash_map::DefaultHasher;
use std::hash::Hasher;
use std::sync::OnceLock;
use dashmap::DashMap;

pub static CCR_HOST_RAM_CACHE: OnceLock<DashMap<u64, String>> = OnceLock::new();

/// Retorna a referência global singleton ao DashMap na RAM do Host.
pub fn ccr_cache() -> &'static DashMap<u64, String> {
    CCR_HOST_RAM_CACHE.get_or_init(DashMap::new)
}

/// Limpa completamente o cache CCR em RAM.
pub fn clear_ccr_cache() {
    ccr_cache().clear();
}

pub const MARKER_PREFIX: &str = "[SOULS CCR: ";
pub const MARKER_MID: &str = " linhas comprimidas. Para recuperar os dados integrais brutos, invoque a ferramenta headroom_retrieve(hash=\"";
pub const MARKER_SUFFIX: &str = "\")]";

/// Computa o hash u64 (DefaultHasher) de uma string.
pub fn hash_text(text: &str) -> u64 {
    let mut hasher = DefaultHasher::new();
    hasher.write(text.as_bytes());
    hasher.finish()
}

/// Formata o marcador de resgate CCR estrito.
pub fn ccr_marker_for(lines_count: usize, hash_hex: &str) -> String {
    format!("{MARKER_PREFIX}{lines_count}{MARKER_MID}{hash_hex}{MARKER_SUFFIX}")
}

/// Aplica a compressão de janela deslizante baseada em eventos (CCR).
/// - Blocos com 5 ou menos linhas NÃO sofrem compressão.
/// - Blocos com mais de 5 linhas disparam a compressão CCR.
pub fn compress(text: &str) -> String {
    let lines: Vec<&str> = text.lines().collect();
    if lines.len() <= 5 {
        return text.to_string();
    }

    // Se o texto inteiro tem mais de 5 linhas, gera o hash u64 do texto original.
    let hash_u64 = hash_text(text);
    let hash_hex = format!("{:x}", hash_u64);

    // Armazena no DashMap na RAM do Host (Zero VRAM na dGPU)
    ccr_cache().insert(hash_u64, text.to_string());

    ccr_marker_for(lines.len(), &hash_hex)
}

/// Reidrata e expande marcadores de compressão CCR de volta para o texto original lossless na RAM.
pub fn rehydrate_ccr(text: &str) -> String {
    let mut result = text.to_string();

    let get_entry = |hash: u64| -> Option<String> {
        ccr_cache()
            .get(&hash)
            .map(|e| e.value().clone())
            .or_else(|| crate::cognition::context::ccr_dedup::DEDUP_CACHE.get(&hash).map(|e| e.value().clone()))
    };

    // Busca marcadores SOULS CCR
    let mut cursor = 0;
    while let Some(rel_start) = result[cursor..].find(MARKER_PREFIX) {
        let abs_start = cursor + rel_start;
        if let Some(hash_pos) = result[abs_start..].find("hash=\"") {
            let hash_start = abs_start + hash_pos + 6;
            if let Some(end_quote) = result[hash_start..].find('"') {
                let hash_end = hash_start + end_quote;
                let hash_hex = &result[hash_start..hash_end];
                if let Ok(hash_u64) = u64::from_str_radix(hash_hex, 16) {
                    if let Some(original_block) = get_entry(hash_u64) {
                        // Encontra o fim do marcador `")]`
                        if let Some(marker_end_rel) = result[abs_start..].find(MARKER_SUFFIX) {
                            let marker_end = abs_start + marker_end_rel + MARKER_SUFFIX.len();
                            result.replace_range(abs_start..marker_end, &original_block);
                            cursor = abs_start + original_block.len();
                            continue;
                        }
                    }
                }
            }
        }
        cursor = abs_start + MARKER_PREFIX.len();
    }

    // Suporte também para o marcador legado `[SOULS-DEDUP: Block Hash 0x<hex>. ...]`
    let legacy_prefix = "[SOULS-DEDUP: Block Hash 0x";
    let legacy_suffix = ". Use souls_fill para resgatar se necessário]";
    cursor = 0;
    while let Some(rel_start) = result[cursor..].find(legacy_prefix) {
        let abs_start = cursor + rel_start;
        let hex_start = abs_start + legacy_prefix.len();
        if hex_start + 16 <= result.len() {
            let hex = &result[hex_start..hex_start + 16];
            if let Ok(hash_u64) = u64::from_str_radix(hex, 16) {
                if let Some(original) = get_entry(hash_u64) {
                    if let Some(suf_rel) = result[abs_start..].find(legacy_suffix) {
                        let end_pos = abs_start + suf_rel + legacy_suffix.len();
                        result.replace_range(abs_start..end_pos, &original);
                        cursor = abs_start + original.len();
                        continue;
                    }
                }
            }
        }
        cursor = abs_start + legacy_prefix.len();
    }

    result
}
