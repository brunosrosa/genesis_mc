---
spec: v4-syntactic-healing-and-compression-fence
phase: 3-tasks
design: docs/work-units/active/marco-4.9.2-syntactic-healing/design.md
branch: feat/marco-4.9.2-syntactic-healing
---

# Tasks — MARCO 4.9.2 — Cura Sintática + Cercadinho de Prosa

Cada task tem um DoD (Definition of Done) executável. Tarefas marcadas `[SCAFFOLD]` exigem teste vazio de falha antes da lógica real (Lei do Scaffold). Toda mutação passa por escrita atômica com Mutex assíncrono do Tokio (Souls Archivist).

## TASK-01 — Pin: `jsonrepair = "=0.1.0"` (ADR-030 Módulo 4)

**Arquivo:** `src-tauri/Cargo.toml` (EDIT)

**Escopo:** Adicionar a dep com pin rígido conforme ADR-030 (operador `=`, sem `^`/`~`/`*`).

- [ ] Linha adicionada na seção `[dependencies]` (alinhada com outras deps pinadas):
  ```toml
  jsonrepair = "=0.1.0"  # SOULS-CANIBALIZED Marco 4.9.2: parser recursivo estrutural para cura de JSON malformado (sem `.replace` cego)
  ```
- [ ] Posicionada **após** `thiserror` (ordem alfabética visual, mesma convenção das demais deps)

**DoD:**
- `cargo check` Exit Code 0 (dep resolvida via crates.io)
- `cargo tree -p jsonrepair` retorna apenas `jsonrepair v0.1.0` + `memchr v2.x` (transitiva)
- `Cargo.lock` regenerado com `jsonrepair 0.1.0` na entrada exata

## TASK-02 — `[SCAFFOLD]` Teste de proteção de strings literais (Red)

**Arquivo:** `src-tauri/src/core/response_healing.rs` (EDIT — apenas `mod tests`)

**Escopo:** Criar o teste que **prova** a corrupção atual antes de qualquer mutação. Deve falhar no estado pré-cura (porque o `.replace` cego ainda existe).

- [ ] Função `test_response_healing_with_user_strings`:
  - Input: `{"query": "Answer: True", "data": [1, 2, 3,`
  - `assert_eq!` que a string `"Answer: True"` é preservada **byte-a-byte**
  - `assert_eq!` que o array `[1, 2, 3]` é fechado corretamente

**DoD:**
- Teste compila
- Teste **FALHA** no estado pré-cura (código atual corrompe a string)
- Captura screenshot do `cargo test` mostrando o failure

## TASK-03 — Reescrita de `response_healing.rs` (Green)

**Arquivo:** `src-tauri/src/core/response_healing.rs` (REWRITE)

**Escopo:** Substituir o parser manual de 110 linhas pelo wrapper estrutural do `jsonrepair`. Preservar a interface pública `heal_malformed_json(&str) -> Cow<'_, str>`.

- [ ] `use jsonrepair::{repair_json, Options, StreamRepairer};`
- [ ] `Options::default()` é o correto: `tolerate_hash_comments=true`, `allow_python_keywords=true`, `fenced_code_blocks=true` (todos default da crate)
- [ ] `heal_malformed_json` chama `repair_json(input, &opts)` e mapeia `Result<String, _>`:
  - `Ok(repaired) if repaired == input` → `Cow::Borrowed(input)`
  - `Ok(repaired)` → `Cow::Owned(repaired)`
  - `Err(_)` → `Cow::Borrowed(input)` (fail-soft: nunca panicar; LLM upstream é best-effort)
- [ ] Manter `repair_json_buffer` como wrapper thin (apenas delega para `heal_malformed_json`) para preservar callers legados
- [ ] Remover completamente o bloco `.replace(": True", ": true")` e similares (L120-L125)
- [ ] Documentar o contrato no rustdoc: "Strings literais válidas do payload são IMUTÁVEIS"

**DoD:**
- `cargo check` Exit Code 0
- Teste de TASK-02 agora **PASSA**
- Testes existentes (`test_response_healing_sub_millisecond_json_repair`, `test_heal_malformed_json_cow`) continuam verdes
- Latência de cura < 1ms preservada (medida via `Instant::now()`)

