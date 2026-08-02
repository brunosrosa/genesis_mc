//! Máquina de estados socrática (PRD-032 §3) com disjuntor cognitivo 5 → 7 (HITL).
//!
//! Tríade: `Regular | Revision | Branching`.
//! Disjuntor: `DEFAULT_HARD_LIMIT = 5`; `HITL_EXTENDED_LIMIT = 7` sob
//! `hitl_authorized: true` no payload.
//! Validação de branching: `is_revision=true ⇒ revises_thought.is_some()`.
//! Erros: `CognitiveError::OverthinkingThresholdBreached`,
//! `CognitiveError::RevisionWithoutTarget`, `CognitiveError::OrphanBranch`.

use std::collections::HashMap;

use crate::cognition::memory_graph::errors::CognitiveError;
use crate::cognition::thinking::types::{
    BranchId, BranchSummary, ThoughtData, ThoughtId, ThinkingResponse,
};

/// Teto absoluto padrão do disjuntor.
pub const DEFAULT_HARD_LIMIT: u32 = 5;

/// Teto elástico sob autorização HITL explícita do Arquiteto.
pub const HITL_EXTENDED_LIMIT: u32 = 7;

/// Estado in-RAM de uma sessão socrática.
///
/// HashMap<BranchId, Vec<ThoughtId>> alocada temporariamente no contexto do
/// laço da tarefa, limpando-se o heap imediatamente no teardown do subagente
/// (PRD-032 §3.1).
pub struct ThinkingEngine {
    session_id: String,
    main_thread: Vec<ThoughtData>,
    branches: HashMap<BranchId, Vec<ThoughtId>>,
    hard_limit: u32,
    hitl_authorized: bool,
}

impl Default for ThinkingEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl ThinkingEngine {
    /// Cria uma sessão socrática com o teto padrão (5 pensamentos).
    pub fn new() -> Self {
        Self {
            session_id: format!("thn_{}", uuid::Uuid::new_v4().simple()),
            main_thread: Vec::with_capacity(DEFAULT_HARD_LIMIT as usize),
            branches: HashMap::new(),
            hard_limit: DEFAULT_HARD_LIMIT,
            hitl_authorized: false,
        }
    }

    /// Ativa a flag HITL (estica teto para 7). Idempotente.
    pub fn authorize_hitl(&mut self) {
        self.hitl_authorized = true;
        self.hard_limit = HITL_EXTENDED_LIMIT;
    }

    /// Teto atual (5 ou 7).
    pub fn current_limit(&self) -> u32 {
        self.hard_limit
    }

    /// Contagem de pensamentos já consumidos.
    pub fn thought_count(&self) -> u32 {
        self.main_thread.len() as u32
    }

    /// Sessão de pensamento está fechada? (`next_thought_needed == false`
    /// no último pensamento registrado, ou teto estourado).
    pub fn is_closed(&self) -> bool {
        self.main_thread
            .last()
            .map(|t| !t.next_thought_needed)
            .unwrap_or(false)
            || self.thought_count() >= self.hard_limit
    }

    /// Registra um novo pensamento. Aplica validação de branching e
    /// disjuntor FinOps. Aborta com `CognitiveError` se violar qualquer
    /// lei do PRD-032.
    pub fn push_thought(&mut self, t: ThoughtData) -> Result<ThinkingResponse, CognitiveError> {
        // 0) HITL late-binding: se o pensamento atual pede autorização, estica o teto.
        if t.hitl_authorized.unwrap_or(false) {
            self.authorize_hitl();
        }

        // 1) Validação de branching: revision ⇒ revises_thought obrigatório.
        if t.is_revision.unwrap_or(false) && t.revises_thought.is_none() {
            return Err(CognitiveError::RevisionWithoutTarget);
        }

        // 2) Validação de referência: revision ⇒ revises_thought deve existir.
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

        // 4) Validação de branch_id único por sessão (se informado, não pode
        //    existir previamente; branches são imutáveis após commit).
        if let Some(ref bid) = t.branch_id {
            if self.branches.contains_key(bid) {
                return Err(CognitiveError::OrphanBranch(format!(
                    "branch_id duplicado: {bid}"
                )));
            }
        }

        // 5) Disjuntor FinOps: teto rígido.
        let next_idx = self.main_thread.len() as u32 + 1;
        if next_idx > self.hard_limit {
            return Err(CognitiveError::OverthinkingThresholdBreached {
                actual: next_idx,
                max: self.hard_limit,
            });
        }

        // 6) Commit: arquiva no thread principal OU em branch.
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

    /// Acesso somente-leitura aos pensamentos da thread principal.
    pub fn main_thread(&self) -> &[ThoughtData] {
        &self.main_thread
    }

    /// Acesso somente-leitura ao mapa de ramificações.
    pub fn branches(&self) -> &HashMap<BranchId, Vec<ThoughtId>> {
        &self.branches
    }

    /// Sessão de pensamento em uso (telemetria / debug).
    pub fn session_id(&self) -> &str {
        &self.session_id
    }

    /// Flag HITL ativa?
    pub fn is_hitl_authorized(&self) -> bool {
        self.hitl_authorized
    }
}

#[cfg(test)]
mod tests {
    //! TDD obrigatório do Marco 3.5 — disjuntor cognitivo.
    //! Refs: PRD-032 §3.2, ADR-040.

