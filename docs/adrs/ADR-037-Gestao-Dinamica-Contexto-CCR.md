---
id: "ADR-037"
title: "ADR-037-Gestao-Dinamica-Contexto-CCR"
version: 1.1
status: Ativo_Inegociavel
epic: "Infraestrutura"
description: "Mitigação de Context Rot e OOM no Gateway Rust via Gestão Dinâmica de Orçamento de Contexto, Poda Semântica Determinística em Rust (tree-sitter/AST), Desidratação Semântica Ativa (souls_compress_memory, souls_dedup, souls_fill) e Compressão Reversível CCR (DashMap Zero-VRAM)."
---

# ADR-037: Gestão Dinâmica de Contexto e Compressão Reversível (CCR)

## Status

Aceito (Ativo, Inegociável e Fundacional para SOULS V4)

## Contexto Técnico e Gargalo de Context Rot / OOM

Durante a execução de fluxos de alta intensidade cognitiva — como *RAG Temporal*, *Deep Research* e *Enxames Agênticos* —, o Gateway Rust do ecossistema SOULS lida com payloads massivos no histórico de conversa ($T_{\text{hist}}$). O acúmulo desordenado de logs, saídas JSON de ferramentas, diffs de código e prospecções textuais longas induz a dois vetores críticos de falha no sistema:

1. **Context Rot (Degradação Semântica):** A diluição da atenção do modelo de linguagem primário em janelas extensas (> 32k-128k tokens), degradando a capacidade de raciocínio e aderência às instruções do *System Prompt*.
2. **Exaustão de VRAM e Latência (OOM Risk):** Na infraestrutura bare-metal delimitada à dGPU NVIDIA RTX 2060m ($6.0 \text{ GB}$ VRAM), contextos excessivamente longos causam explosão no tamanho das matrizes da *KV Cache*, arriscando acionamentos de OOM (*Out of Memory*) ou forçando paginação PCIe para a RAM com perda drástica de *throughput* (tokens/segundo).

O projeto open-source `headroomlabs-ai/headroom` demonstrou uma abordagem promissora para mitigação dinâmica desse problema. No entanto, a implementação original baseia-se em rackeries Python 3.10+, inferência de redes neurais via PyTorch/ModernBERT (`Kompress-v2-base`), bindings PyO3 e persistência SQLite/HNSW. Essa stack introduz **disputa de VRAM** (de 500 MB a 2.5 GB alocados pelo PyTorch/CUDA pool na dGPU), overhead de Garbage Collection (GC/GIL) e latências não determinísticas ($15 \text{ ms}$ a $316 \text{ ms}$ por pedido).

Para garantir os princípios da **Lei de Ferro SOULS Bare-Metal** (**Zero VRAM extra**, **Zero-Runtime Python/Node em produção** e latência sub-milissegundo), a heurística de orçamentação e triagem do Headroom deve ser transmutada para algoritmos determinísticos $O(N)$ nativos em Rust executados estritamente na RAM do Host.

## Declaração do Problema

Como realizar a gestão dinâmica e contínua da janela de contexto no Gateway Tokio do SOULS — comprimindo e podando histórico redundante de ferramentas, logs e código sem perda irreversível de informação — mantendo **Zero consumo de VRAM adicional na GPU**, latência de triagem $< 1.0 \text{ ms}$ e operando com interceção *loopback* local ultrarrápida?

## Decisão Arquitetural

Fica estabelecido o padrão **Gestão Dinâmica de Contexto e Compressão Reversível (CCR)**, incorporado nativamente ao Gateway Tokio Rust do SOULS. O motor opera sob **4 Leis Inegociáveis**:

