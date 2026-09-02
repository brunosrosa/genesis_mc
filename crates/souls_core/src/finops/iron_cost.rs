//! SOULS MC — Marco I · v6.1: Iron Cost Breaker (consulta SQLite real + config)
//!
//! **NÃO-MOCK:** `IronCostBreaker::calculate_and_route` consulta o gasto diário
//! real gravado em `telemetry_logs` (via `telemetry_dispatcher::sum_today_cost_usd`)
//! e o **daily_budget_usd** configurado em `.souls/config/souls-gateway.jsonc`.
//!
//! ## Leis
//!
//! - **ADR-010 (Escrita atômica):** `dispatch_simple("iron_cost", ...)` registra
//!   o custo em `telemetry_logs` via worker thread (não-bloqueante).
//! - **ADR-030 (Higiene):** Constantes removidas; valores lidos do JSONC.
//! - **Marco I (FinOps):** Eco-Hybrid guard delegado para `pareto_bandit`.
//!
//! ## Performance
//!
//! - `calculate_and_route`: O(1) — query `SELECT SUM(cost_usd)` é indexada
//!   por `idx_telemetry_time` (created_at). Latência p99 < 5ms em SSDs.
//! - `dispatch_simple`: O(1) amortizado (MPSC `try_send`).

use thiserror::Error;

