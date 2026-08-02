//! Tipos de dados da camada de Observabilidade Cognitiva (Marco 3.7 Fase B).
//!
//! Reflete o schema v3 do `souls_state.db` (`file_access_logs`,
//! `telemetry_logs`). Mantem zero-cost abstractions: apenas `i64`, `String`,
//! `f64` — sem `Option<Box<dyn Trait>>` ou `Arc<Mutex<...>>`.

/// Schema da tabela `file_access_logs` (v3).
///
/// Append-only: cada invocacao de tool que toca filesystem (read, edit,
/// get_ast, multi_read, smart_read, souls_stub_fill, headroom_retrieve,
/// tree, outline) gera exatamente um registro.
///
/// O indice `(file_path, accessed_at)` otimiza a query do [`super::heatmap`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileAccessLog {
    /// Chave primaria auto-incrementada.
    pub id: i64,
    /// Caminho do arquivo acessado (relativo ao workspace root ou absoluto).
    pub file_path: String,
    /// Nome canonico da tool MCP que originou o acesso (ex: "read", "edit").
    pub tool: String,
    /// Timestamp do acesso em epoch seconds (Unix time).
    pub accessed_at: i64,
}

/// Schema da tabela `telemetry_logs` (v3).
///
/// Cada tool MCP que consome tokens pode emitir um registro. Eficiencia E3
/// e calculada em [`super::e3_efficiency`].
///
/// O indice `(tool, created_at)` otimiza agregacoes por ferramenta.
#[derive(Debug, Clone, PartialEq)]
pub struct TelemetryLog {
    pub id: i64,
    /// Nome canonico da tool (ex: "compress", "souls_compress").
    pub tool: String,
    /// Tokens de entrada (input prompt). `0` quando nao aplicavel.
    pub tokens_in: i64,
    /// Tokens de saida (response body). `0` quando nao aplicavel.
    pub tokens_out: i64,
    /// Custo da operacao em USD (decimal; FinOps de Cloud Brain). `0.0` para
    /// tools locais sem custo monetario.
    pub cost_usd: f64,
    /// Duracao da operacao em milissegundos (latencia observada).
    pub duration_ms: i64,
    /// Timestamp de criacao em epoch seconds.
    pub created_at: i64,
}
