---
aliases:
  - "Especificação Técnica: Arquitetura de SLMs"
  - Harness e Protocolo Peer-Review
---
# SPEC-013: Governança de SLMs, Exosqueleto (Harness), Taxonomia e Protocolo Peer-Review

**Status:** Aprovado / Emenda de Arquitetura
**Escopo:** Souls MC (SODA V6) — Engine de Cognição e Orquestração Local
**Alvo de Hardware:** Intel i9 | 32GB RAM | RTX 2060m (6GB VRAM GDDR6)
**Stack de Execução:** Rust (Tokio) + Wasmtime + FrankenSQLite (L2) + Constrained Decoding (`llguidance`)

## 1. Visão Geral e Propósito

Este documento consolida a arquitetura de **otimização de Small Language Models (SLMs)** e o protocolo de **auditoria cruzada (Peer-Review)** no Souls MC (SODA V6).

O objetivo é maximizar a inteligência útil e a precisão de execução de modelos compactos de borda (1B a 8B parâmetros) enjaulados em um **Harness Bare-Metal em Rust**, definindo os limites operacionais, o ciclo de vida de auditoria por pares e a estratégia de transbordamento (Spill-over) para nuvem/modelos pesados.

## 2. O Papel do Harness e a Elevação de Capacidade das SLMs

Modelos de linguagem sem suporte externo gastam até 50% de sua capacidade computacional e atenção tentando manter formatação sintática, delimitar blocos de código ou gerenciar estados.

```
+-------------------------------------------------------------------------+
|                         SISTEMA SEM HARNESS                             |
|  SLM (4B/7B) = [Atenção Sintática (40%)] + [Raciocínio Útil (60%)]      |
+-------------------------------------------------------------------------+

+-------------------------------------------------------------------------+
|                     SISTEMA SOULS MC (COM HARNESS)                      |
|  Harness Rust (llguidance / Tree-Sitter) = Trava Sintática O(1)         |
|  SLM (4B/7B) = [Raciocínio Útil, Lógica e Síntese (100%)]                |
+-------------------------------------------------------------------------+
```

### Componentes da Armadura (Exosqueleto Bare-Metal):

1. **Constrained Decoding (`llguidance`):** Interceptação e mascaramento de logits no nível da GPU/Engine de inferência. A SLM é fisicamente impedida de emitir qualquer token fora da gramática estrita (JSON/YAML/LEAN).
2. **Poda Sintática Pré-LLM (`lean_vacuum`):** Sanitização de ruído sintático, comentários mortos e espaços em branco na CPU host (AVX2) antes do envio dos dados à GPU.
3. **Ancoramento Estruturado (AST Parsers via `tree-sitter`):** Resolução exata de símbolos e escopos sem exigir que a SLM memorize a estrutura de arquivos.

## 3. Destilação de Oráculos e Transferência Cognitiva

Para manter alta densidade de inteligência no limite térmico de 6GB VRAM, o Souls MC adota modelos locais derivados do processo de **Destilação de Oráculos (Teacher-Student)**.

### Pipeline de Destilação do Conhecimento:

- **Cadeias de Raciocínio Sintético (CoT Distillation):** Modelos de Fronteira / Massivos (2.3T+ como Kimi K3 / DeepSeek-V3) resolvem problemas complexos e geram marcas de pensamento detalhadas.
- **Supervised Fine-Tuning (SFT) & GRPO:** A SLM local (4B–8B) é treinada diretamente sobre as cadeias de raciocínio purificadas do modelo gigante.
- **Divergência KL (Logit Matching):** A SLM aprende a copiar a distribuição de probabilidades do oráculo, retendo estruturas de raciocínio profundo em vez de fatos encyclopédicos estáticos.

## 4. Taxonomia Unificada de Modelos do Ecossistema

|   |   |   |   |   |
|---|---|---|---|---|
|**Sigla**|**Categoria**|**Faixa de Parâmetros**|**Hardware Target**|**Função Primária no Souls MC**|
|**NLU / Micro-Model**|Encoders (Task-Specific)|< 500M|CPU (AVX2/NEON)|Extração $O(1)$, NER, re-ranking e triagem (ex.: GliClass).|
|**SLM**|Small Language Model|1B a 8B|GPU Local (4–8GB VRAM) / RAM|**Live Chat, REPL local, uso de Tools e raciocínio de borda.**|
|**Mid-LLM / I-LLM**|Intermediate LLM|14B a 70B|Workstations / Unified RAM|Processamento local denso e análise profunda offline.|
|**Frontier / Massive**|Massive Cloud LLM|100B a 2.3T+|Data Center / Cloud|Oráculos externos, destilação sintética e _spill-over_.|

