//! Schema analítico do `souls_heuristic_vault.db` (SODA V6).
//!
//! Tabela `repo_heuristics` materializada em modo `STRICT` com as 85 colunas
//! canônicas da especificação do SODA V6.

use rusqlite::Connection;
use std::path::Path;

/// DDL canônico do schema V6 para `repo_heuristics` (85 colunas STRICT).
pub const REPO_HEURISTICS_V6_STRICT_DDL: &str = "
CREATE TABLE IF NOT EXISTS repo_heuristics (
    project_name TEXT PRIMARY KEY,
    status_atualizacao TEXT NOT NULL DEFAULT '',
    status_fase TEXT NOT NULL DEFAULT '',
    repo_url TEXT NOT NULL DEFAULT '',
    repo_analised_version TEXT NOT NULL DEFAULT '',
    repo_version TEXT NOT NULL DEFAULT '',
    ultima_versao_online TEXT NOT NULL DEFAULT '',
    indicacao_otimista_canibalizacao TEXT NOT NULL DEFAULT '',
    lote_id TEXT NOT NULL DEFAULT '',
    data_ultima_analise TEXT NOT NULL DEFAULT '',
    analise_origem TEXT NOT NULL DEFAULT '',
    declared_description TEXT NOT NULL DEFAULT '',
    proposta_original_resumo TEXT NOT NULL DEFAULT '',
    stack_base TEXT NOT NULL DEFAULT '',
    licenca TEXT NOT NULL DEFAULT '',
    lente_a_sentido_prod_ux TEXT NOT NULL DEFAULT '',
    lente_b_estrutura_arq TEXT NOT NULL DEFAULT '',
    lente_c_realidade_ops TEXT NOT NULL DEFAULT '',
    visao_do_enxame TEXT NOT NULL DEFAULT '',
    justificativa_decisao TEXT NOT NULL DEFAULT '',
    executive_verdict TEXT NOT NULL DEFAULT '',
    classificacao_terminal TEXT NOT NULL DEFAULT '',
    acao_de_canibalizacao TEXT NOT NULL DEFAULT '',
    categoria_arquitetural TEXT NOT NULL DEFAULT '',
    horizonte_extracao TEXT NOT NULL DEFAULT '',
    tipo_integracao TEXT NOT NULL DEFAULT '',
    categoria_nuance_tecnica TEXT NOT NULL DEFAULT '',
    integracao_papel_exato TEXT NOT NULL DEFAULT '',
    ouro_a_extrair TEXT NOT NULL DEFAULT '',
    deep_pattern TEXT NOT NULL DEFAULT '',
    transplantable_core TEXT NOT NULL DEFAULT '',
    logic_math_heuristic TEXT NOT NULL DEFAULT '',
    real_structural_problem TEXT NOT NULL DEFAULT '',
    must_components_prod_ux TEXT NOT NULL DEFAULT '',
    must_components_arq TEXT NOT NULL DEFAULT '',
    must_components_ops TEXT NOT NULL DEFAULT '',
    detected_toxic_deps TEXT NOT NULL DEFAULT '',
    do_not_absorb TEXT NOT NULL DEFAULT '',
    where_ai_should_not_enter TEXT NOT NULL DEFAULT '',
    adoptability_level TEXT NOT NULL DEFAULT '',
    longitudinal_sustainability TEXT NOT NULL DEFAULT '',
    abandonment_risk TEXT NOT NULL DEFAULT '',
    maintenance_burden TEXT NOT NULL DEFAULT '',
    onboarding_friction TEXT NOT NULL DEFAULT '',
    observability_operational TEXT NOT NULL DEFAULT '',
    recoverability_level TEXT NOT NULL DEFAULT '',
    degradation_behavior TEXT NOT NULL DEFAULT '',
    curation_burden TEXT NOT NULL DEFAULT '',
    time_to_first_clear_value TEXT NOT NULL DEFAULT '',
    imperfection_tolerance TEXT NOT NULL DEFAULT '',
    evolution_cost TEXT NOT NULL DEFAULT '',
    regulatory_risk TEXT NOT NULL DEFAULT '',
    bare_metal_fit TEXT NOT NULL DEFAULT '',
    extractability_level TEXT NOT NULL DEFAULT '',
    operability_level TEXT NOT NULL DEFAULT '',
    entropy_risk TEXT NOT NULL DEFAULT '',
    design_misuse_risk TEXT NOT NULL DEFAULT '',
    intrinsic_ethics_risk TEXT NOT NULL DEFAULT '',
    discipline_dependency TEXT NOT NULL DEFAULT '',
    risco_principal TEXT NOT NULL DEFAULT '',
    risco_linha_vermelha TEXT NOT NULL DEFAULT '',
    observacoes TEXT NOT NULL DEFAULT '',
    score_final REAL NOT NULL DEFAULT 0.0,
    score_fit_geral_soda REAL NOT NULL DEFAULT 0.0,
    score_fit_geral_souls REAL NOT NULL DEFAULT 0.0,
    score_philosophical_fit INT NOT NULL DEFAULT 0,
    score_bare_metal_fit INT NOT NULL DEFAULT 0,
    score_architectural_extractability INT NOT NULL DEFAULT 0,
    score_operability INT NOT NULL DEFAULT 0,
    score_creep_risk INT NOT NULL DEFAULT 0,
    score_runtime_sovereignty INT NOT NULL DEFAULT 0,
    score_model_logic_value INT NOT NULL DEFAULT 0,
    score_ethics_safety INT NOT NULL DEFAULT 0,
    score_intrinsic_risk INT NOT NULL DEFAULT 0,
    capability_nature_primary TEXT NOT NULL DEFAULT '',
    architectural_topology TEXT NOT NULL DEFAULT '',
    runtime_sovereignty_fit TEXT NOT NULL DEFAULT '',
    local_first_fit TEXT NOT NULL DEFAULT '',
    temporal_stability TEXT NOT NULL DEFAULT '',
    score_architectural_priority REAL NOT NULL DEFAULT 0.0,
    score_human_product_priority REAL NOT NULL DEFAULT 0.0,
    score_absorption_readiness REAL NOT NULL DEFAULT 0.0,
    score_operational_priority REAL NOT NULL DEFAULT 0.0,
    score_sustainability_adjusted_fit REAL NOT NULL DEFAULT 0.0,
    valid_from INT NOT NULL DEFAULT 0,
    valid_to INT NOT NULL DEFAULT 0,
    embargo_status INT NOT NULL DEFAULT 0
) STRICT;
";