```
+-----------------------------------------------------------------------------------+
|                              GATEWAY TOKIO (RUST)                                 |
|                                                                                   |
|  [ INCOMING REQUEST ]                                                             |
|           |                                                                       |
|           v                                                                       |
|  +-----------------------------------------------------------------------------+  |
|  | LEI 1: Medição de Orçamento & Triggering ($H_{in} = C_{max} - B_{out} - \delta$)|  |
|  +-----------------------------------------------------------------------------+  |
|           |                                                                       |
|     (Trigger Active?)                                                             |
|     /           \                                                                 |
|   (Sim)        (Não)                                                              |
|   /               \                                                               |
|  v                 v                                                              |
|  +------------------------------+     +----------------------------------------+  |
|  | LEI 2: Poda Semântica        |     | Bypass Direct (Zero Overhead)          |  |
|  | Determinística (souls-router) |     +----------------------------------------+  |
|  | - SmartCrusher (serded/simd) |                         |                       |
|  | - CodeCompressor (tree-sitter|                         |                       |
|  | - LogCompressor (heuristics) |                         |                       |
|  +------------------------------+                         |                       |
|           |                                               |                       |
|           v                                               |                       |
|  +----------------------------------------------------+   |                       |
|  | LEI 3: Paradigma CCR (Compress-Cache-Retrieve)     |   |                       |
|  | - Grava Payload Bruto no `DashMap` (Host RAM)      |   |                       |
|  | - Injeta Marcador `[hash=X]` e Tool `headroom_ret` |   |                       |
|  +----------------------------------------------------+   |                       |
|           |                                               |                       |
|           +-----------------------+-----------------------+                       |
|                                   |                                               |
|                                   v                                               |
|               [ DESPACHO LLM (KV Cache Aligned) ]                                 |
|                                   |                                               |
|                   (LLM chama `headroom_retrieve`)                                 |
|                                   |                                               |
|                                   v                                               |
|  +-----------------------------------------------------------------------------+  |
|  | LEI 4: Interceção Tool Loopback (< 1ms / Zero-VRAM / Host RAM Lookup)       |  |
|  +-----------------------------------------------------------------------------+  |
+-----------------------------------------------------------------------------------+
```

---

### Lei 1: A Matemática do Orçamento de Contexto

A capacidade total da janela do modelo alvo é delimitada por $C_{\text{max}}$. O orçamento máximo de entrada permitido para a carga de mensagens ($H_{\text{in}}$ - Headroom de Entrada) é calculado deterministicamente pela equação:

$$H_{\text{in}} = C_{\text{max}} - B_{\text{out}} - \delta_{\text{safe}}$$

Onde:
- $B_{\text{out}}$ é a reserva mandatória para a geração da resposta do LLM (`output_buffer_tokens`), com piso fixado entre $4.000$ e $8.000$ tokens.
- $\delta_{\text{safe}}$ é a margem de segurança determinística (fixada em $512$ tokens) destinada a absorver flutuações de contagem no tokenizador.

O volume total de tokens $T_{\text{total}}$ é particionado em 4 zonas operacionais estritas:

$$T_{\text{total}} = T_{\text{sys}} + T_{\text{tools}} + T_{\text{hist}} + T_{\text{live}}$$

#### Matriz de Preservação e Prioridades de Zona:

| Zona | Componente | Diretiva de Preservação | Regra de Poda |
| :--- | :--- | :--- | :--- |
| $T_{\text{sys}}$ | System Prompt & Instruções Globais | **Proteção Absoluta (Imutável)** | Poda proibida. Apenas estabilização via `CacheAligner`. |
| $T_{\text{tools}}$| Schemas de Ferramentas (Tool Declarations) | **Proteção Estrutural** | Poda proibida. Apenas deduplicação de schemas redundantes. |
| $T_{\text{live}}$ | Janela Ativa Recente ($k$ turnos, ex: $k=3$) | **Preservação de Curto Prazo** | Isento de compressão para garantir coerência no turno atual. |
| $T_{\text{hist}}$ | Histórico de Ferramentas, Logs e Mensagens Antigas | **Alvo Primário de Compressão** | Poda determinística agressiva até satisfazer a meta $\Delta R$. |

A decisão booleana de acionamento do pipeline de poda é calculada a cada chamada HTTP do Gateway:

$$\text{Trigger} = \begin{cases} 1, & \text{se } T_{\text{total}} > H_{\text{in}} \text{ ou modo } \text{optimize} = \text{true} \\ 0, & \text{caso contrário} \end{cases}$$

Se $\text{Trigger} = 1$, a meta exata de redução de tokens $\Delta R$ a ser eliminada da zona $T_{\text{hist}}$ é:

