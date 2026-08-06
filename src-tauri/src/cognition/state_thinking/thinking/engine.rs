//! Motor socrático in-RAM (`ThinkingEngine`) e Roteador Metacognitivo (`ThinkingParadigm`).
//!
//! MARCO 4.7.0 / 4.7.1 — Transplante do ThinkingEngine (DoD).
//! - Assinatura de sessões/pensamentos via UUIDv7 em RAM.
//! - Suporte a 4 paradigmas: `LinearCoT`, `TreeToT`, `GraphGoT`, `CouncilMAD`.
//! - Insumo de contexto `ThinkingContext` e método dinâmico `determine_paradigm`.
//! - Validação estrita de integridade (`RevisionWithoutTarget`, `OverthinkingThresholdBreached`).

use std::collections::HashMap;
use serde::{Deserialize, Serialize};

use crate::cognition::memory_graph::errors::CognitiveError;
use crate::cognition::memory_graph::uuid::generate_uuid_v7;
use crate::cognition::thinking::types::{
    BranchId, BranchSummary, ThoughtData, ThoughtId, ThinkingResponse,
};

/// Teto absoluto padrão do disjuntor de Overthinking.
pub const DEFAULT_HARD_LIMIT: u32 = 5;

/// Teto elástico sob autorização HITL explícita do Arquiteto (Svelte 5).
pub const HITL_EXTENDED_LIMIT: u32 = 7;

/// Paradigmas de raciocínio socrático suportados pelo ThinkingEngine.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum ThinkingParadigm {
    /// Raciocínio linear padrão "passo a passo" para tarefas simples.
    #[default]
    LinearCoT,
    /// Árvore de pensamentos com backtracking automático em falhas do Ralph Loop.
    TreeToT,
    /// Fusão e síntese de ramos concorrentes (merge_sessions) via SQLite.
    GraphGoT,
    /// Debate Multi-Agente em background para decisões críticas.
    CouncilMAD,
}

/// Insumo de contexto para decisão dinâmica do Roteador Metacognitivo.
#[derive(Debug, Clone, PartialEq)]
pub struct ThinkingContext {
    pub file_path: String,
    pub impact_score: f64,         // Provém do souls_mcp.repo_impact
    pub consecutive_failures: u32,  // Contador de falhas do Ralph Loop corrente
    pub hitl_authorized: bool,
}

/// Estado in-RAM de uma sessão socrática com roteador metacognitivo.
pub struct ThinkingEngine {
    session_id: String,
    main_thread: Vec<ThoughtData>,
    branches: HashMap<BranchId, Vec<ThoughtId>>,
    hard_limit: u32,
    hitl_authorized: bool,
    paradigm: ThinkingParadigm,
}

impl Default for ThinkingEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl ThinkingEngine {
    /// Cria uma sessão socrática assinada com UUIDv7 e teto padrão (5 pensamentos).
    pub fn new() -> Self {
        Self {
            session_id: format!("thn_{}", generate_uuid_v7()),
            main_thread: Vec::with_capacity(DEFAULT_HARD_LIMIT as usize),
            branches: HashMap::new(),
            hard_limit: DEFAULT_HARD_LIMIT,
            hitl_authorized: false,
            paradigm: ThinkingParadigm::LinearCoT,
        }
    }

    /// Determina deterministicamente o paradigma de raciocínio com base no contexto.
    /// - Regra 1: Se `consecutive_failures > 1`, força compulsoriamente `TreeToT`.
    /// - Regra 2: Se `impact_score > 3.0`, seleciona `CouncilMAD`.
    /// - Regra 3: Caso contrário, seleciona `LinearCoT`.
    pub fn determine_paradigm(ctx: &ThinkingContext) -> ThinkingParadigm {
        if ctx.consecutive_failures > 1 {
            ThinkingParadigm::TreeToT
        } else if ctx.impact_score > 3.0 {
            ThinkingParadigm::CouncilMAD
        } else {
            ThinkingParadigm::LinearCoT
        }
    }

    /// Instancia um `ThinkingEngine` ajustado para o `ThinkingContext` fornecido.
    pub fn with_context(ctx: &ThinkingContext) -> Self {
        let mut engine = Self::new();
        let paradigm = Self::determine_paradigm(ctx);
        engine.set_paradigm(paradigm);
        if ctx.hitl_authorized {
            engine.authorize_hitl();
        }
        engine
    }

    /// Paradigma de raciocínio ativo.
    pub fn paradigm(&self) -> ThinkingParadigm {
        self.paradigm
    }

    /// Define o paradigma de raciocínio manualmente.
    pub fn set_paradigm(&mut self, paradigm: ThinkingParadigm) {
        self.paradigm = paradigm;
    }

