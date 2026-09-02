---
aliases:
  - "Especificação Técnica: Pipeline SDD-TDD e Governança HITL"
---
# SPEC-014: Pipeline SDD/TDD, Proactive Epistemic Probing e Governança HITL por Exceção

**Status:** Aprovado / Emenda de Arquitetura
**Escopo:** Souls MC (SODA V6) — Engine de Orquestração, Especificação e Execução Agêntica
**Alvo de Hardware:** Intel i9 | 32GB RAM | RTX 2060m (6GB VRAM GDDR6)
**Stack de Execução:** Rust (Tokio) + Wasmtime + FrankenSQLite (L2) + Svelte 5 / Overlay Canvas

## 1. Visão Geral e Propósito

Este documento formaliza o pipeline de desenvolvimento do Souls MC (SODA V6), estabelecendo a simbiose entre **Spec-Driven Development (SDD)** no nível macro e **Test-Driven Development (TDD)** no nível micro.

A arquitetura elimina o _Vibe Coding_ e o _Goal Drift_ em Small Language Models (SLMs) ao enjaular o fluxo em quatro pilares:

1. **Investigação Epistêmica Proativa (`epistemic_probe`):** Identificação e resolução de ambiguidades antes de gerar código.
2. **Cascata de Especificação Declarativa:** Geração de documentos atômicos (`REQUIREMENTS.md`, `DESIGN.md`, `TASKS.md`, `TEST_SPECS.md`).
3. **Governança HITL por Exceção:** Intervenção do operador humano restrita à validação de intenção no Canvas e respostas a ambiguidades irredutíveis.
4. **Execução Micro-TDD e Ralph Loop:** Validação determinística em sandbox local com escalonação para nuvem apenas após esgotamento de retentativas.

## 2. Matriz de Artefatos SDD (Macro-Pipeline)

O SDD extirpa a ambiguidade _antes_ que qualquer linha de código seja gerada. Toda iniciativa é fatiada em 4 documentos encadeados:

```
[1. REQUIREMENTS.md] ──> [2. DESIGN.md] ──> [3. TASKS.md] ──> [4. TEST_SPECS.md]
  (PRD / Negócio)        (Specs / Arq)      (Lotes DAG)        (Casos de Teste)
```

| **Documento**         | **Foco de Conteúdo**                                                                    | **Agente Gerador**         | **Validador (Peer-Review)**     |
| --------------------- | --------------------------------------------------------------------------------------- | -------------------------- | ------------------------------- |
| **`REQUIREMENTS.md`** | Escopo, regras de negócio, restrições não-funcionais e não-objetivos.                   | Master Agent + `/grill-me` | **HITL Gate #1 (Operador)**     |
| **`DESIGN.md`**       | Arquitetura técnica, schemas, structs/enums Rust, contratos de API e ADRs.              | Arch SLM / Heavy LLM       | Critic SLM / LLM (Sessão Limpa) |
| **`TASKS.md`**        | Matriz de tarefas atômicas independentes organizadas em Grafo Aclíclico Dirigido (DAG). | TaskPlanner SLM            | Critic SLM / LLM (Sessão Limpa) |
| **`TEST_SPECS.md`**   | Especificação dos cenários de teste (entradas, saídas e critérios de aceite).           | TestPlanner SLM            | Critic SLM / LLM (Sessão Limpa) |

## 3. Investigação Epistêmica Proativa (`epistemic_probe`)

Para evitar que o sistema dependa do acionamento manual do usuário, a orquestração monitora continuamente a ambiguidade da instrução em tempo $O(1)$.

```
[Input do Usuário] ──> [Métrica de Entropia O(1)] ──> Clareza >= 85% ──> [Gerar PRD Direto]
                                                  └── Clareza < 85%  ──> [Disparar Probe Proativo]
```

### 3.1. Mapeamento Agnóstico de Aliases de Interface

No backend Rust, a Máquina de Estados Finita (FSM) é denominada **`soda::epistemic_probe`**. O frontend aceita de forma transparente qualquer alias comum de mercado:

- `/grill-me`, `/grill` (Padrão Cursor / Roo Code / BMAD)
- `/interview`, `/probe` (Padrão Spec-Driven)
- `/plan`, `/architect`, `/spec` (Padrão Claude Code / Windsurf)
- `/clarify`, `/ask` (Padrão Aider)