/// Garantidor de schema: aplica PRAGMAs e cria a tabela `repo_heuristics` STRICT V6.
pub fn ensure_heuristic_vault_v6_schema(conn: &Connection) -> Result<(), String> {
    conn.execute_batch(
        "PRAGMA foreign_keys = ON;
         PRAGMA journal_mode = WAL;
         PRAGMA busy_timeout = 5000;",
    )
    .map_err(|e| format!("Falha ao configurar PRAGMAs em souls_heuristic_vault.db: {e}"))?;

    conn.execute(REPO_HEURISTICS_V6_STRICT_DDL, [])
        .map_err(|e| format!("Falha ao criar tabela repo_heuristics STRICT V6: {e}"))?;

    Ok(())
}

/// Helper para inicializar conexão com o vault analítico.
pub fn init_vault_connection(db_path: &Path) -> Result<Connection, String> {
    let conn = Connection::open(db_path)
        .map_err(|e| format!("Falha ao abrir {}: {e}", db_path.display()))?;
    ensure_heuristic_vault_v6_schema(&conn)?;
    Ok(conn)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_heuristic_vault_v6_strict_schema() {
        let conn = Connection::open_in_memory().expect("abre :memory:");
        ensure_heuristic_vault_v6_schema(&conn).expect("cria schema V6 STRICT");

        // Valida que a tabela existe e está em modo STRICT (rejeita coerções incorretas de tipo)
        let res = conn.execute(
            "INSERT INTO repo_heuristics (project_name, score_final) VALUES (12345, 'texto_invalido_para_real')",
            [],
        );
        assert!(res.is_err(), "Tabela STRICT deve rejeitar tipo incorreto em coluna REAL");
    }
}