    /// Altera deterministicamente o paradigma e a profundidade de busca
    /// com base na complexidade da tarefa e erros repetidos de compilação (Ralph Loop).
    pub fn select_paradigm(&mut self, task_complexity: u32, consecutive_ralph_failures: u32) {
        let ctx = ThinkingContext {
            file_path: String::new(),
            impact_score: task_complexity as f64,
            consecutive_failures: consecutive_ralph_failures,
            hitl_authorized: self.hitl_authorized,
        };
        self.paradigm = Self::determine_paradigm(&ctx);
    }

    /// Retorna a profundidade de busca permitida para o paradigma atual.
    pub fn max_search_depth(&self) -> u32 {
        match self.paradigm {
            ThinkingParadigm::LinearCoT => 5,
            ThinkingParadigm::TreeToT => 7,
            ThinkingParadigm::GraphGoT => 7,
            ThinkingParadigm::CouncilMAD => 7,
        }
    }

    /// Ativa a flag HITL (estica teto para 7 pensamentos). Idempotente.
    pub fn authorize_hitl(&mut self) {
        self.hitl_authorized = true;
        self.hard_limit = HITL_EXTENDED_LIMIT;
    }

    /// Teto atual (5 ou 7).
    pub fn current_limit(&self) -> u32 {
        self.hard_limit
    }

    /// Contagem de pensamentos registrados na sessão.
    pub fn thought_count(&self) -> u32 {
        self.main_thread.len() as u32
    }

    /// Verifica se a sessão de pensamento foi encerrada.
    pub fn is_closed(&self) -> bool {
        self.main_thread
            .last()
            .map(|t| !t.next_thought_needed)
            .unwrap_or(false)
            || self.thought_count() >= self.hard_limit
    }

    /// Registra um novo pensamento socrático.
    /// Valida obrigatoriedade de target em revisões e impõe o disjuntor FinOps.
    pub fn push_thought(&mut self, t: ThoughtData) -> Result<ThinkingResponse, CognitiveError> {
        // 0) HITL late-binding: se o payload autorizar HITL, estica o teto.
        if t.hitl_authorized.unwrap_or(false) {
            self.authorize_hitl();
        }

        // 1) Validação de branching: revision ⇒ revises_thought obrigatório.
        if t.is_revision.unwrap_or(false) && t.revises_thought.is_none() {
            return Err(CognitiveError::RevisionWithoutTarget);
        }

        // 2) Validação de referência: revises_thought deve apontar para nó existente.
        if let Some(target) = t.revises_thought {
            if target < 1 || (target as usize) > self.main_thread.len() {
                return Err(CognitiveError::OrphanBranch(format!(
                    "revises_thought={target}"
                )));
            }
        }

        // 3) Validação de branch_from_thought: deve apontar para pensamento existente.
        if let Some(target) = t.branch_from_thought {
            if target < 1 || (target as usize) > self.main_thread.len() {
                return Err(CognitiveError::OrphanBranch(format!(
                    "branch_from_thought={target}"
                )));
            }
        }

        // 4) Validação de branch_id único por sessão.
        if let Some(ref bid) = t.branch_id {
            if self.branches.contains_key(bid) {
                return Err(CognitiveError::OrphanBranch(format!(
                    "branch_id duplicado: {bid}"
                )));
            }
        }

        // 5) Disjuntor FinOps: teto rígido de overthinking.
        let next_idx = self.main_thread.len() as u32 + 1;
        if next_idx > self.hard_limit {
            return Err(CognitiveError::OverthinkingThresholdBreached {
                actual: next_idx,
                max: self.hard_limit,
            });
        }

        // 6) Commit no thread principal ou branch.
        let mode = t.mode();
        let thought_number = t.thought_number;
        if let Some(ref bid) = t.branch_id {
            self.branches
                .entry(bid.clone())
                .or_default()
                .push(thought_number);
        }
        self.main_thread.push(t);

        // 7) Emite resposta canônica.
        let branches: Vec<BranchSummary> = self
            .branches
            .iter()
            .map(|(k, v)| BranchSummary {
                branch_id: k.clone(),
                thought_count: v.len(),
            })
            .collect();

        Ok(ThinkingResponse {
            thought_number,
            total_thoughts: self.hard_limit,
            next_thought_needed: next_idx < self.hard_limit
                && self
                    .main_thread
                    .last()
                    .map(|t| t.next_thought_needed)
                    .unwrap_or(false),
            branches,
            mode,
        })
    }

    /// Retorna os pensamentos da thread principal.
    pub fn main_thread(&self) -> &[ThoughtData] {
        &self.main_thread
    }

    /// Retorna as ramificações de pensamentos.
    pub fn branches(&self) -> &HashMap<BranchId, Vec<ThoughtId>> {
        &self.branches
    }