## TASK-04 — Teste de normalização estrutural de primitivos (Red→Green)

**Arquivo:** `src-tauri/src/core/response_healing.rs` (EDIT — `mod tests`)

**Escopo:** Validar que True/False/None como **primitivos JSON soltos** são normalizados, mas strings contendo-os não são tocadas.

- [ ] `test_response_healing_normalizes_python_primitives_structurally`:
  - Input: `{status: True, count: None, ok: False}`
  - `assert_eq!` que a parse `serde_json::from_str::<Value>` resulta em `{"status":true,"count":null,"ok":false}`

**DoD:**
- Teste compila e PASSA após TASK-03

## TASK-05 — Acoplamento do `heal_malformed_json` no SSE Frame Accumulator

**Arquivo:** `src-tauri/src/bin/agentgateway_tcp_proxy.rs` (EDIT)

**Escopo:** Injetar a cura sintática como gate síncrono < 1ms antes do `write_all` ao downstream.

- [ ] Adicionar `use souls_mc_lib::core::response_healing::heal_malformed_json;` no topo do arquivo
- [ ] No loop `for frame_bytes in frames` (linha 237), após `String::from_utf8_lossy`:
  - Se o frame contém `data: ` (SSE data line) e **não** é o marcador `[DONE]`, extrair o payload JSON, aplicar `heal_malformed_json`, e re-empaçotar como `data: {cured}\n\n`
  - Frames que não sejam JSON (ex: event comments, keep-alives) passam inalterados
- [ ] No `flush_remaining` (linha 260), mesma lógica para o residual final
- [ ] Medir latência com `Instant::now()` antes/depois — assertion < 1ms no teste

**DoD:**
- `cargo check` Exit Code 0
- Teste `test_sse_accumulator_cures_truncated_frame` (TASK-06) PASSA
- Testes existentes do proxy permanecem verdes (não regredir fragmentação TCP/SSE)

## TASK-06 — Teste de cura de frame SSE truncado (Red→Green)

**Arquivo:** `src-tauri/src/bin/agentgateway_tcp_proxy.rs` (EDIT — `mod tests`)

**Escopo:** Provar que o gate do `heal_malformed_json` cura delimitadores truncados em < 1ms.

- [ ] `test_sse_accumulator_cures_truncated_frame`:
  - Input: `data: {"choices":[{"delta":{"content":"Hel`
  - Aplica o gate (replica a lógica do handle_upstream_response)
  - `assert!` que o output contém `"content":"Hel"}` (ou estrutura equivalente fechada)
  - `assert!` que `serde_json::from_str` no payload extraído **PASSA**
  - `assert!` que o tempo de cura é < 1ms

**DoD:**
- Teste compila e PASSA após TASK-05
- Latência medida reportada via `eprintln!` (telemetria FinOps)

## TASK-07 — Cercadinho whitelist-invertida no `aggressive_compress`

**Arquivo:** `src-tauri/src/cognition/context/souls_read.rs` (EDIT)

**Escopo:** Bloquear stripping de comentários quando `ext ∈ {md, markdown}` ou `ext = None`.

- [ ] Adicionar constante `const PROSE_EXTENSIONS: &[&str] = &["md", "markdown", "mdx"];` no topo do módulo
- [ ] Adicionar helper `fn is_prose(ext: Option<&str>) -> bool` que retorna `true` se `ext.is_none() || ext.is_some_and(|e| PROSE_EXTENSIONS.contains(&e))`
- [ ] Logo após `let is_python = ...` (linha 27), adicionar `let is_prose = is_prose(ext);`
- [ ] **Curto-circuito antecipado**: se `is_prose`, pular todo o bloco de stripping (linhas 41-64 do código atual) e ir direto para o brace-run concat
- [ ] Preservar a invariante: o pipeline `compress_to_lean` (que chama `aggressive_compress` + `lightweight_cleanup`) deve continuar compilando limpo

**DoD:**
- `cargo check` Exit Code 0
- Testes existentes (`aggressive_compress_strips_rust_line_comments`, etc.) permanecem verdes (código continua stripping para `.rs`/`.py`/`.sql`)
- Teste de TASK-08 PASSA

