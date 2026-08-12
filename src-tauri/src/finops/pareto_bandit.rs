use serde::{Deserialize, Serialize};
use crate::core::hardware_profiler::SystemTopology;

/// Tiers de Roteamento da Cascata Zero-Trust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RoutingTier {
    /// Nível 0: Local Velocista (LFM 1.2B) - Tarefas simples (complexidade < 0.3) e baixo contexto. Custo $0.
    Tier0,
    /// Nível 1: Local Trator (Nemotron 4B) - Tarefas complexas (complexidade >= 0.3), cabe em VRAM (5.1GB) e barramento PCIe. Custo $0.
    Tier1,
    /// Nível 2: Nuvem Premium (DeepSeek/Claude) - Contexto estourou a física do host ou alta complexidade. Trava lambda.
    Tier2,
}

impl RoutingTier {
    pub fn name(&self) -> &'static str {
        match self {
            RoutingTier::Tier0 => "Tier 0 (Local Velocista - LFM 1.2B)",
            RoutingTier::Tier1 => "Tier 1 (Local Trator - Nemotron 4B)",
            RoutingTier::Tier2 => "Tier 2 (Nuvem Premium - DeepSeek/Claude)",
        }
    }

    pub fn is_local(&self) -> bool {
        matches!(self, RoutingTier::Tier0 | RoutingTier::Tier1)
    }
}

/// Candidato a Modelo para Avaliação de Utilidade Pareto
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelCandidate {
    pub model_id: String,
    pub tier: RoutingTier,
    /// q: Qualidade / Acurácia esperada do modelo (0.0 a 1.0)
    pub expected_quality: f32,
    /// c: Custo da API em microdólares (0 para modelos locais)
    pub api_cost_micro_usd: f32,
    /// l: Latência estimada em milissegundos
    pub estimated_latency_ms: f32,
}

impl ModelCandidate {
    pub fn new(
        model_id: impl Into<String>,
        tier: RoutingTier,
        expected_quality: f32,
        api_cost_micro_usd: f32,
        estimated_latency_ms: f32,
    ) -> Self {
        Self {
            model_id: model_id.into(),
            tier,
            expected_quality,
            api_cost_micro_usd,
            estimated_latency_ms,
        }
    }

    /// Calcula a utilidade Pareto U = q - (lambda * c) - (beta * l)
    pub fn calculate_utility(&self, lambda: f32, beta: f32) -> f32 {
        calculate_pareto_utility(
            self.expected_quality,
            self.api_cost_micro_usd,
            self.estimated_latency_ms,
            lambda,
            beta,
        )
    }
}

/// Limite rígido de VRAM para o Tier 1 (Nemotron 4B): 5.1 GB
pub const TIER1_VRAM_BUDGET_BYTES: u64 = (5.1 * 1024.0 * 1024.0 * 1024.0) as u64; // ~5.47 GB

/// Limiar mínimo do barramento PCIe para suportar VRAM Spillover sem colapso de throughput (GB/s)
pub const MIN_PCIE_BANDWIDTH_GBPS_FOR_SPILLOVER: f32 = 32.0;

/// PASSO 1: A EQUAÇÃO DE UTILIDADE DE PARETO E³
/// U = q - (lambda * c) - (beta * l)
/// - q = Qualidade/Acurácia esperada (0.0 a 1.0)
/// - c = Custo da API em microdólares ($0 para modelos locais)
/// - l = Latência estimada em milissegundos
/// - lambda = Marcapasso orçamentário diário do usuário
/// - beta = Sensibilidade à latência da tarefa atual
pub fn calculate_pareto_utility(
    q: f32,
    c: f32,
    l: f32,
    lambda: f32,
    beta: f32,
) -> f32 {
    q - (lambda * c) - (beta * l)
}

/// Estima o consumo total de memória do Tier 1 (Nemotron 4B) em bytes para um dado contagem de tokens.
/// Base: ~2.8 GB (pesos Q4_K_M) + KV Cache (512 KB / token).
pub fn estimate_tier1_vram_usage(token_count: u32) -> u64 {
    let base_weights = (2.8 * 1024.0 * 1024.0 * 1024.0) as u64;
    let kv_cache = (token_count as u64) * 512 * 1024;
    base_weights + kv_cache
}