### 3.2. Deduplicação por Memória Canônica (L2/L3)

Antes de formular qualquer pergunta ao operador, o `epistemic_probe` executa uma busca na Memória Canônica (**FrankenSQLite - `souls_state.db`** / **LanceDB**). Se a dúvida for resolvida por uma preferência já cadastrada (ex.: _"Operador prefere SQLite a PostgreSQL"_), a pergunta é **assimilada em silêncio**, informando apenas a decisão assumida.

## 4. Governança HITL por Exceção (HITL Gates)

O operador humano atua estritamente como **Árbitro de Intenção**, sem necessidade de redigir especificações ou inspecionar código intermediário.

```
 ┌────────────────────────────────────────────────────────────────────────┐
 │ HITL GATE #1: ASSINATURA DE PRD (CANVAS OVERLAY)                       │
 ├────────────────────────────────────────────────────────────────────────┤
 │ 1. O Master conclui o /grill-me e gera o REQUIREMENTS.md.              │
 │ 2. O documento é projetado no Painel Canvas com resumo de 3 linhas.   │
 │ 3. Operador aprova (Sign-off) ou injeta ajuste curto.                  │
 └────────────────────────────────────────────────────────────────────────┘
                                    │ (Aprovado)
                                    ▼
 ┌────────────────────────────────────────────────────────────────────────┐
 │ CASCATA AUTOMÁTICA DE SDD + PEER-REVIEW (DESIGN, TASKS, TEST_SPECS)    │
 └────────────────────────────────────────────────────────────────────────┘
                                    │ (Se detectar ambiguidade de negócio)
                                    ▼
 ┌────────────────────────────────────────────────────────────────────────┐
 │ HITL GATE #2: EXCEÇÃO DE AMBIGUIDADE (WAITING_HUMAN_CLARIFICATION)     │
 ├────────────────────────────────────────────────────────────────────────┤
 │ 1. O Peer-Review (Critic) detecta dúvida de negócio no DESIGN.md.      │
 │ 2. Emite Card de Pergunta Objetiva no Canvas.                          │
 │ 3. Operador responde com 1 clique/palavra e libera a esteira.          │
 └────────────────────────────────────────────────────────────────────────┘
                                    │
                                    ▼
 ┌────────────────────────────────────────────────────────────────────────┐
 │ HANDOVER REPORT: DOSSIÊ DE LIBERAÇÃO PARA O WORKER (TDD)               │
 └────────────────────────────────────────────────────────────────────────┘
```

## 5. Execução Micro-TDD e o Ralph Loop (Runtime)

Uma vez liberado o Dossiê de Handover, a fase de escrita de código é assumida pelo **Worker SLM (4B–8B)** operando sob controle estrito do **Environment Harness** (compiladores, linters e runners de teste).

```
                        ┌────────────────────────┐
                        │   TAREFA N (TASKS.MD)   │
                        └───────────┬────────────┘
                                    │
                                    ▼
                        ┌────────────────────────┐
                        │   WORKER ESCREVE TESTE │
                        │      (Fase RED)        │
                        └───────────┬────────────┘
                                    │
                                    ▼
                        ┌────────────────────────┐
                        │ WORKER ESCREVE CÓDIGO  │
                        │     (Fase GREEN)       │
                        └───────────┬────────────┘
                                    │
                                    ▼
                        ┌────────────────────────┐
                        │   ENVIRONMENT HARNESS  │
                        │ (Compilador / Runner)  │
                        └───────────┬────────────┘
                                    │
                     ┌──────────────┴──────────────┐
                     │                             │
             [Passou (GREEN)]             [Falhou (RED)]
                     │                             │
                     ▼                             ▼
           [Executa Refactor]             [Dispara Ralph Loop]
                     │                    (Max 3 Retentativas)
                     ▼                             │
            [Próxima Tarefa]             (Se esgotar 3 tentativas)
                                                   │
                                                   ▼
                                         [Spill-Over para Cloud]
```

### 5.1. O Ralph Loop (Retentativas Locais)

1. **Feedback Frio:** O erro retornado do terminal (ex.: `cargo test` ou `clippy`) é injetado diretamente no prompt do Worker SLM.
2. **Limite Rígido:** Teto de **1 a 3 retentativas locais**.
3. **Prevenção de Ciclos:** Se o erro persistir na 3ª tentativa, o ciclo é suspenso para evitar desperdício térmico e _Context Rot_.