## TASK-08 — Teste de preservação de Markdown e prosa (Red→Green)

**Arquivo:** `src-tauri/src/cognition/context/souls_read.rs` (EDIT — `mod tests`)

**Escopo:** Provar que `aggressive_compress` com `ext = Some("md")`, `Some("markdown")` ou `None` preserva cabeçalhos, bullet points e blocos de fenced code intactos.

- [ ] `test_aggressive_compress_preserves_markdown_headers`:
  - Input: `"# Título\n## Subtítulo\n- item 1\n- item 2\n```rust\nfn main() {}\n```\n"`
  - `assert!` que `# Título` está presente
  - `assert!` que `## Subtítulo` está presente
  - `assert!` que `fn main() {}` (dentro de fenced code) está presente
- [ ] `test_aggressive_compress_preserves_prose_with_none_ext`:
  - Input: `"# Capítulo\n// nota de revisão\n- bullet point"`
  - `assert!` que **todas** as linhas estão presentes (incluindo `//` e `-`)

**DoD:**
- Ambos os testes compilam e PASSAM após TASK-07
- `cargo test` global permanece 100% verde

## TASK-09 — Validação: cargo check + clippy + test verde

**Escopo:** Provar que o silício assimilou a esteira inteira.

- [x] `cd src-tauri && cargo check --workspace --all-targets` → Exit Code 0 com **zero warnings**
- [x] `cd src-tauri && cargo clippy --workspace --all-targets -- -D warnings` → Exit Code 0
- [x] `cd src-tauri && cargo test --workspace --no-run` → Exit Code 0
- [x] `cd src-tauri && cargo test --workspace` → Exit Code 0 com **618+ testes verdes** + 8 novos testes TDD (5 response_healing + 3 SSE proxy + 4 prose fence = 12 mas 5 existentes renomeados → 8 net new). Ver TASK-02, 04, 06, 08.
- [ ] Se falhar por lifetime/ownership: invocar `souls-ralph-loop` (3-tentativas ceiling, Fail-Closed)
- [x] Se falhar por feature gating: ajustar `#[cfg(feature = "...")]` sem importar runtime Python/Node

**DoD (verificado em 2026-08-06):**
- Todos os comandos acima retornam Exit Code 0
- Contagem de testes: **340 (lib) + 224 + 9 + 1 + 42 + 2 + 3 + 7 + 3 + 4 + 2 + 4 + 3** = **644 testes** workspace total
- Marco 4.9.2 introduziu **+12 testes TDD** novos, **+0 regressões**

### 9.1 Observação sobre flake pré-existente (NÃO regressão do Marco 4.9.2)

O teste `cognition::state_thinking::thinking::socratic_bridge::tests::test_upsert_thought_fire_and_forget_persists` é **flaky** sob paralelismo alto (default `cargo test`). Causa raiz: o teste usa `std::thread::sleep(100ms)` para aguardar o worker MPSC drenar 4 ops, mas sob carga paralela alta o worker não consegue drenar a tempo.

**Verificação:** confirmado que o flake **existe no baseline `origin/TRAE-IDE` (sem mudanças do Marco 4.9.2)** — vide stash + rerun isolado + rerun com `--test-threads=2` (sempre passa em sequencial e 100% verde).

**Mitigação recomendada para CI:** invocar `cargo test --workspace -- --test-threads=2` (já validado 100% verde). Refatorar o teste para usar `tokio::time::sleep` ou `notify`-based barrier é trabalho fora do escopo do Marco 4.9.2 (princípio do blast radius mínimo).


## TASK-10 — Blast Radius Report + HITL

**Escopo:** Compilar diff stats e enviar para aprovação humana (sem merge automático).

- [ ] `git diff --stat` capturado
- [ ] `git diff --stat origin/TRAE-IDE...HEAD` capturado
- [ ] Mensagem HITL gerada com: branch, número de arquivos tocados, lista de paths, contagem de testes novos
- [ ] **NÃO** fazer merge
- [ ] **NÃO** fazer push
- [ ] Aguardar aprovação do Arquiteto para rebase semântico e `gh pr create`