## 5. Protocolo Peer-Review: O Triângulo ACE (Actor-Critic-Environment)

A auditoria entre modelos aumenta a taxa de precisão de 10% a 25%, desde que intermediada por um **verificador determinístico** e contida dentro do limite de convergência.

```
                       ┌─────────────────────────┐
                       │      WORKER (SLM)       │
                       │   (Gera Rascunho v1)    │
                       └────────────┬────────────┘
                                    │
                                    ▼
                       ┌─────────────────────────┐
                       │  ENVIRONMENT HARNESS    │
                       │ (Compilador/Linter/TDD) │
                       └────────────┬────────────┘
                                    │ (Falha/Status Frio)
                                    ▼
                       ┌─────────────────────────┐
                       │   CRITIC (Peer-Review)  │
                       │ (Sessão Isolada / Rubro)│
                       └────────────┬────────────┘
                                    │ (Feedback Direcionado)
                                    └─────────────────────► (Retorna ao Worker)
```

### Regras do Ciclo Virtuoso:

1. **O Número Mágico:** Máximo de **1 a 3 ciclos** de interação. Acima de 3 turnos, ocorre o fenômeno de **Sicofancia (Sycophancy)** e degradação por retornos decrescentes.
2. **Isolamento de Sessão (KV Cache Flush):** O papel de _Critic_ deve ser executado em uma **sessão separada** (mesmo se utilizar o mesmo modelo SLM local). Isso limpa o viés de atenção do KV Cache e garante imparcialidade na leitura do código/resposta.
3. **Ancoramento Determinístico:** O _Critic_ nunca avalia em texto livre abstrato. Ele recebe o artefato gerado mais o relatório de execução do **Environment Harness** (ex.: `Falha de compilação na linha 24`).

## 6. Mapeamento das Zonas de Dificuldade e Vacinas Arquiteturais

| **Zona de Dificuldade da SLM**        | **Manifestação do Limite**                                      | **Vacina / Tratamento no Souls MC**                                                                                |
| ------------------------------------- | --------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------ |
| **1. Inércia de Contexto Longo**      | Fenômeno _Lost in the Middle_ em prompts $> 8k$ tokens.         | **RLM / Context Slicing:** Fatiamento via Wasmtime e visão materializada ($T_{\text{state\_mv}}$) em Notação LEAN. |
| **2. Drift de Objetivo (Goal Drift)** | Perda do rumo da tarefa em planos multi-etapas.                 | **Orquestração por DAG/FSM em Rust:** A SLM executa apenas sub-tarefas atômicas guiadas por máquina de estados.    |
| **3. Entropia de Formatação**         | Quebra de sintaxe JSON/YAML ou injeção de conversa em payloads. | **Constrained Decoding (`llguidance`):** Máscara de logits no motor de inferência.                                 |
| **4. Amnésia de Fatos Específicos**   | Alucinação de APIs, bibliotecas ou regras obsoletas.            | **Grounding por Banco Relacional (L2/L3):** Injeção de snippets do SQLite (FTS5) / LanceDB antes do prompt.        |
| **5. Ilusão de Sucesso em Código**    | Geração de código sintaticamente bonito que não compila.        | **Environment Harness (TDD/Linters):** Validação física em sandbox antes de aceitar o artefato.                    |

## 7. Estratégia FinOps de Transbordamento (Spill-Over)

Para equilibrar custo, soberania e tempo de execução, o roteamento de tarefas adota três camadas:

1. **Camada Hot (Local Immediate):** Loop síncrono local (Worker SLM + Harness + Critic SLM). Resolve ~85% das requisições em milissegundos sem custo.
2. **Camada Warm (Cloud Escalate por Exceção):** Disparada apenas se o teste local falhar após 3 tentativas. O nó com falha é empacotado e enviado a um oráculo na nuvem.
3. **Camada Cold (Batch Overnight / Background):** Tarefas massivas e não-urgentes (ex.: re-análise de repositórios do Harvester) são salvas na fila SQLite (`souls_heuristic_vault.db`) e processadas assincronamente por daemons locais.

## 8. Diretriz de Conformidade

1. Toda execução de agente local deve utilizar **Constrained Decoding** para garantir formato de saída estrito.
2. O ciclo de **Peer-Review** não deve ultrapassar 3 iterações sem intervenção determinística ou escalonação.
3. Consultas à web ou nuvem devem persistir suas sínteses no banco relacional local (**L2/L3**) para que o controle retorne imediatamente à SLM local.