### 5.2. Escalonação FinOps (Spill-Over por Exceção)

Ao estourar o Ralph Loop local:

1. O estado da tarefa + o log de erro do terminal são empacotados em um payload desidratado.
2. O roteador **ParetoBandit** envia a requisição para uma Heavy LLM na nuvem (ex.: DeepSeek / Claude).
3. A Heavy LLM resolve o ponto cego específico e retorna a correção.
4. O controle **retorna imediatamente para a SLM local**, que prossegue na lista do `TASKS.md`.

## 6. Captura de Memória Dual-Rate (Tempo Real vs. AutoDream)

A assimilação de decisões e preferências do operador opera em duas velocidades para nunca gerar latência perceptível no chat ativo:

```
[Decisão no HITL / Chat Pause] ──┬──> Tempo Real (< 10ms) ──> Injeção L2 SQLite + Update T_state_mv
                                 └──> Pausas (Idle Time)  ──> Reconciliação de Vetores via Tokio
                                 └──> Noturno (02:00)     ──> AutoDream Daemon (Langevin Decay + LLM-Wiki)
```

1. **Captura em Tempo Real (< 10ms):** Toda escolha feita em portões HITL ou pausas no _live chat_ é gravada de forma síncrona na tabela de preferências do **FrankenSQLite** (`souls_state.db`) e refletida na Visão Materializada ($T_{\text{state\_mv}}$) da próxima mensagem.
2. **Processamento em Pausas (Idle Time):** Durante momentos de inatividade de digitação do usuário, a runtime Rust (`tokio::spawn`) executa a reconciliação de vetores e resolve conflitos leves de contexto.
3. **AutoDream Noturno (Chyros Daemon):** Executado em momentos de ociosidade profunda (ex.: 02:00 AM) para aplicar a equação de decaimento de Langevin, re-clusterizar o banco vetorial **LanceDB**, atualizar o grafo no **LadybugDB** e reorganizar as páginas Markdown da **LLM-Wiki**.

## 7. Contratos de Dados em Rust (TDD Base)

Para implementação do motor de especificações no backend (`src-tauri/src/orchestrator/sdd.rs`):

```
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Estado da FSM de Investigação Epistêmica
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EpistemicProbeState {
    pub session_id: Uuid,
    pub clarity_score: f32, // 0.0 a 1.0 (Threshold: >= 0.85)
    pub active_alias_used: String, // ex: "/grill-me"
    pub pending_questions: Vec<ProbeQuestion>,
    pub assimilated_preferences: Vec<String>,
}

/// Pergunta Socrática do Probe
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProbeQuestion {
    pub id: usize,
    pub question_text: String,
    pub options: Option<Vec<String>>,
    pub is_resolved_by_memory: bool,
}

/// Estado do Pipeline SDD e Portões HITL
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SddPipelineStage {
    EpistemicProbing,
    WaitingHitlPrdSignoff { prd_path: String },
    GeneratingCascade { current_doc: String },
    WaitingHitlAmbiguityException { ambiguity_details: String },
    HandoverReady { dossier_id: Uuid },
    ExecutingMicroTdd { active_task_id: String, retry_count: u8 },
}

/// Registro de Débito Técnico e Handover
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HandoverDossier {
    pub dossier_id: Uuid,
    pub requirements_ref: String,
    pub design_ref: String,
    pub tasks_ref: String,
    pub test_specs_ref: String,
    pub technical_debts: Vec<String>,
    pub trade_offs_accepted: Vec<String>,
}
```

## 8. Diretriz de Conformidade

1. Nenhuma execução de código em lote deve ser iniciada sem o aceite do **`REQUIREMENTS.md` (HITL Gate #1)**.
2. A validação de documentos intermediários (`DESIGN.md`, `TASKS.md`, `TEST_SPECS.md`) deve ser feita via **Peer-Review por Critic em sessão isolada**.
3. Todo erro de execução deve passar pelo **Ralph Loop local (máximo 3 tentativas)** antes de acionar a escalonação para nuvem.
4. Preferências e decisões do operador devem ser persistidas na **Memória Canônica L2 (< 10ms)** e refletidas no $T_{\text{state\_mv}}$ sem aguardar rotinas noturnas.