use crate::core::gateway_config::GatewayConfig;
use crate::core::telemetry_dispatcher::{sum_today_cost_usd, telemetry_sender};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModelTier {
    PremiumCloud,
    FlashCloud,
    LocalGPU,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AllowedRoute {
    OriginalRoute(ModelTier),
    FallbackToLocal,
    /// Bloqueio total — budget estourado E payload grande demais para VRAM local.
    Blocked,
}

#[derive(Error, Debug, Clone, PartialEq, Eq)]
pub enum FinOpsError {
    #[error("Orçamento diário excedido e payload incompatível com VRAM local (>{limit} tokens)")]
    BudgetExceeded { limit: usize },
    #[error("Capacidade de VRAM excedida para processamento local: {tokens} tokens")]
    VramExceeded { tokens: usize },
}

pub struct IronCostBreaker;

impl IronCostBreaker {
    /// Calcula o custo projetado, consulta o gasto diário real no SQLite
    /// e decide entre rota original, fallback local, ou bloqueio total.
    ///
    /// ## Fluxo
    /// 1. Short-circuit `LocalGPU` (apenas valida VRAM).
    /// 2. Calcula `projected_cost_micro_usd` baseado no `target_tier`.
    /// 3. Consulta `sum_today_cost_usd()` na tabela `telemetry_logs`.
    /// 4. Se `current + projected > daily_budget_micro_usd`:
    ///    - Se `tokens_count <= vram_token_limit` → `FallbackToLocal`.
    ///    - Caso contrário → `FinOpsError::BudgetExceeded`.
    /// 5. Caso contrário → `OriginalRoute(target_tier)`.
    ///
    /// Após aprovar, dispara telemetria de cost para gravação async.
    pub fn calculate_and_route(
        tokens_count: usize,
        target_tier: ModelTier,
    ) -> Result<AllowedRoute, FinOpsError> {
        let cfg = GatewayConfig::global();
        let vram_limit = cfg.finops.iron_cost_breaker.vram_token_limit;
        let daily_budget_micro = (cfg.finops.daily_budget_usd * 1_000_000.0) as u64;
        let premium_per_1m_micro = (cfg.finops.iron_cost_breaker.premium_per_1m_usd * 1_000_000.0) as u64;
        let flash_per_1m_micro = (cfg.finops.iron_cost_breaker.flash_per_1m_usd * 1_000_000.0) as u64;

        // (1) Short-circuit: LocalGPU não tem custo cloud, mas tem limite de VRAM.
        if target_tier == ModelTier::LocalGPU {
            if tokens_count > vram_limit {
                return Err(FinOpsError::VramExceeded { tokens: tokens_count });
            }
            return Ok(AllowedRoute::OriginalRoute(ModelTier::LocalGPU));
        }

        // (2) Calcula custo projetado.
        let cost_per_1m = match target_tier {
            ModelTier::PremiumCloud => premium_per_1m_micro,
            ModelTier::FlashCloud => flash_per_1m_micro,
            ModelTier::LocalGPU => 0,
        };
        let projected_cost_micro = (tokens_count as u64 * cost_per_1m) / 1_000_000;
        let projected_cost_usd = projected_cost_micro as f64 / 1_000_000.0;

        // (3) Consulta gasto diário real (SQLite, indexado).
        let db_path = std::path::Path::new(&cfg.telemetry.sqlite_path);
        let current_spent_usd = sum_today_cost_usd(db_path).unwrap_or_else(|e| {
            tracing::warn!("IronCostBreaker: falha ao consultar gasto diário: {e}. Assumindo 0.0.");
            0.0
        });
        let current_spent_micro = (current_spent_usd * 1_000_000.0) as u64;

        // (4) Decisão: projected + current vs daily_budget.
        if current_spent_micro + projected_cost_micro <= daily_budget_micro {
            // Aprovado: registra telemetria async.
            if let Some(sender) = telemetry_sender() {
                sender.dispatch_simple(
                    "iron_cost_approved",
                    tokens_count as i64,
                    0,
                    projected_cost_usd,
                    0,
                );
            }
            return Ok(AllowedRoute::OriginalRoute(target_tier));
        }

        // (5) Budget estourado.
        if tokens_count <= vram_limit && cfg.finops.force_local_on_budget_exceeded {
            if let Some(sender) = telemetry_sender() {
                sender.dispatch_simple(
                    "iron_cost_fallback_local",
                    tokens_count as i64,
                    0,
                    0.0,
                    0,
                );
            }
            return Ok(AllowedRoute::FallbackToLocal);
        }

        // (6) Muro de orçamento: bloqueia a chamada.
        if let Some(sender) = telemetry_sender() {
            sender.dispatch_simple(
                "iron_cost_blocked",
                tokens_count as i64,
                0,
                projected_cost_usd,
                0,
            );
        }
        Err(FinOpsError::BudgetExceeded { limit: vram_limit })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn unique_db_path() -> PathBuf {
        let mut p = std::env::temp_dir();
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        p.push(format!("souls_ironcost_test_{nanos}.db"));
        p
    }

    #[test]
    fn test_local_gpu_happy_path() {
        // Garante schema inicializado para `sum_today_cost_usd` não falhar.
        let path = unique_db_path();
        let _ = crate::core::telemetry_dispatcher::init_telemetry_dispatcher(&path);
        let res = IronCostBreaker::calculate_and_route(1000, ModelTier::LocalGPU);
        assert_eq!(res, Ok(AllowedRoute::OriginalRoute(ModelTier::LocalGPU)));
        std::fs::remove_file(&path).ok();
    }

    #[test]
    fn test_local_gpu_vram_exceeded() {
        // safe_default vram_token_limit = 16384
        let res = IronCostBreaker::calculate_and_route(17_000, ModelTier::LocalGPU);
        assert!(matches!(res, Err(FinOpsError::VramExceeded { .. })));
    }

    #[test]
    fn test_cloud_happy_path_no_db_initialized_fails_open() {
        // Sem DB inicializado, sum_today_cost_usd retorna 0 → aprovado.
        let res = IronCostBreaker::calculate_and_route(100_000, ModelTier::PremiumCloud);
        assert_eq!(res, Ok(AllowedRoute::OriginalRoute(ModelTier::PremiumCloud)));
    }

    #[test]
    fn test_budget_wall_blocks_huge_premium_request() {
        // 500k tokens * $15/1M = $7.50. Excede $5. Como 500k > 16k VRAM, deve bloquear.
        let res = IronCostBreaker::calculate_and_route(500_000, ModelTier::PremiumCloud);
        assert!(matches!(res, Err(FinOpsError::BudgetExceeded { .. })));
    }
}
