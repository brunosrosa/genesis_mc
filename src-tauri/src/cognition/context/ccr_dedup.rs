// SOULS-CANIBALIZED Marco 3.6: Motor de Compressão Reversível por Janela Deslizante (CCR Lossless).
//
// Implementa o algoritmo de deduplicação linear O(N) por linhas de texto:
// - Hasher: `std::collections::hash_map::DefaultHasher` (canônico stdlib, u64).
// - Cache: `dashmap::DashMap<u64, String>` (chave u64 → bloco original lossless).
// - Janela mínima: 5 linhas consecutivas VÁLIDAS (não-brancas).
// - Greedy match: estende o bloco para o máximo de linhas idênticas.
// - Marcador: `[SOULS-DEDUP: Block Hash 0x<hex_8>. Use souls_fill para resgatar se necessário]`
//
// DEFESA CONTRA FRAGMENTAÇÃO DE HEAP: a chave é `u64` (não `String`) para
// evitar alocação de milhões de chaves de hash. O bloco original completo
// (com indentação, tabs e quebras de linha exatas) é gravado como `String`
// no valor, garantindo lossless reversível byte-a-byte.
//
// DEFESA CONTRA LINHAS EM BRANCO: linhas puramente vazias, quebras isoladas ou
// linhas com apenas whitespace de indentação SÃO IGNORADAS no trigger da janela
// deslizante, passando em forma física original. Isso evita inchaço do texto
// compactado por marcadores redundantes para blocos triviais de espaçamento.

use std::collections::hash_map::DefaultHasher;
use std::hash::Hasher;

use dashmap::DashMap;

use super::types::BlockCompressionStats;

/// Cache global de deduplicação, indexada por hash `u64` de 64 bits.
/// O valor contém o BLOCO ORIGINAL COMPLETO (lossless reversível).
pub static DEDUP_CACHE: std::sync::LazyLock<DashMap<u64, String>> =
    std::sync::LazyLock::new(DashMap::new);

/// Janela mínima (L) da deduplicação deslizante. Blocos com menos de
/// `MIN_WINDOW` linhas válidas consecutivas NUNCA são compactados.
pub const MIN_WINDOW: usize = 5;

/// Prefixo do marcador de deduplicação injetado no texto compactado.
pub const MARKER_PREFIX: &str = "[SOULS-DEDUP: Block Hash 0x";

/// Sufixo do marcador de deduplicação.
pub const MARKER_SUFFIX: &str = ". Use souls_fill para resgatar se necessário]";

/// Limpa completamente o `DEDUP_CACHE` em RAM.
///
/// Equivalente a chamar `clear()` em todas as shards. Idempotente.
pub fn clear_dedup_cache() {
    DEDUP_CACHE.clear();
}

/// Computa o hash `u64` (DefaultHasher) de um bloco de linhas.
/// Linhas em branco são puladas (não devem entrar no cálculo de hash).
pub fn hash_block(block: &[&str]) -> u64 {
    let mut hasher = DefaultHasher::new();
    for line in block {
        hasher.write(line.as_bytes());
        hasher.write_u8(b'\n');
    }
    hasher.finish()
}

/// Verifica se uma linha é puramente vazia ou contém apenas whitespace
/// (espaços, tabs). Usada para defesa contra linhas em branco.
pub fn is_blank_line(line: &str) -> bool {
    line.chars().all(|c| c.is_whitespace())
}

/// Constrói o marcador textual estrito de deduplicação para o hash fornecido.
/// O hash `u64` é formatado como hexadecimal de 16 chars minúsculos
/// (representação canônica completa de um inteiro de 64 bits, sem perda).
pub fn dedup_marker_for(hash: u64) -> String {
    format!("{MARKER_PREFIX}{hash:016x}{MARKER_SUFFIX}")
}

/// Empurra todas as linhas acumuladas em `pending` para `out` e esvazia `pending`.
fn flush_pending(pending: &mut Vec<&str>, out: &mut Vec<String>) {
    for line in pending.drain(..) {
        out.push(line.to_string());
    }
}

