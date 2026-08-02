// SOULS-CANIBALIZED Marco 3.6: Tipos canônicos do Conveyor Belt de Contexto.
//
// O tipo `BlockCompressionStats` encapsula a telemetria de uma operação de
// compressão por janela deslizante, útil para testes TDD e telemetria interna.

/// Telemetria de uma operação `compress_with_dedup`.
///
/// Não persiste em SQLite (Marco 3.6). Fica em heap efêmero e é consumido
/// pelo caller (handler MCP ou teste) imediatamente após a compressão.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BlockCompressionStats {
    /// Quantidade de blocos de 5+ linhas que foram identificados como duplicados
    /// e substituídos por marcadores `[SOULS-DEDUP: ...]`.
    pub deduplicated_blocks: usize,
    /// Quantidade de linhas físicas que foram AMPUTADAS do texto original
    /// (substituídas por marcadores).
    pub amputated_lines: usize,
    /// Quantidade de entradas NOVAS gravadas em `DEDUP_CACHE` durante esta operação.
    pub cache_inserts: usize,
    /// Tamanho do texto final compactado em caracteres.
    pub compacted_chars: usize,
    /// Tamanho do texto original em caracteres.
    pub original_chars: usize,
}

impl BlockCompressionStats {
    /// Redução percentual (0-100) entre original e compactado.
    /// Retorna 0 quando original_chars == 0 (evita divisão por zero).
    pub fn saved_percent(&self) -> u32 {
        if self.original_chars == 0 {
            return 0;
        }
        let ratio = self.compacted_chars as f64 / self.original_chars as f64;
        let saved = ((1.0 - ratio) * 100.0).round();
        saved.clamp(0.0, 100.0) as u32
    }
}