    /// Identificador único da sessão (UUIDv7 prefixado).
    pub fn session_id(&self) -> &str {
        &self.session_id
    }

    /// Retorna se o HITL autorizou a expansão do limite.
    pub fn is_hitl_authorized(&self) -> bool {
        self.hitl_authorized
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn regular(n: u32, last: bool) -> ThoughtData {
        ThoughtData {
            thought: format!("pensamento socrático #{n}"),
            thought_number: n,
            total_thoughts: DEFAULT_HARD_LIMIT,
            next_thought_needed: !last,
            is_revision: None,
            revises_thought: None,
            branch_from_thought: None,
            branch_id: None,
            needs_more_thoughts: None,
            hitl_authorized: None,
        }
    }

    #[test]
    fn test_thinking_disjuntor_loop() {
        let mut engine = ThinkingEngine::new();
        for i in 1..=DEFAULT_HARD_LIMIT {
            let r = engine.push_thought(regular(i, i == DEFAULT_HARD_LIMIT));
            assert!(
                r.is_ok(),
                "pensamento #{i} deve ser aceito dentro do limite {DEFAULT_HARD_LIMIT}"
            );
        }
        assert_eq!(engine.thought_count(), DEFAULT_HARD_LIMIT);
        assert_eq!(engine.current_limit(), DEFAULT_HARD_LIMIT);

        // 6º pensamento sem HITL DEVE ser abortado pelo disjuntor.
        let r = engine.push_thought(regular(6, false));
        assert!(r.is_err());
        let err = r.unwrap_err();
        assert!(
            matches!(err, CognitiveError::OverthinkingThresholdBreached { actual: 6, max: 5 }),
            "esperado OverthinkingThresholdBreached {{ actual: 6, max: 5 }}, obteve: {err:?}"
        );
    }

    #[test]
    fn test_revision_validation() {
        let mut engine = ThinkingEngine::new();
        let _ = engine.push_thought(regular(1, false));

        // Criar pensamento Revision sem apontar revises_thought deve disparar RevisionWithoutTarget
        let mut t2 = regular(2, false);
        t2.is_revision = Some(true);
        t2.revises_thought = None;
        let r = engine.push_thought(t2);
        assert!(r.is_err());
        assert!(
            matches!(r.unwrap_err(), CognitiveError::RevisionWithoutTarget),
            "esperado RevisionWithoutTarget"
        );
    }

    #[test]
    fn test_dynamic_paradigm_selection() {
        let mut engine = ThinkingEngine::new();
        assert_eq!(engine.paradigm(), ThinkingParadigm::LinearCoT);
        assert_eq!(engine.max_search_depth(), 5);

        // Cenário 1: Tarefa de baixa complexidade (impact_score <= 3.0, failures <= 1) -> LinearCoT
        let ctx_simple = ThinkingContext {
            file_path: "src/utils.rs".to_string(),
            impact_score: 1.2,
            consecutive_failures: 0,
            hitl_authorized: false,
        };
        assert_eq!(ThinkingEngine::determine_paradigm(&ctx_simple), ThinkingParadigm::LinearCoT);
        engine.set_paradigm(ThinkingEngine::determine_paradigm(&ctx_simple));
        assert_eq!(engine.paradigm(), ThinkingParadigm::LinearCoT);
        assert_eq!(engine.max_search_depth(), 5);

        // Cenário 2: Erros repetidos no Ralph Loop (consecutive_failures > 1) -> TreeToT
        let ctx_failures = ThinkingContext {
            file_path: "src/core.rs".to_string(),
            impact_score: 1.5,
            consecutive_failures: 2,
            hitl_authorized: false,
        };
        assert_eq!(ThinkingEngine::determine_paradigm(&ctx_failures), ThinkingParadigm::TreeToT);
        engine.set_paradigm(ThinkingEngine::determine_paradigm(&ctx_failures));
        assert_eq!(engine.paradigm(), ThinkingParadigm::TreeToT);
        assert_eq!(engine.max_search_depth(), 7);

        // Cenário 3: Alto raio de impacto (impact_score > 3.0) -> CouncilMAD
        let ctx_critical = ThinkingContext {
            file_path: "src/infra/security.rs".to_string(),
            impact_score: 4.5,
            consecutive_failures: 0,
            hitl_authorized: true,
        };
        assert_eq!(ThinkingEngine::determine_paradigm(&ctx_critical), ThinkingParadigm::CouncilMAD);
        let engine_critical = ThinkingEngine::with_context(&ctx_critical);
        assert_eq!(engine_critical.paradigm(), ThinkingParadigm::CouncilMAD);
        assert_eq!(engine_critical.max_search_depth(), 7);
        assert!(engine_critical.is_hitl_authorized());
    }
}
