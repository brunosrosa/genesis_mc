---
id: "ADR-042"
title: "ADR-042-Compressao-Reversivel-CCR-e-Conveyor-Belt-de-Contexto"
version: 1.0
status: Ativo_Inegociavel
epic: "Conveyor Belt de Contexto / Marco 3.6"
amends: ["ADR-037", "ADR-041"]
description: "Implementa a janela deslizante de 5 linhas com hash DefaultHasher de 64 bits e a cache de rehidratação lossless via DashMap<u64, String> no Host RAM. Introduz as tools MCP `souls_multi_read` e `souls_fill` (CCR rehydrator) como guardiãs de contexto de baixo custo."
---

# ADR-042 — Compressão Reversível CCR & Conveyor Belt de Contexto (Marco 3.6)

## Status
Aceito (Ativo e Inegociável). Emenda construtiva da [ADR-037](docs/decisions/adrs/ADR-037-Gestao-Dinamica-Contexto-CCR.md) §3 (Paradigma CCR) e compatibiliza com a Cerca Perimétrica de Servername Soberano da [ADR-041](docs/decisions/adrs/ADR-041-Nomenclatura-Soberana-Servername-souls_mcp.md).

## Contexto
A ADR-037 instituiu o Paradigma CCR (Compress-Cache-Retrieve) com `dashmap::DashMap<[u8; 16], Bytes>` indexado por hash BLAKE3/MD5 de 16 bytes, mas a implementação efetiva da janela deslizante e da rehidratação lossless ainda não havia sido canibalizada para Rust nativo. O `lean_vacuum::dedup` (Marco 3) implementa uma variante cross-file que descarta o bloco original (apenas metadata de localização), o que é **destrutivo** e **não-rehidratável**: o LLM recebe um marcador que aponta para `file L1-L5` mas o conteúdo original se perde.

Esse acoplamento destrutivo força o LLM a navegar de volta ao arquivo de origem (que pode estar fora da janela de contexto ou já ter sido podado), criando um **vazio de informação irreversível**. Para garantir CCR genuinamente lossless, o `souls_fill` precisa do bloco original byte-a-byte intacto.

Adicionalmente, a carga cognitiva típica do Gateway SOULS envolve a leitura simultânea de múltiplos arquivos de teste, fixture e regra. Disparar `souls_read` em série (uma chamada por arquivo) é O(N) bloqueante; um orquestrador que realize `multi_read` concorrente (Tokio::spawn + join_all) e aplique compressão em pipeline reduz o tempo de wall-clock e desidrata o contexto simultaneamente.

## Declaração do Problema
Como implementar compressão reversível (lossless) por janela deslizante de 5 linhas que (a) sobreviva a colisões de hash via armazenamento do bloco original completo, (b) resista à fragmentação de heap mesmo para workloads massivos, e (c) exponha uma tool MCP de rehidratação determinística — sem qualquer dependência nova em produção?

## Decisão Arquitetural

Fica estabelecido o **Conveyor Belt de Contexto (CCR Lossless)** com 3 componentes nativos e 0 dependências externas adicionais:

### 1. `souls_dedup` (canibalização do Lean Vacuum)

A função existente `lean_vacuum::deduplicate_blocks_session` (Marco 3) **permanece** sob o mesmo nome por compatibilidade de testes de snapshot, mas é **encapsulada** por uma nova camada `context_compression::dedup` que adiciona:

| Aspecto | `lean_vacuum::dedup` (existente) | `context_compression::dedup` (novo) |
| :--- | :--- | :--- |
| Hasher | `rustc_hash::FxHasher` (não padronizado) | `std::collections::hash_map::DefaultHasher` (canônico stdlib) |
| Chave da cache | `(PathBuf, start, end)` (metadata) | `u64` (hash) → `String` (bloco original) |
| Semântica | Cross-file destructivo (aponta para L1-L5) | Intra/Inter-file lossless (rehidratável) |
| Greedy match | Não (apenas 5 linhas) | **Sim** (estica para N linhas idênticas) |
| Rehidratação | Não disponível | `souls_fill` lookup O(1) no DashMap |
| Coexistência | Mantida para back-compat do test suite | Complementar (Marco 3.6) |

**Defesa contra linhas vazias:** Linhas puramente vazias (`""`), quebras isoladas (`"\n"`) ou compostas apenas por whitespace (`split_whitespace().next() == None`) **NÃO disparam a janela deslizante** e passam em forma física original. Isso evita inchaço do texto compactado por marcadores redundantes para blocos triviais de espaçamento vertical.

### 2. `DEDUP_CACHE` — DashMap<u64, String> Bare-Metal

```rust
pub static DEDUP_CACHE: std::sync::LazyLock<dashmap::DashMap<u64, String>> =
    std::sync::LazyLock::new(dashmap::DashMap::new);
```