/// Verifica se ocorre Spillover do KV Cache/pesos fora da VRAM alocável.
pub fn is_tier1_spillover(token_count: u32, topology: &SystemTopology) -> bool {
    let estimated_vram = estimate_tier1_vram_usage(token_count);
    let vram_limit = if topology.vram_total_bytes > 0 {
        topology.vram_total_bytes.min(TIER1_VRAM_BUDGET_BYTES)
    } else {
        TIER1_VRAM_BUDGET_BYTES
    };
    estimated_vram > vram_limit
}

/// PASSO 2 & PASSO 3: CASCATA ZERO-TRUST E GUILHOTINA DO HARDWARE
/// 
/// Hierarquia Zero-Trust:
/// - Tier 0 (Local Velocista LFM 1.2B): complexidade < 0.3 e baixo contexto (<= 2048 tokens). Custo $0.
/// - Tier 1 (Local Trator Nemotron 4B): complexidade >= 0.3, cabe em 5.1GB VRAM ou PCIe >= 32 GB/s. Custo $0.
/// - Tier 2 (Nuvem Premium DeepSeek/Claude): contexto excedido ou complexidade altíssima (>= 0.85) ou Spillover com PCIe < 32.0 GB/s.
pub fn select_optimal_route(
    task_complexity: f32,
    token_count: u32,
    topology: &SystemTopology,
) -> RoutingTier {
    // 1. Alta complexidade ou contexto massivo forçam diretamente Nuvem Premium (Tier 2)
    if task_complexity >= 0.85 || token_count > 16384 {
        return RoutingTier::Tier2;
    }

    // 2. Tarefa simples (< 0.3) e baixo contexto (<= 2048) -> Tier 0 (Local Velocista)
    if task_complexity < 0.3 && token_count <= 2048 {
        return RoutingTier::Tier0;
    }

    // 3. Avaliações do Tier 1 (Local Trator)
    let spillover = is_tier1_spillover(token_count, topology);
    let pcie_bw = topology.pcie_bandwidth_estimated_gbps.unwrap_or(0.0);
    let pcie_below_threshold = pcie_bw < MIN_PCIE_BANDWIDTH_GBPS_FOR_SPILLOVER;

    // PASSO 3: A GUILHOTINA DO HARDWARE
    // Se houver Spillover de VRAM E a largura de banda PCIe for < 32.0 GB/s, PROÍBE o Tier 1.
    if spillover && pcie_below_threshold {
        if task_complexity < 0.3 {
            return RoutingTier::Tier0;
        } else {
            return RoutingTier::Tier2;
        }
    }

    // Se couber na VRAM ou o barramento PCIe for rápido o suficiente (>= 32 GB/s)
    if task_complexity >= 0.3 {
        RoutingTier::Tier1
    } else {
        RoutingTier::Tier0
    }
}

/// Roteador ParetoBandit (ADR-FinOps)
#[derive(Debug, Clone)]
pub struct ParetoBanditRouter {
    /// Marcapasso orçamentário diário do usuário (lambda)
    pub daily_budget_lambda: f32,
}

impl ParetoBanditRouter {
    pub fn new(daily_budget_lambda: f32) -> Self {
        Self { daily_budget_lambda }
    }

    pub fn select_route(
        &self,
        task_complexity: f32,
        token_count: u32,
        topology: &SystemTopology,
    ) -> RoutingTier {
        select_optimal_route(task_complexity, token_count, topology)
    }

    /// Calculates quality deviation and adjusts lambda cost parameter if local model degrades
    pub fn get_adjusted_lambda(&self, elo_rating: f64, ema_score: f64) -> f32 {
        if elo_rating < 1150.0 || ema_score < 0.7 {
            let factor = 1.0 + (1200.0 - elo_rating) / 100.0;
            self.daily_budget_lambda * (factor as f32)
        } else {
            self.daily_budget_lambda
        }
    }

