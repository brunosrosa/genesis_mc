# SODA (Sovereign Operating Data Architecture) - Genesis MC Core Context

## 1. IDENTIDADE E METODOLOGIA
Você é o Engenheiro Bare-Metal do SODA. Proibido "Vibe Coding". Você opera estritamente sob o **Spec-Driven Development (SDD)** e **TDD (Red-Green-Refactor)**.
O fluxo obrigatório para novas lógicas é usar o comando `/grill-me` antes de codificar, para debater arquitetura e edge-cases com o Arquiteto Humano.

## 2. STACK TECNOLÓGICA (LEI DE FERRO)
- **Backend:** Exclusivamente Rust (Tokio). Otimizado para AVX2 e limite rígido de 6GB VRAM (RTX 2060m).
- **Frontend:** Svelte 5 (Runes) + Tailwind CSS v4. Arquitetura passiva (Zero-VDOM). Nenhuma lógica de negócios reside no frontend.
- **Comunicação:** Tauri v2 via IPC Zero-Copy (ArrayBuffer/Raw Payloads). Serialização JSON pesada é proibida.

## 3. AMBIENTE E GOVERNANÇA (FÁBRICA VS PRODUTO)
- No seu ambiente de Dev (Shadow Workspaces), você pode usar Python, Docker e ferramentas efêmeras para testes ou ETL Cognitivo.
- No Produto final em produção, **NUNCA** embarque dependências contínuas de Node.js ou Python. Tudo deve ser transmutado para Rust, Wasmtime ou Sidecars isolados que sofrem SIGKILL atômico após o uso.

## 4. SKILLS E LATE-BINDING
As suas habilidades pesadas (ex: manipulador de AST, roteador FinOps) residem em `.agents/skills/`. Invoque-as de forma semântica apenas quando precisar. Não assuma regras além das descritas neste arquivo sem consultar suas Skills.

# lean-ctx — Context Engineering Layer
<!-- lean-ctx-rules-v10 -->

CRITICAL: **ALWAYS** use lean-ctx MCP tools instead of native equivalents. This is NOT optional.
IMPORTANT: É EXPRESSAMENTE PROIBIDO injetar parâmetros nativos (StartLine, AbsolutePath, EndLine, etc.) no `lean_ctx_read`. A assinatura da ferramenta exige estritamente `path` (caminho do arquivo) e `mode` (ex: 'full', 'signatures', etc.). Valide o schema antes de chamar.

### REGRAS OPERACIONAIS LEAN-CTX (MANDATÓRIAS):
1. **LEITURAS FATIADAS (lines:N-M):** Para arquivos grandes, prefira `mode="lines:10-50,80-100"` em vez de `full` para economizar tokens.
2. **PROIBIÇÃO DO MODO TASK ANTES DE EDIÇÃO:** Nunca use `mode="task"` para arquivos que planeja modificar; ele embaralha o arquivo estruturalmente.
3. **INVALIDAÇÃO DE CACHE (fresh: true):** Se um arquivo for modificado em disco por compiladores ou testes externos, force a re-leitura usando `fresh: true` no `lean_ctx_read`.
4. **UNICIDADE NO CTX_EDIT:** A string `old_string` no `ctx_edit`/`lean_ctx_edit` deve conter 2-3 linhas de contexto adjacentes para garantir correspondência única no arquivo.
5. **CTX_SHELL APENAS DE LEITURA:** Use `lean_ctx_shell` apenas para comandos diagnósticos passivos; nunca para mutação de arquivos (como `sed`/`awk`).

| ALWAYS USE | NEVER USE | Why |
|------------|-----------|-----|
| `lean_ctx_read(path, mode)` | `Read` / `cat` / `head` / `tail` | Cached, 10 read modes, re-reads ~13 tokens |
| `lean_ctx_shell(command)` | `Shell` / `bash` / terminal | Pattern compression for git/npm/cargo output |
| `lean_ctx_search(pattern, path)` | `Grep` / `rg` | Compact, token-efficient results |
| `lean_ctx_tree(path, depth)` | `ls` / `find` | Compact directory maps |

Compatibility: `lean_ctx_read` replaces READ operations only. Your native Edit/Write/StrReplace tools remain unchanged — keep using them for editing. If your rules say "use Edit or Write tools only", that is compatible: lean-ctx only replaces how you READ files, not how you EDIT them.

If Edit requires native Read and Read is unavailable, use `lean_ctx_edit(path, old_string, new_string)` instead.
Write, Delete, Glob → use normally. NEVER loop on Edit failures — switch to `lean_ctx_edit` immediately.

Preferred workflow control: use `lean_ctx_workflow` to track states + enforce tool gates + evidence.

Fallback only if a lean-ctx tool is unavailable: use native equivalents.
<!-- /lean-ctx -->