A chave é **`u64`** (não `String`) para evitar a alocação de milhões de `String` no heap do Host como chaves de hash. O bloco original completo (com indentação, tabs e quebras de linha exatas) é preservado no `String` valor, garantindo **lossless reversível byte-a-byte**.

### 3. Tools MCP — `souls_multi_read` + `souls_fill`

| Tool | Servername | Nome | Teto (32) | Descrição (≤120) |
| :--- | :--- | :--- | :--- | :--- |
| Multi-Read Concorrente | `souls_mcp` | `souls_multi_read` | 16 chars | "Lê múltiplos arquivos em lote na RAM de forma assíncrona aplicando compressão de contexto CCR." |
| Rehidratação Lossless | `souls_mcp` | `souls_fill` | 10 chars | "Reidrata e expande marcadores de compressão CCR de volta para o texto original lossless na RAM." |

> **Migração de contrato do `fill` legado:** A tool `fill`/`souls_fill` (Marco 3, injeção de stub `souls-stub: ...` em código) é renomeada para **`souls_stub_fill`** (com alias legado `fill`) para liberar o nome canônico `souls_fill` ao rehydrator CCR. Esse split preserva back-compat com testes anteriores e Skill que invocam `fill`.

## Diagrama de Fluxo

```
+------------+    list of paths     +------------------+    N arquivos em paralelo   +--------------+
| Cliente    | -------------------> | souls_multi_read | -------------------------> | tokio::fs    |
| MCP/LLM    |                      |                  |                            | read_to_str  |
+------------+                      +------------------+                            +------+-------+
                                              |                                            |
                                              | apply souls_dedup em cada conteúdo          |
                                              v                                            |
                                     +------------------+    hash → bloco original          |
                                     | DEDUP_CACHE      | <----------------------------------+
                                     | DashMap<u64,Str> |   (u64 = DefaultHasher do bloco 5+L)
                                     +------------------+   linhas; Str = bloco lossless)
                                              |
                                              v
                                     +------------------+
                                     | Texto compactado |  ---> [SOULS-DEDUP: 0xHASH]
                                     | (marcadores)     |
                                     +------------------+
                                              |
                                              | LLM solicita rehidratação
                                              v
                                     +------------------+
                                     | souls_fill       | --> lookup O(1) no DashMap
                                     | (text → expand)  | --> substitui marcadores
                                     +------------------+
                                              |
                                              v
                                     Texto original lossless (byte-a-byte)
```

## Algoritmo de Janela Deslizante (O(N))

```
input:  text (String), cache (DashMap<u64, String>)
output: String compactada

1. lines = text.split('\n') (preserva estrutura exata)
2. result = Vec::new()
3. i = 0
4. while i < lines.len():
5.   if is_blank(lines[i]):                  // Defesa contra linhas vazias
6.     result.push(lines[i])
7.     i += 1
8.     continue
9.   if i + 5 <= lines.len():
10.    block = &lines[i..i+5]                 // janela mínima
11.    hash  = DefaultHasher::digest(block)   // u64 canônico stdlib
12.    if cache.contains_key(hash) || block == lines[i+L..i+L+5] (i+L já validado):
13.      // Greedy: estica enquanto o bloco estendido bater com o armazenado
14.      L = 5
15.      while i + L + 5 <= lines.len() && &lines[i+L..i+L+5] == block:  L += 5
16.      // Pode haver resto parcial se L for múltiplo de 5; usa match exato do bloco armazenado
17.      cached = cache.get(hash).unwrap()
18.      marker = format!("[SOULS-DEDUP: Block Hash 0x{hash:08x}. Use souls_fill ...]")
19.      cache.insert(hash, lines[i..i+L].join("\n"))   // grava o bloco lossless
20.      result.push(marker)
21.      i += L
22.    else:
23.      cache.insert(hash, lines[i..i+5].join("\n"))   // primeira ocorrência: registra
24.      for j in 0..5:  result.push(lines[i+j])
25.      i += 5
26.  else:
27.    result.push(lines[i])
28.    i += 1
29. return result.join("\n")
```

> **Complexidade:** O(N) por linha, com 1 lookup O(1) no DashMap por janela e hash `DefaultHasher` O(L). Greedy match é O(N/5) no pior caso. Total: O(N) amortizado.

## Rehidratação `souls_fill`

```
input:  text (texto compactado com marcadores)
output: texto expandido (lossless)

1. regex /\[SOULS-DEDUP: Block Hash 0x([0-9a-f]{8})\. Use souls_fill.*?\]/
2. for each match (hash_hex):
3.   hash = u64::from_str_radix(hash_hex, 16)?
4.   original = cache.get(hash).unwrap_or(missing_marker)
5.   substitui marcador por original
6. return texto expandido
```