$$\Delta R = T_{\text{total}} - H_{\text{in}}$$

---

### Lei 2: Poda Semântica Determinística em Rust e Garras de Desidratação Ativa

É terminantemente **PROIBIDO** o uso de modelos baseados em redes neurais (PyTorch, ONNX, ModernBERT, HuggingFace) ou chamadas de runtime Python para classificação e poda de texto no Gateway.

A compressão de $T_{\text{hist}}$ é realizada por motores determinísticos $O(N)$ em memória RAM Host, reforçada pelas **Garras de Desidratação Semântica Ativa**:

1. **'souls_compress_memory':** Algoritmo de poda semântica que desidrata manuais e arquivos de regras (`.cursorrules` / `CLAUDE.md`) reduzindo a prosa a linguagem telegráfica, preservando intactos caminhos de arquivo, tipos e blocos de código.
2. **'souls_dedup':** Motor de varredura que analisa referências cruzadas e deduplica trechos compartilhados entre múltiplos arquivos antes de enviá-los ao contexto do LLM, economizando até 40% de massa textual.
3. **'souls_fill':** Mecanismo de preenchimento dinâmico de contexto que ajusta cirurgicamente a taxa de compressão de cada arquivo de forma individual com base nas restrições físicas de VRAM e no orçamento de tokens de API.
4. **Roteamento de Conteúdo por SWAR/AVX2 (`souls-router`):** Inspeção rápida dos primeiros 64 bytes do buffer usando instruções SIMD/AVX2 para categorizar o payload sem parsing integral:
   - JSON (`[` ou `{`) $\rightarrow$ Rota `SmartCrusher`.
   - Logs/ANSI (`[ERROR]`, `WARN`, `stacktrace`) $\rightarrow$ Rota `LogCompressor`.
   - Código Fonte (`fn`, `pub struct`, `def`, `class`, `import`) $\rightarrow$ Rota `CodeCompressor` (AST).
   - Prosa/Texto Livre $\rightarrow$ Rota `Pruning` Determinístico por Frases/Parágrafos.
5. **Poda Sintática de Código (`CodeCompressor` via `tree-sitter`):**
   - Utilização nativa do crate Rust `tree-sitter`.
   - Nó da AST correspondentes a declarações (`FunctionDeclarations`, `ImportDeclarations`, `StructDefinitions`) são preservados.
   - Blocos de implementação interna (`body`, `block`) têm os limites de bytes substituídos em tempo de varredura por stubs estáticos (`/* ... corpo omitido via CCR ... */`).
6. **Poda Estrutural JSON (`SmartCrusher` via `simd-json` / `serde_json`):**
   - Estratégia de divisão em 3 zonas ($K$-Split): preserva o cabeçalho ($K_{\text{head}}$), o rodapé ($K_{\text{tail}}$) e condensa elementos intermediários de baixa variância em um nó estatístico explicativo.
7. **Alocação Arena e Zero-Copy (`bumpalo` + `Cow<'a, str>`):**
   - Estruturas de mensagem reutilizam ponteiros da camada de transporte (`&'a str`). Fatias modificadas utilizam alocação em Arena de vida curta (`bumpalo`), garantindo desalocação em tempo constante $O(1)$ ao término do despacho.

---

### Lei 3: Paradigma CCR (Compress-Cache-Retrieve) 100% Host RAM (`DashMap` Zero-VRAM)

Para erradicar o risco de perda irreversível de contexto técnico, a poda aplica o protocolo **CCR**:

1. **Hash e Armazenamento no Host:** Quando um segmento de $T_{\text{hist}}$ de tamanho $S_{\text{orig}}$ é comprimido para $S_{\text{comp}}$, o buffer original é persistido no Host RAM em um mapa concorrente `dashmap::DashMap<[u8; 16], Bytes>`, indexado pelo hash MD5/BLAKE3 ($16$ bytes) do conteúdo original.
2. **Zero-VRAM Guarantee:** A cache reside 100% na memória RAM principal do computador (CPU Host). Nenhuma página de memória é reservada ou alocada na VRAM da GPU.
3. **Política de Despejo LRU:** A cache opera sob limite configurável de RAM (ex: 256 MB) com política de despejo LRU (*Least Recently Used*). Sem banco de dados em disco (Zero-SQLite overhead no caminho crítico).
4. **Injeção de Marcadores Semânticos:** O texto comprimido inserido no prompt do LLM contém o marcador de resgate:
   
   `[SOULS CCR: 150 linhas comprimidas. Para recuperar os dados integrais brutos, invoque a ferramenta headroom_retrieve(hash="a1b2c3d4e5f6")]`

