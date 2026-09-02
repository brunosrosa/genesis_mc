---
aliases:
  - 'Especificação Técnica: Governança de Contexto e Motor REPL-RLM do "Master Agent"'
---
# SPEC-012: Governança de Contexto, Ingestão Zero-Friction e Motor REPL/RLM do Agente Master

**Status:** Aprovado / Emenda de Arquitetura
**Escopo:** Souls MC (SODA V6) — Engine de Cognição Local
**Alvo de Hardware:** Intel i9 | 32GB RAM | RTX 2060m (6GB VRAM GDDR6)
**Stack de Execução:** Rust (Tokio) + Wasmtime + FrankenSQLite (L2) + Svelte 5 / Overlay Canvas

## 1. Visão Geral e Propósito

Este documento cimenta o protocolo de **Governança de Contexto** e o **Ciclo de Execução REPL (Recursive Language Modeling)** do agente **Master** no Souls MC.

O objetivo é sustentar a ilusão operacional de **"Contexto Infinito"** com respostas imediatas no _live chat_, eliminando o _Context Rot_ (deterioração da atenção) e respeitando de forma inegociável o teto térmico e de memória física da RTX 2060m.

## 2. Arquitetura do Motor REPL/RLM do Master

O agente **Master** atua como o regente do sistema. Ele opera em uma sessão contínua de alto nível e delega tarefas pesadas para instâncias efêmeras isoladas.

```
+-----------------------------------------------------------------------+
|                         SESSÃO PRINCIPAL (MASTER)                      |
|  - Prompt Enxunto (T_sys + T_tools + T_state_mv + T_live_diff)        |
|  - Monitoramento contínuo de Pressão de Tokens                         |
+-----------------------------------------------------------------------+
                                   |
         +-------------------------+-------------------------+
         | (Delegar Investigação)                            | (Eventos / Respostas)
         v                                                   v
+----------------------------------+             +----------------------+
|   SUB-SESSÕES EFÊMERAS (Sub-RLM) |             |  LIVE CHAT / CANVAS  |
|  - Sandbox em Wasmtime           |             |  - Respostas Curtas  |
|  - Context Slicing atômico       |             |  - Flips de Artefato |
|  - Retorno: Reduce Sintético     |             +----------------------+
+----------------------------------+
```

### Principais Regras de Execução:

1. **Single-Session Master:** O Master mantém o estado contínuo da relação com o operador sem acumular o histórico bruto de todas as interações.
2. **Context Slicing:** Ao instanciar um **Sub-RLM** (ex.: analisar uma dependência ou varrer uma AST), o Master injeta apenas o recorte de dados estritamente necessário para aquela tarefa.
3. **Reduce Sintético:** O Sub-RLM executa de forma enjaulada e retorna apenas uma struct compacta com os achados. O log de execução interno do Sub-RLM é descartado da janela do Master.

## 3. Pipeline de Ingestão de Entrada (Zero-Friction Ingestion)

Para evitar que o usuário precise "fatiar" prompts, cortar logs ou resumir códigos manualmente antes de enviar ao chat, o Souls MC aplica um pipeline de pré-processamento **Bare-Metal pré-LLM** em tempo $O(1)$.

```
[Input do Usuário] ──> [Filtro Rust/CPU] ──> [Hash / DashMap (RAM)] ──> [Prompt Enxuto]
 (Texto Imenso / Logs)   (Poda & Vacuum)      (Ponteiro de Resgate)    (Tokens Otimizados)
```

1. **Aceitação Sem Restrição (UX Amigável):** A caixa de entrada aceita coleções massivas de código ou logs (ex.: 50k+ caracteres).
2. **Poda e Desidratação** $O(1)$**:**
    - **`lean_vacuum`:** Higieniza formatação morta, linhas vazias e ruídos sintáticos na CPU host (AVX2) antes de tocar na GPU.
    - **Abstração por Hash:** Se a entrada contiver blocos imensos de dados não-conversacionais, o payload bruto é gravado em memória RAM do host (`DashMap`) mapeado por um hash Blake3.
    - **Injeção Sintética:** No prompt da LLM entra apenas o trecho conversacional relevante e um ponteiro:
        `[ATTACHMENT_ATTACHED: hash_7f3a | Tipo: Cargo.lock | 120 linhas críticas extraídas]`
3. **Reidratação Sob Demanda:** Se o Master precisar ler o bloco inteiro, ele dispara a tool local `souls_fill(hash_7f3a)` para reidratar partes do dado.

## 4. Topologia e Particionamento da Janela de Contexto

A janela de contexto do Master é mantida rigidamente estruturada sob 4 zonas topológicas:

$$\text{Janela Total} = T_{\text{sys}} + T_{\text{tools}} + T_{\text{state\_mv}} + T_{\text{live\_diff}}$$