    use super::*;
    use crate::cognition::thinking::types::ThinkingMode;

    fn regular(n: u32, last: bool) -> ThoughtData {
        ThoughtData {
            thought: format!("pensamento regular #{n}"),
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
        // Empilha 5 pensamentos (limite padrão). Todos OK.
        for i in 1..=DEFAULT_HARD_LIMIT {
            let r = engine.push_thought(regular(i, i == DEFAULT_HARD_LIMIT));
            assert!(
                r.is_ok(),
                "pensamento #{i} deve ser aceito (limite {DEFAULT_HARD_LIMIT})"
            );
        }
        assert_eq!(
            engine.thought_count(),
            DEFAULT_HARD_LIMIT,
            "5 pensamentos consumidos"
        );
        assert_eq!(engine.current_limit(), DEFAULT_HARD_LIMIT, "teto = 5");

        // 6º pensamento DEVE ser abortado pelo disjuntor FinOps.
        let r = engine.push_thought(regular(6, false));
        assert!(r.is_err(), "6º pensamento deve ser rejeitado");
        let err = r.unwrap_err();
        assert!(
            matches!(err, CognitiveError::OverthinkingThresholdBreached { actual: 6, max: 5 }),
            "erro deve ser OverthinkingThresholdBreached {{ actual: 6, max: 5 }}, got: {err:?}"
        );
    }

    #[test]
    fn test_thinking_hitl_extension_to_7() {
        let mut engine = ThinkingEngine::new();
        // 5 pensamentos Regulares com HITL no 4º (estica para 7).
        for i in 1..=3 {
            let r = engine.push_thought(regular(i, false));
            assert!(r.is_ok(), "pensamento #{i} aceito");
        }
        // 4º pensamento com hitl_authorized: true → estica teto.
        let mut t4 = regular(4, false);
        t4.hitl_authorized = Some(true);
        let r = engine.push_thought(t4);
        assert!(r.is_ok(), "4º pensamento com HITL aceito");
        assert!(engine.is_hitl_authorized(), "HITL flag ativa");
        assert_eq!(engine.current_limit(), HITL_EXTENDED_LIMIT, "teto = 7");

        // 5º, 6º, 7º pensamentos (todos dentro do teto HITL).
        for i in 5..=HITL_EXTENDED_LIMIT {
            let r = engine.push_thought(regular(i, i == HITL_EXTENDED_LIMIT));
            assert!(
                r.is_ok(),
                "pensamento #{i} deve ser aceito (teto HITL {HITL_EXTENDED_LIMIT})"
            );
        }
        assert_eq!(engine.thought_count(), HITL_EXTENDED_LIMIT, "7 pensamentos consumidos");

        // 8º pensamento DEVE ser abortado.
        let r = engine.push_thought(regular(8, false));
        assert!(r.is_err(), "8º pensamento deve ser rejeitado");
        let err = r.unwrap_err();
        assert!(
            matches!(err, CognitiveError::OverthinkingThresholdBreached { actual: 8, max: 7 }),
            "erro deve ser OverthinkingThresholdBreached {{ actual: 8, max: 7 }}, got: {err:?}"
        );
    }

    #[test]
    fn test_thinking_revision_without_target() {
        let mut engine = ThinkingEngine::new();
        // 1º pensamento regular (OK).
        let _ = engine.push_thought(regular(1, false));

        // 2º pensamento: is_revision=true MAS sem revises_thought.
        let mut t2 = regular(2, false);
        t2.is_revision = Some(true);
        t2.revises_thought = None;
        let r = engine.push_thought(t2);
        assert!(r.is_err(), "revision sem target deve falhar");
        assert!(
            matches!(r.unwrap_err(), CognitiveError::RevisionWithoutTarget),
            "erro deve ser RevisionWithoutTarget"
        );
    }

    #[test]
    fn test_thinking_branching_routing() {
        let mut engine = ThinkingEngine::new();
        // Pensamento Regular.
        let r = engine.push_thought(regular(1, false)).unwrap();
        assert_eq!(r.mode, ThinkingMode::Regular);

        // Branch a partir do pensamento 1.
        let mut t2 = regular(2, false);
        t2.branch_from_thought = Some(1);
        t2.branch_id = Some("branch-A-zero-copy".to_string());
        let r = engine.push_thought(t2).unwrap();
        assert_eq!(r.mode, ThinkingMode::Branching);

        // Revision do pensamento 1.
        let mut t3 = regular(3, false);
        t3.is_revision = Some(true);
        t3.revises_thought = Some(1);
        let r = engine.push_thought(t3).unwrap();
        assert_eq!(r.mode, ThinkingMode::Revision);

        // Confere registro de branch.
        assert_eq!(engine.branches().get("branch-A-zero-copy").unwrap().len(), 1);
    }
}