---

### Lei 4: Tool Loopback Interceptado Localmente em < 1ms pelo Gateway Tokio

O Gateway Tokio injeta de forma transparente no payload da requisição a ferramenta padrão do sistema:

```json
{
  "name": "headroom_retrieve",
  "description": "Recupera o bloco original de código, logs ou JSON comprimido pelo Gateway SOULS CCR.",
  "parameters": {
    "type": "object",
    "properties": {
      "hash": { "type": "string", "description": "Hash de 16-bytes do bloco a ser restaurado" }
    },
    "required": ["hash"]
  }
}
```

#### Regra de Interceção Loopback:

1. Se o modelo de linguagem responder emitindo uma chamada à ferramenta `headroom_retrieve(hash="...")`, o Gateway Tokio **NÃO** repassa essa chamada para a aplicação nem para o cliente.
2. O Gateway intercepta a chamada na camada HTTP proxy, busca a chave correspondente no `DashMap` em RAM Host em $< 100 \ \mu\text{s}$ (microssegundos), injeta o payload original no histórico e re-despacha a chamada imediatamente para o LLM.
3. Todo o ciclo de interceção e resposta de loopback é concluído em latência total **$< 1.0 \text{ ms}$**, sem intervenção de IO de disco ou consumo de ciclos de GPU.

---

## Consequências e Trade-offs

### Impactos Positivos:

- **Imunidade a OOM na VRAM:** Garantia matemática de $0 \text{ MB}$ de VRAM consumidos pelo motor de compressão, deixando 100% da dGPU RTX 2060m disponível para tensores do modelo primário.
- **Erradicação do Context Rot:** Históricos massivos de chamadas de ferramentas e logs são condensados em stubs de alta densidade informativa, preservando a atenção do LLM nas instruções críticas.
- **KV Cache Alignment (Economia de Custo/Latência):** O módulo `CacheAligner` em Rust estabiliza os prefixos dinâmicos do System Prompt (ex: timestamps), aumentando a taxa de *KV Cache Hit* no provedor de inferência para até $> 90\%$.
- **Desempenho Bare-Metal:** Latência de triagem $< 1.0 \text{ ms}$ (vs. $15\text{ms}-316\text{ms}$ no Headroom Python original).

### Riscos e Mitigações:

- **Risco:** Consumo de RAM no Host com acúmulo de chaves no `DashMap`.
  - *Mitigação:* Limite rígido de alocação de memória (ex: cap de 256MB) com despejo atômico LRU gerido em tempo de execução.
- **Risco:** Loops de chamadas excessivas à ferramenta `headroom_retrieve` pelo LLM se a compressão for agressiva demais.
  - *Mitigação:* Preservação mandatória da zona $T_{\text{live}}$ ($k=3$ a $5$ turnos) e calibração fina da meta de redução $\Delta R$.

---

## Compliance com a Lei de Ferro SOULS

| Diretiva SOULS | Estado do ADR-037 | Garantia de Implementação |
| :--- | :--- | :--- |
| **Zero VRAM Extra** | **CONFORME** | 100% alocado na RAM Host via `DashMap` concorrente. |
| **No Python/Node Runtime** | **CONFORME** | Transmutado inteiramente para Rust nativo (`tree-sitter`, `simd-json`, `bumpalo`). |
| **SDD / Spec-Driven** | **CONFORME** | Este ADR oficializa a física de gestão de contexto antes de qualquer código/PRD. |
| **Latência Sub-milissegundo**| **CONFORME** | Interceção loopback $< 100 \ \mu\text{s}$ no Tokio, total $< 1.0 \text{ ms}$. |