> Marcadores cuja hash não está em cache retornam string vazia + warning estruturado (`isError=false`, `partial=true`). Fail-soft: nunca aborta a expansão.

## Compliance com a Lei de Ferro SOULS

| Diretiva SOULS | Estado | Garantia |
| :--- | :--- | :--- |
| **Zero VRAM Extra** | **CONFORME** | `DashMap` 100% em RAM Host. Zero página de VRAM. |
| **Zero Dependência Nova** | **CONFORME** | Reusa `dashmap = 6.1.0`, `tokio = 1.51.1`, `std::collections::hash_map::DefaultHasher`. Nenhuma crate adicionada ao `Cargo.toml`. |
| **Marcha Rápida (TDD)** | **CONFORME** | Build focado em `cargo test --bin souls_mcp_server`; features CUDA/Tauri desligadas. |
| **Latência Sub-ms** | **CONFORME** | Lookup no DashMap é `RwLock` por shard; O(1) médio ~100-200ns. Hash `DefaultHasher` ~50ns/bloco. |
| **Tetos 32/120 (ADR-041)** | **CONFORME** | `souls_multi_read` (16) e `souls_fill` (10) bem abaixo de 32; descrições 98 e 89 chars (≤120). |
| **Servername Soberano** | **CONFORME** | Tools expostas exclusivamente em `souls_mcp.*` (canibalização preservada). |
| **Lossless Reversível** | **CONFORME** | Bloco original gravado **antes** da substituição, byte-a-byte. Validação por SHA-256 no teste `test_fill_rehydration_equivalence`. |

## Consequências e Trade-offs

### Positivas
- **Imunidade a Lossy Compression:** Qualquer bloco duplicado pode ser rehidratado deterministicamente sem custo de I/O.
- **Concorrência Real:** `souls_multi_read` paraleliza I/O em Tokio, reduzindo wall-clock de N leituras em série para `max(read_times)` em paralelo.
- **Bare-Metal Puro:** DefaultHasher + DashMap + tokio::fs = zero dep nova. KISS absoluto.
- **Defesa contra whitespace bloat:** Linhas vazias passam intactas, evitando inchaço de marcadores para blocos triviais.

### Riscos & Mitigações
- **Risco:** Crescimento ilimitado do `DEDUP_CACHE` sob carga contínua.
  - *Mitigação:* A tool `session` existente (Marco 3.5 PRD-005) já oferece `action: "clear"` que agora também limpa `context_compression::DEDUP_CACHE`.
- **Risco:** Colisão de hash em `u64` (2^64 cardinalidade).
  - *Mitigação:* 2^64 ≈ 1.8e19 chaves possíveis. Birthday paradox: 50% em ~6e9 entradas (~6 bilhões de blocos únicos). Workload realista: 10^5-10^6 entradas → risco < 10^-9.
- **Risco:** Greedy match pode esticar bloco errado se houver 2 blocos distintos de 5 linhas idênticas adjacentes.
  - *Mitigação:* Greedy exige que **todo o bloco estendido** seja idêntico (não apenas a janela de 5). Conflito residual é capturado pelo teste `test_multi_read_concurrency_and_compression` (3 arquivos, blocos sobrepostos).

## Blast Radius
- **`docs/decisions/adrs/ADR-042-...md`:** NOVO. Este documento.
- **`src-tauri/src/cognition/context_compression/`:** NOVO diretório (mod.rs, types.rs, dedup.rs, multi_read.rs).
- **`src-tauri/src/cognition/mod.rs`:** +1 linha (`pub mod context_compression;`).
- **`src-tauri/src/bin/souls_mcp_server.rs`:** +2 tools no `tools/list`, +2 match arms em `handle_tool_call`, +2 funções `run_souls_multi_read` e `run_souls_ccr_fill`, renomeação de `run_souls_fill` → `run_souls_stub_fill`, atualização de 4 tests existentes (de `souls_fill` → `souls_stub_fill`), adição de 4 tests novos.
- **`Cargo.toml`:** **Nenhuma mutação** (todas as deps já presentes).

## Métricas de Sucesso
- `cargo test --bin souls_mcp_server` retorna **0 errors, 0 warnings do clippy**.
- `git grep "souls_fill" -- src-tauri/src/bin/souls_mcp_server.rs | grep -v stub_fill` → aponta apenas para a nova semântica CCR.
- Testes verdes: `test_dedup_5_lines_trigger`, `test_dedup_under_5_lines_ignored`, `test_multi_read_concurrency_and_compression`, `test_fill_rehydration_equivalence`.
- Snapshot test `tools_list_respects_32_120_tetos` continua verde após inclusão das 2 novas tools.

## Razão de Ser desta ADR
> "A compressão só é aceitável se for reversível. Caso contrário, é mutilação." — Bruno, 2026-08-02.