    /// Selects route with dynamic pacing based on local model ELO rating and EMA score
    pub fn select_route_with_pacing(
        &self,
        task_complexity: f32,
        token_count: u32,
        topology: &SystemTopology,
        elo_rating: f64,
        ema_score: f64,
    ) -> RoutingTier {
        if elo_rating < 1150.0 || ema_score < 0.7 {
            // Escalation pacing: force route to Tier 2 (Cloud) to preserve focus flow
            RoutingTier::Tier2
        } else {
            self.select_route(task_complexity, token_count, topology)
        }
    }

    /// Bootstraps route selection reading local model ratings directly from WeEvolveEngine
    pub fn select_route_with_weevolve(
        &self,
        task_complexity: f32,
        token_count: u32,
        topology: &SystemTopology,
        local_model_id: &str,
    ) -> RoutingTier {
        let engine = crate::cognition::learning::WeEvolveEngine::global();
        let (elo, ema) = engine.get_rating(local_model_id);
        self.select_route_with_pacing(task_complexity, token_count, topology, elo, ema)
    }

    pub fn score_candidate(&self, candidate: &ModelCandidate, beta: f32) -> f32 {
        candidate.calculate_utility(self.daily_budget_lambda, beta)
    }

    /// Seleciona o melhor candidato entre múltiplos modelos elegíveis usando a Utilidade de Pareto
    pub fn rank_candidates(
        &self,
        candidates: &[ModelCandidate],
        beta: f32,
    ) -> Option<ModelCandidate> {
        candidates
            .iter()
            .max_by(|a, b| {
                let score_a = self.score_candidate(a, beta);
                let score_b = self.score_candidate(b, beta);
                score_a.partial_cmp(&score_b).unwrap_or(std::cmp::Ordering::Equal)
            })
            .cloned()
    }
}

// ============================================================================
// Marco I · v6.1 — Route Decision canônico (FinOps-aware + Eco-Hybrid guard)
// ============================================================================

use crate::core::gateway_config::{GatewayConfig, RouteEndpoint};

/// Categoria semântica da tarefa. Eco-Hybrid (rotas gratuitas / free) é
/// vetado para categorias em `eco_hybrid_forbidden` do `GatewayConfig`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TaskKind {
    Factual,
    Classification,
    Summarization,
    CodeWrite,
    Refactor,
    Architecture,
    Other,
}

impl TaskKind {
    /// Heurística de classificação: presença de keywords no prompt.
    /// Ordem importa — primeiro match vence.
    pub fn classify_from_prompt(prompt: &str) -> Self {
        let lower = prompt.to_ascii_lowercase();
        if lower.contains("refactor") || lower.contains("reorganiz") || lower.contains("renomear") {
            TaskKind::Refactor
        } else if lower.contains("fn ") || lower.contains("function ") || lower.contains("def ")
            || lower.contains("implement") || lower.contains("code ") || lower.contains("código")
        {
            TaskKind::CodeWrite
        } else if lower.contains("arquitetura") || lower.contains("architecture") || lower.contains("design system") {
            TaskKind::Architecture
        } else if lower.contains("resum") || lower.contains("summariz") || lower.contains("tl;dr") {
            TaskKind::Summarization
        } else if lower.contains("classif") || lower.contains("categoriz") || lower.contains("rotul") {
            TaskKind::Classification
        } else if lower.contains("?") || lower.contains("quem") || lower.contains("quando")
            || lower.contains("onde") || lower.contains("what") || lower.contains("when") {
            TaskKind::Factual
        } else {
            TaskKind::Other
        }
    }

    pub fn is_eco_hybrid_forbidden(&self, cfg: &GatewayConfig) -> bool {
        let s = match self {
            TaskKind::CodeWrite => "code_write",
            TaskKind::Refactor => "refactor",
            TaskKind::Architecture => "architecture",
            TaskKind::Factual => "factual",
            TaskKind::Classification => "classification",
            TaskKind::Summarization => "summarization",
            TaskKind::Other => "other",
        };
        cfg.finops.eco_hybrid_forbidden.iter().any(|p| p == s)
    }
}