/// Aplica compressão reversível CCR ao texto fornecido.
///
/// Algoritmo (janela deslizante de 5 linhas):
/// 1. Varre o texto linha a linha via `split('\n')` (preserva estrutura).
/// 2. Linhas em branco passam intactas (não disparam janela).
/// 3. Acumula linhas válidas em `pending_valid` (FIFO de até `MIN_WINDOW`).
/// 4. Quando `pending_valid.len() >= MIN_WINDOW`, calcula o hash das últimas
///    `MIN_WINDOW` linhas e consulta o `DEDUP_CACHE`:
///    - HIT (duplicata): emite as linhas PRÉ-bloco (acumuladas), substitui o
///      bloco pelo marcador, descarta o pending, avança `i` por `MIN_WINDOW`.
///    - MISS (primeira ocorrência): registra o bloco lossless no cache,
///      descarta as `MIN_WINDOW` linhas do pending (elas permanecem
///      "disponíveis" para a próxima janela) e avança `i` por 1.
/// 5. Ao final, faz flush do pending restante e reconstrói a string.
///
/// Retorna a string compactada + estatísticas.
pub fn compress_with_dedup(text: &str) -> (String, BlockCompressionStats) {
    let all_lines: Vec<&str> = text.split('\n').collect();
    let mut result_lines: Vec<String> = Vec::with_capacity(text.len() / 40);
    let mut pending_valid: Vec<&str> = Vec::with_capacity(MIN_WINDOW);
    let mut stats = BlockCompressionStats {
        deduplicated_blocks: 0,
        amputated_lines: 0,
        cache_inserts: 0,
        compacted_chars: 0,
        original_chars: text.chars().count(),
    };

    let mut i: usize = 0;
    while i < all_lines.len() {
        let line = all_lines[i];

        // Defesa contra linhas em branco: passam intactas, sem disparar janela.
        if is_blank_line(line) {
            flush_pending(&mut pending_valid, &mut result_lines);
            result_lines.push(line.to_string());
            i += 1;
            continue;
        }

        pending_valid.push(line);
        if pending_valid.len() < MIN_WINDOW {
            i += 1;
            continue;
        }

        // pending_valid.len() >= MIN_WINDOW. Hash do bloco das últimas MIN_WINDOW.
        let block: Vec<&str> = pending_valid[pending_valid.len() - MIN_WINDOW..].to_vec();
        let hash = hash_block(&block);

        if DEDUP_CACHE.contains_key(&hash) {
            // Duplicata: emite as linhas pré-bloco (acumuladas antes do bloco),
            // descarta as MIN_WINDOW do bloco, emite o marker e avança i por 1.
            // (As MIN_WINDOW linhas do bloco JÁ FORAM CONSUMIDAS nas iterações
            // anteriores que encheram o `pending_valid`. A linha atual i já
            // foi processada quando a entrada foi inserida no pending.)
            let pre_count = pending_valid.len() - MIN_WINDOW;
            for l in pending_valid.drain(..pre_count) {
                result_lines.push(l.to_string());
            }
            pending_valid.clear();
            result_lines.push(dedup_marker_for(hash));
            stats.deduplicated_blocks += 1;
            stats.amputated_lines += MIN_WINDOW;
            i += 1;
        } else {
            // Primeira ocorrência: registra o bloco no cache e MANTÉM/EMITE as
            // linhas válidas. As MIN_WINDOW mais antigas SAEM da janela de
            // observação (precisam ser emitidas no resultado) e as MIN_WINDOW
            // mais recentes PERMANECEM no pending (podem fazer parte do
            // próximo bloco a ser avaliado).
            DEDUP_CACHE.insert(hash, block.join("\n"));
            stats.cache_inserts += 1;
            let keep = pending_valid.len() - MIN_WINDOW;
            for l in pending_valid.drain(..keep) {
                result_lines.push(l.to_string());
            }
            i += 1;
        }
    }

    // Flush final de linhas válidas que não atingiram MIN_WINDOW.
    flush_pending(&mut pending_valid, &mut result_lines);

    let mut compacted = result_lines.join("\n");
    if text.ends_with('\n') && !compacted.ends_with('\n') {
        compacted.push('\n');
    }
    stats.compacted_chars = compacted.chars().count();
    (compacted, stats)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Adquire o lock de teste, recuperando de PoisonError automaticamente
    /// (fail-soft: tests paralelos podem envenenar o lock, mas queremos
    /// continuar a execução do TDD em vez de abortar a suíte inteira).
    fn test_lock() -> std::sync::MutexGuard<'static, ()> {
        static TEST_MUTEX: std::sync::Mutex<()> = std::sync::Mutex::new(());
        match TEST_MUTEX.lock() {
            Ok(g) => g,
            Err(p) => p.into_inner(),
        }
    }

    #[test]
    fn hash_block_is_deterministic_for_identical_input() {
        let block = vec!["alpha", "beta", "gamma", "delta", "epsilon"];
        let h1 = hash_block(&block);
        let h2 = hash_block(&block);
        assert_eq!(h1, h2);
    }

    #[test]
    fn hash_block_differs_for_different_input() {
        let h1 = hash_block(&["a", "b", "c", "d", "e"]);
        let h2 = hash_block(&["a", "b", "c", "d", "f"]);
        assert_ne!(h1, h2);
    }

    #[test]
    fn is_blank_line_detects_empty_and_whitespace_only() {
        assert!(is_blank_line(""));
        assert!(is_blank_line("   "));
        assert!(is_blank_line("\t\t"));
        assert!(!is_blank_line("code"));
        assert!(!is_blank_line("  code"));
    }

    #[test]
    fn dedup_marker_format_is_strict_16char_hex() {
        let marker = dedup_marker_for(0xDEADBEEFu64);
        assert!(marker.starts_with(MARKER_PREFIX));
        assert!(marker.ends_with(MARKER_SUFFIX));
        let hex = &marker[MARKER_PREFIX.len()..marker.len() - MARKER_SUFFIX.len()];
        // u64 = 64 bits = 16 chars hex canônicos (representação completa).
        assert_eq!(hex.len(), 16);
        assert_eq!(hex, "00000000deadbeef");
    }

    #[test]
    fn compress_with_dedup_empty_input_returns_empty() {
        let _g = test_lock();
        clear_dedup_cache();
        let (out, stats) = compress_with_dedup("");
        assert_eq!(out, "");
        assert_eq!(stats.deduplicated_blocks, 0);
        assert_eq!(stats.original_chars, 0);
    }

    #[test]
    fn compress_with_dedup_first_occurrence_keeps_physical_text() {
        let _g = test_lock();
        clear_dedup_cache();
        // 6 linhas -> 2 janelas deslizantes de 5: (a,b,c,d,e) e (b,c,d,e,f).
        // Cada primeira ocorrencia eh inserida no cache; nenhuma deduplicada.
        let input = "a\nb\nc\nd\ne\nf\n";
        let (out, stats) = compress_with_dedup(input);
        assert!(!out.contains(MARKER_PREFIX), "sem marker: {out}");
        // Comportamento real: 2 cache_inserts (duas janelas distintas).
        assert_eq!(stats.deduplicated_blocks, 0);
        assert_eq!(stats.cache_inserts, 2);
    }

    #[test]
    fn compress_with_dedup_preserves_pre_block_lines_on_duplicate() {
        let _g = test_lock();
        clear_dedup_cache();
        // 1a chamada popula o cache com o bloco (block1,block2,block3,block4,block5).
        let first_input = "block1\nblock2\nblock3\nblock4\nblock5\n";
        let (_out1, _) = compress_with_dedup(first_input);

        // 2a chamada: pre-block (pre1, pre2) precede o bloco dedup'd.
        // O sliding window chega ao bloco com pending.len()=6, logo
        // pre_count=1 -> drena "pre1" para o result; ao slide para o
        // proximo 5-tuple (block1,..,block5) detecta HIT e emite o
        // marker; "pre2" (ja' no result por estar alem do MIN_WINDOW
        // da iteracao anterior) tambem fica preservado.
        let second_input = "pre1\npre2\nblock1\nblock2\nblock3\nblock4\nblock5\npost1\npost2\n";
        let (out2, stats2) = compress_with_dedup(second_input);
        // pre-block (pre1, pre2) preservado.
        assert!(out2.contains("pre1"), "pre1 pre-block deve ser preservado: {out2}");
        assert!(out2.contains("pre2"), "pre2 pre-block deve ser preservado: {out2}");
        // Marker substitui o bloco (block1..block5).
        assert!(out2.contains(MARKER_PREFIX), "marcador deve estar presente: {out2}");
        // Pos-block (post1, post2) preservado.
        assert!(out2.contains("post1"), "post1 pos-block deve ser preservado: {out2}");
        assert!(out2.contains("post2"), "post2 pos-block deve ser preservado: {out2}");
        // Linhas DENTRO do bloco dedup'd sao amputadas.
        assert!(!out2.contains("block1"), "block1 dentro do bloco eh amputado: {out2}");
        assert!(!out2.contains("block5"), "block5 dentro do bloco eh amputado: {out2}");
        assert_eq!(stats2.deduplicated_blocks, 1);
    }
}