| **Zona**                | **Conteúdo**                                                                                             | **Natureza da Memória**                            |
| ----------------------- | -------------------------------------------------------------------------------------------------------- | -------------------------------------------------- |
| $T_{\text{sys}}$        | Diretrizes de sistema, restrições e personalidade                                                        | Estática / Imutável                                |
| $T_{\text{tools}}$      | Schemas compactos de ferramentas/MCPs ativos                                                             | Dinâmica por Modo                                  |
| $T_{\text{state\_mv}}$  | **Visão Materializada de Estado** (decisões, preferências e fatos canônicos condensados em Notação LEAN) | Atualizada Assincronamente                         |
| $T_{\text{live\_diff}}$ | Buffer das últimas $N$ mensagens da conversa atual                                                       | **Janela Curta (**$N = 4 \text{ a } 8$ **turnos)** |

## 5. Ciclo de Vida da Memória e Triggers Assíncronos

A persistência de memória opera em **duas esteiras simultâneas (Dual-Track)** para nunca desacelerar o tempo de resposta no chat ativo.

```
              Turno Concluído (Entrada / Saída)
                             │
         ┌───────────────────┴───────────────────┐
         ▼                                       ▼
 [Trilho Síncrono (Live)]             [Trilho Assíncrono (CPU)]
  - Devolve resposta ao usuário        - Extração de Fatos Chave-Valor
  - Buffer $T_{\text{live\_diff}}$ +1  - Injeção atômica no SQLite (L2)
                                       - Checa Threshold de Pressão
```

### 5.1. Trilho 1: Micro-Extração por Turno (Background)

- A cada turno ($Input \rightarrow Output$), uma task leve (`tokio::spawn`) analisa a interação e extrai **decisões e preferências explícitas** (ex.: `preferencia_porta_api: 8080`).
- O fato é gravado no banco **FrankenSQLite** (`souls_state.db` - L2) em milissegundos sem bloquear o streaming da resposta.

### 5.2. Trilho 2: Macro-Compressão por Pressão de Tokens

- **Métrica de Trigger:** A reescrita do $T_{\text{state\_mv}}$ ocorre quando a contagem de tokens de $T_{\text{live\_diff}}$ atinge **2.048 tokens** ou **80% da capacidade alvo do KV Cache**.
- **AutoDream / Chyros Daemon:** Em momento de ociosidade, o daemon consolida os turnos antigos de $T_{\text{live\_diff}}$, atualiza a Notação LEAN no $T_{\text{state\_mv}}$, e remove as mensagens antigas da janela ativa, mantendo apenas os últimos 4 a 8 turnos.

## 6. Governança de Saída (Output Intent Budgeting)

O tamanho da saída da LLM é governado ativamente com base na **Intenção do Comando**, otimizando a computação e a legibilidade da interface.

### 6.1. Fast-Path Conversacional (Respostas no Chat)

- **Orçamento:** 250 a 500 tokens.
- **Uso:** Confirmações, dúvidas diretas, alinhamentos e bate-papo.
- **UX:** Mantém a conversa fluida e com baixo tempo até o primeiro token (_Time-to-First-Token_).

### 6.2. Deep-Path / Relatórios (Roteamento para Overlay Canvas)

- **Orçamento:** Expandido dinamicamente (2.048 a 4.096+ tokens).
- **Uso:** Refatorações de código, especificações técnicas, PRDs ou relatórios analíticos longos.
- **Regra de Ouro de UX:** Respostas longas **não são ejetadas na linha do tempo do chat**. O Master emite um resumo de 2 linhas no chat e roteia o payload completo para o **Painel Lateral do Canvas (Generative UI / Artefato)**.

## 7. Contratos de Dados em Rust (TDD Base)

Para orientação de implementação no backend em Rust (`src-tauri/src/cognition/repl.rs`):

```
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Estado do Buffer Conversacional do Master
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MasterReplState {
    pub session_id: Uuid,
    pub live_turn_count: usize,
    pub current_token_pressure: usize,
    pub token_pressure_threshold: usize, // Ex: 2048
    pub live_diff_buffer: Vec<ChatMessage>,
}

/// Resultado Sintético retornado por Sub-Sessões RLM
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubRlmResult {
    pub task_id: Uuid,
    pub success: bool,
    pub synthetic_summary_lean: String,
    pub generated_artifacts: Vec<String>,
    pub tokens_consumed: usize,
}

/// Orçamento de Saída por Intenção
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IntentBudget {
    FastPathConversational { max_tokens: u16 }, // Ex: 350
    DeepPathArtifact { max_tokens: u16 },        // Ex: 4096
}
```

## 8. Conclusão e Diretriz de Conformidade

Com este protocolo:

1. O usuário tem liberdade total para colar volumes grandes de texto sem travar a interface.
2. O chat permanece leve, legível e rápido, com janelas curtas ($N = 4\text{ a }8$).
3. O conhecimento e as decisões canônicas são acumulados deterministicamente na Tríade de Memória (L2/L3).
4. O consumo de VRAM e o tempo de atenção do modelo permanecem dentro do envelope térmico do hardware local.