/// Decisão concreta retornada pelo `evaluate_route_decision`.
#[derive(Debug, Clone, PartialEq)]
pub struct RouteDecision {
    pub endpoint: RouteEndpoint,
    pub task_kind: TaskKind,
    /// Razões estruturadas (auditáveis). Cada veto vira um item.
    pub reasons: Vec<String>,
    /// Bloqueio total (true) ou apenas sugestão de rota alternativa (false).
    pub hard_block: bool,
}

/// Avalia qual endpoint deve atender a requisição. Considera:
/// 1. Whitelist/blacklist de modelos.
/// 2. Eco-Hybrid guard (tasks `code_write`/`refactor`/`architecture` NUNCA vão para rotas gratuitas).
/// 3. Complexity score (calculado de `task_complexity`).
/// 4. Daily budget (consultado externamente via `IronCostBreaker`).
pub fn evaluate_route_decision(
    task_kind: TaskKind,
    task_complexity: f32,
    requested_model: &str,
) -> RouteDecision {
    let cfg = GatewayConfig::global();
    let mut reasons = Vec::new();

    // (1) Whitelist/blacklist check.
    if cfg.models.blacklist.iter().any(|m| m == requested_model) {
        return RouteDecision {
            endpoint: cfg.routes.heavy_brain_endpoint.clone(),
            task_kind,
            reasons: vec![format!("modelo '{}' em blacklist", requested_model)],
            hard_block: true,
        };
    }
    let in_whitelist = cfg.models.whitelist.iter().any(|m| m == requested_model);
    if !in_whitelist && !requested_model.is_empty() {
        reasons.push(format!("modelo '{}' não está na whitelist", requested_model));
    }

    // (2) Eco-Hybrid guard: tasks proibidas nunca vão para free/cheap routes.
    if task_kind.is_eco_hybrid_forbidden(&cfg) {
        reasons.push(format!(
            "task {:?} proibida para rotas Eco-Hybrid (eco_hybrid_forbidden)",
            task_kind
        ));
        // Força heavy_brain_endpoint.
        return RouteDecision {
            endpoint: cfg.routes.heavy_brain_endpoint.clone(),
            task_kind,
            reasons,
            hard_block: false, // Não bloqueia; só redireciona
        };
    }

    // (3) Complexity-based routing.
    if task_complexity <= cfg.routes.fast_worker_endpoint.max_complexity {
        reasons.push(format!(
            "complexity {:.2} <= {:.2} (fast_worker)",
            task_complexity, cfg.routes.fast_worker_endpoint.max_complexity
        ));
        return RouteDecision {
            endpoint: cfg.routes.fast_worker_endpoint.clone(),
            task_kind,
            reasons,
            hard_block: false,
        };
    }

    // Default: heavy_brain.
    reasons.push(format!(
        "complexity {:.2} > {:.2} (heavy_brain)",
        task_complexity, cfg.routes.fast_worker_endpoint.max_complexity
    ));
    RouteDecision {
        endpoint: cfg.routes.heavy_brain_endpoint.clone(),
        task_kind,
        reasons,
        hard_block: false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::hardware_profiler::CpuInstructionSet;

    fn mock_topology(pcie_gbps: Option<f32>, vram_gb: f32) -> SystemTopology {
        SystemTopology {
            gpu_name: "RTX 2060m".to_string(),
            vram_total_bytes: (vram_gb * 1024.0 * 1024.0 * 1024.0) as u64,
            ram_total_bytes: 32 * 1024 * 1024 * 1024,
            is_dedicated_gpu: true,
            primary_simd_extension: CpuInstructionSet::Avx2,
            is_nvme_ssd: true,
            pcie_bandwidth_estimated_gbps: pcie_gbps,
        }
    }

    #[test]
    fn test_pareto_utility_math() {
        // U = q - (lambda * c) - (beta * l)
        // q = 0.9, c = 10 microUSD, l = 100ms, lambda = 0.01, beta = 0.001
        // U = 0.9 - (0.01 * 10) - (0.001 * 100) = 0.9 - 0.1 - 0.1 = 0.7
        let u = calculate_pareto_utility(0.9, 10.0, 100.0, 0.01, 0.001);
        assert!((u - 0.7).abs() < 1e-5);
    }

    #[test]
    fn test_tier0_selection_for_simple_tasks() {
        let topo = mock_topology(Some(15.75), 6.0);
        let tier = select_optimal_route(0.15, 512, &topo);
        assert_eq!(tier, RoutingTier::Tier0);
    }

    #[test]
    fn test_tier1_selection_when_within_vram() {
        let topo = mock_topology(Some(15.75), 6.0);
        // Complexidade 0.5, 1000 tokens (cabe no limite de 5.1GB VRAM)
        let tier = select_optimal_route(0.5, 1000, &topo);
        assert_eq!(tier, RoutingTier::Tier1);
    }

    #[test]
    fn test_hardware_guillotine_blocks_tier1_on_low_pcie_spillover() {
        let topo = mock_topology(Some(15.75), 6.0); // PCIe Gen3x16 ~ 15.75 GB/s (< 32.0 GB/s)
        // 10_000 tokens força Spillover de VRAM (> 5.1GB)
        let tier = select_optimal_route(0.5, 10_000, &topo);
        // Guilhotina deve proibir Tier 1 e forçar fallback para Tier 2 (Nuvem)
        assert_eq!(tier, RoutingTier::Tier2);
    }

    #[test]
    fn test_tier1_allowed_on_high_pcie_bandwidth_even_with_spillover() {
        let topo = mock_topology(Some(35.0), 6.0); // PCIe Gen4x16 ~ 35.0 GB/s (>= 32.0 GB/s)
        let tier = select_optimal_route(0.5, 10_000, &topo);
        // Com barramento suficiente (>= 32 GB/s), Tier 1 é mantido
        assert_eq!(tier, RoutingTier::Tier1);
    }

    #[test]
    fn test_tier2_selection_for_extreme_complexity() {
        let topo = mock_topology(Some(64.0), 16.0);
        let tier = select_optimal_route(0.9, 500, &topo);
        assert_eq!(tier, RoutingTier::Tier2);
    }

    // ========================================================================
    // Marco I · v6.1 — Testes do novo evaluate_route_decision
    // ========================================================================

    #[test]
    fn test_classify_code_write_task() {
        assert_eq!(
            TaskKind::classify_from_prompt("Please implement a function to parse JSON"),
            TaskKind::CodeWrite
        );
    }

    #[test]
    fn test_classify_factual_task() {
        assert_eq!(
            TaskKind::classify_from_prompt("What is the capital of France?"),
            TaskKind::Factual
        );
    }

    #[test]
    fn test_classify_refactor_task() {
        assert_eq!(
            TaskKind::classify_from_prompt("Refactor this module to use async/await"),
            TaskKind::Refactor
        );
    }

    #[test]
    fn test_classify_summarization_task() {
        assert_eq!(
            TaskKind::classify_from_prompt("Summarize this paper in 200 words"),
            TaskKind::Summarization
        );
    }

    #[test]
    fn test_route_decision_redirects_code_write_to_heavy_brain() {
        // Sem depender do JSONC: usa safe_default (forbidden inclui code_write).
        let decision = evaluate_route_decision(TaskKind::CodeWrite, 0.5, "anthropic/claude-3-5-haiku");
        assert!(
            decision.reasons.iter().any(|r| r.contains("proibida para rotas Eco-Hybrid")),
            "code_write deve ser bloqueado para eco-hybrid: {:?}", decision.reasons
        );
    }

    #[test]
    fn test_route_decision_blocks_blacklisted_model() {
        // O safe_default coloca "gpt-3.5-turbo-instruct" na blacklist.
        let decision = evaluate_route_decision(TaskKind::Factual, 0.1, "gpt-3.5-turbo-instruct");
        assert!(decision.hard_block, "blacklist deve gerar hard_block");
        assert!(decision.reasons[0].contains("blacklist"));
    }

    #[test]
    fn test_route_decision_simple_task_uses_fast_worker() {
        let decision = evaluate_route_decision(TaskKind::Factual, 0.1, "anthropic/claude-3-5-haiku");
        assert!(
            decision.endpoint.model.contains("haiku")
                || decision.endpoint.estimated_cost_per_1m_usd <= 1.0,
            "factual simples deve cair em fast_worker: {}", decision.endpoint.model
        );
    }
}
