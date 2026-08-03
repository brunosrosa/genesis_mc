# PRD-10.3: Specification for Context Headroom & CCR (Compress-Cache-Retrieve) Gateway Engine

**Status:** Especificação Proposta (Aguardando Aprovação HITL)  
**Módulo:** Gateway Tokio HTTP/Proxy (`souls-router`, `souls-ccr` / `headroom_engine.rs`)  
**Target Hardware:** NVIDIA RTX 2060 Mobile (6.0 GB VRAM) + Intel i9 (Host RAM)  
**Arquitetura:** Rust Nativo (Tokio Async Runtime, Zero-Copy `Cow<'a, str>`, `tree-sitter` AST, `simd-json`, `dashmap::DashMap`)  
**Rastreabilidade ADR:** [ADR-037](file:///z:/souls_mc/docs/decisions/adrs/ADR-037-Gestao-Dinamica-Contexto-CCR.md)

---

## 1. Objetivo Atômico

O **PRD-10.3** especifica a infraestrutura bare-metal de gestão dinâmica da janela de contexto para mitigar **Context Rot** e prevenir falhas de **Out-Of-Memory (OOM)** na dGPU NVIDIA RTX 2060m (6.0 GB VRAM) durante execuções de *RAG Temporal*, *Deep Research* e *Enxames Agênticos*.

O objetivo é implementar no Gateway Tokio Rust do SOULS:
1. **Disjuntor de Tokens e Orçamentação ($H_{\text{in}}$):** Avaliação determinística pré-despacho antes da fase de prefill da KV Cache.
2. **Poda Semântica Zero-Copy (`souls-router`):** Compressão de ASTs de código via `tree-sitter`, resumos JSON via `simd-json` e heurísticas de logs.
3. **Compressão Reversível CCR Zero-VRAM (`souls-ccr`):** Armazenamento do histórico original bruto 100% na RAM do Host (Intel i9) via `DashMap` concorrente e interceção *loopback* local da tool `headroom_retrieve(hash)` em latência $< 1.0 \text{ ms}$.

---

## 2. Rastreabilidade Arquitetural

Este PRD deriva diretamente das Leis Inegociáveis estabelecidas no [ADR-037: Gestão Dinâmica de Contexto e Compressão Reversível (CCR)](file:///z:/souls_mc/docs/decisions/adrs/ADR-037-Gestao-Dinamica-Contexto-CCR.md).

- **Princípio Bare-Metal:** Proibição absoluta de modelos PyTorch, ONNX, ModernBERT, wrappers Python (PyO3) ou persistência SQLite no caminho crítico de mediação do Gateway HTTP.
- **Zero VRAM Footprint:** A totalidade dos buffers de resgate e tabelas de hash reside na memória RAM principal do computador (CPU Host).

---

## 3. Arquitetura de Execução

```
[ INCOMING REQUEST ]
        |
        v
+-------------------------------------------------------------------------------+
| 3.1 ORÇAMENTAÇÃO & DISJUNTOR ($H_{\text{in}}$)                               |
| $H_{\text{in}} = C_{\text{max}} - B_{\text{out}} - \delta_{\text{safe}}$       |
| $T_{\text{total}} = T_{\text{sys}} + T_{\text{tools}} + T_{\text{hist}} + T_{\text{live}}$|
+-------------------------------------------------------------------------------+
        |
  (Trigger = $T_{\text{total}} > H_{\text{in}}$?)
  /             \
(Sim)          (Não)
  /               \
 v                 v
+-----------------------+     +-------------------------------------------------+
| 3.2 PODA ZERO-COPY    |     | BYPASS DIRECT                                   |
| (tree-sitter / AST)   |     +-------------------------------------------------+
+-----------------------+                             |
        |                                             |
        v                                             |
+-------------------------------------------------+   |
| 3.3 REGISTRO CCR (Host RAM)                     |   |
| `DashMap<[u8; 16], Bytes>` (Zero-VRAM)          |   |
| Injeta Tool `headroom_retrieve` & Marcador Hash |   |
+-------------------------------------------------+   |
        |                                             |
        +-----------------------+---------------------+
                                |
                                v
                [ DESPACHO LLM / PREFILL ]
                                |
            (LLM Invoca `headroom_retrieve(hash)`)
                                |
                                v
+-------------------------------------------------------------------------------+
| 3.4 TOOL LOOPBACK LOCAL (< 1ms)                                              |
| Intercepta no Gateway Tokio | Busca no `DashMap` | Injeta Payload Bruto      |
+-------------------------------------------------------------------------------+
```

### 3.1 Fórmula do Orçamento ($H_{\text{in}}$) Pré-Despacho

Antes de transacionar o payload HTTP para o backend de inferência, o Gateway calcula o orçamento disponível:

$$H_{\text{in}} = C_{\text{max}} - B_{\text{out}} - \delta_{\text{safe}}$$

- $C_{\text{max}}$: Limite máximo de contexto do modelo (ex: $128.000$ tokens).
- $B_{\text{out}}$: Reserva mandatória para a resposta do LLM (`output_buffer_tokens`, padrão $4.096$ a $8.192$ tokens).
- $\delta_{\text{safe}}$: Margem de segurança determinística para flutuações de contagem no tokenizador (fixado em $512$ tokens).

Se $T_{\text{total}} > H_{\text{in}}$, o disjuntor aciona a poda da zona $T_{\text{hist}}$ para eliminar $\Delta R = T_{\text{total}} - H_{\text{in}}$ tokens. As zonas $T_{\text{sys}}$ (System Prompt) e $T_{\text{tools}}$ permanecem protegidas e imutáveis.

### 3.2 Poda AST Zero-Copy com `tree-sitter` (`CodeCompressor`)

Para saídas contendo blocos de código fonte:
1. O Gateway analisa o código com o parser sintático nativo do `tree-sitter`.
2. As assinaturas de funções, declarações de tipos e importações são preservadas.
3. O corpo das funções (`body` / `block`) é colapsado e substituído diretamente nos slices do buffer pela fatia de bytes estática `b"{ /* stubbed */ }"`, operando sobre abstrações `Cow<'a, str>` e arenas `bumpalo` para garantir alocação zero no Heap dinâmico intermediate.

### 3.3 Tabela Hash `DashMap` em Host RAM (Intel i9)

O mapa de armazenamento CCR é ancorado estritamente na memória RAM central do sistema Host:

```rust
pub struct SoulsCcrStore {
    // Chave: Hash MD5/BLAKE3 (16 bytes), Valor: Payload bruto original contíguo
    cache: Arc<dashmap::DashMap<[u8; 16], bytes::Bytes>>,
    max_ram_bytes: usize,
}
```

- **Zero-VRAM:** Nenhuma alocação ocorre nos tensores de VRAM da dGPU.
- **LRU Eviction:** Despejo atômico em RAM Host caso a ocupação atinja o limite máximo configurado (padrão 256 MB).

### 3.4 Tool Loopback Local em $< 1.0 \text{ ms}$

O Gateway injeta o schema da ferramenta `headroom_retrieve` na chamada ao LLM.
- Quando o LLM requisita a restauração de um bloco emitindo `headroom_retrieve(hash="a1b2c3d4...")`, o Gateway Tokio **intercepta a chamada localmente**.
- O payload é recuperado do `DashMap` em RAM Host em $< 100 \ \mu\text{s}$.
- O histórico é hidratado e o fluxo prossegue no Gateway **sem qualquer roundtrip de rede externo** e em latência total **$< 1.0 \text{ ms}$**.

---

## 4. Definition of Done (DoD - Testes de Bloqueio)

Para que a fase de implementação TDD (Red-Green-Refactor) seja homologada, a suíte de testes de integração e unitários do Rust DEVE passar obrigatoriamente sem falhas:

### 4.1 Teste de Alocação de Memória RAM Host (`test_ccr_dashmap_allocation_host_ram`)
* **Módulo:** `souls-ccr` / `headroom_engine::tests`
* **Objetivo:** Inserir 1.000 payloads de código e JSON no `DashMap` do CCR Store e validar via inspeção de métricas que a alocação de memória ocorre estritamente na RAM do Host (CPU), garantindo **0 MB de consumo ou alocação adicional na VRAM** da dGPU.

### 4.2 Teste de Poda AST Zero-Copy (`test_ast_code_compressor_zero_copy`)
* **Módulo:** `souls-router` / `code_compressor::tests`
* **Objetivo:** Submeter um arquivo de código Rust/TypeScript de 5.000 linhas ao `CodeCompressor` via `tree-sitter`. O teste deve comprovar que o corpo das funções foi substituído por `b"{ /* stubbed */ }"` e que a operação de reescrita foi realizada sobre um buffer `Cow::Borrowed` / `bumpalo::Bump` com zero alocações intermediárias no Heap dinâmico.

### 4.3 Teste de Matemática do Orçamento (`test_headroom_math_budget`)
* **Módulo:** `souls-router` / `budget_evaluator::tests`
* **Objetivo:** Validar a precisão da equação $H_{\text{in}} = C_{\text{max}} - B_{\text{out}} - \delta_{\text{safe}}$. O teste deve simular requisições de $10.000$ a $200.000$ tokens com variações de $\delta_{\text{safe}} = 512$ e $B_{\text{out}} = 4.096$, garantindo que a trigger booleana $\text{Trigger} = 1$ é acionada exatamente no limiar esperado e calcula a meta de poda $\Delta R$ sem estouros de arredondamento.

### 4.4 Higiene de Compilação & Linter Rígido
* **DoD:** Compilação limpa com Exit Code 0 sob aviso zero:
  ```bash
  cargo check --all-targets --features gateway_ccr -D warnings
  cargo clippy --all-targets --features gateway_ccr -D warnings
  ```

---

## 5. Conclusão & Alinhamento HITL

Este artefato especifica formalmente o plano de execução do **PRD-10.3**. Nenhum código de produção foi escrito nesta etapa.
