# MCP Nomenclature Audit Transition Zero Brand

## Escopo

- Auditoria sem mutacao de codigo.
- Fontes fisicas lidas:
  - `Z:\genesis_mc\docs\adrs\ADR-026-Nomenclatura-Semantica-Zero-Brand.md`
  - `Z:\genesis_mc\gateway-config.yaml`
  - `Z:\genesis_mc\.souls_scratchpad\reports\_MCP_INVENTORY_souls-agent-gateway.txt`
  - `Z:\genesis_mc\src-tauri\src\bin\souls_mcp_server.rs`
  - `Z:\genesis_mc\.agents\skills\**\SKILL.md`

## Verdade Atual

- Inventario efetivamente exposto hoje ao cliente MCP conectado: `52` ferramentas.
- Composicao real capturada:
  - `46` ferramentas do tronco `lean-ctx`
  - `6` ferramentas do tronco `souls-native-ast`
- `gateway-config.yaml` ainda referencia nomes legados do tronco nativo (`souls_*`) e tambem ja contem pistas de migracao (`web_search`, `ctx_.*`).
- `sequentialthinking` e `memory_*` aparecem na allowlist/backends do `gateway-config.yaml`, mas NAO aparecem no inventario capturado do servidor ligado. Portanto, nao entram na matriz atomica desta rodada sem nova captura de exposicao real.

## Achados Criticos

1. Ha tres camadas de nomenclatura convivendo ao mesmo tempo:
   - Servidor ligado na IDE: `souls-agent-gateway`
   - Backend nativo Rust anunciado em `serverInfo.name`: `souls-native-ast`
   - Ferramentas atomicas nativas ainda com prefixo de marca/projeto: `souls_*`
2. O tronco `lean-ctx` ja esta semanticamente quase conforme na camada atomica (`ctx_*`), mas a exposicao atual ainda sangra redundancia via prefixo de backend: `lean-ctx_ctx_*`.
3. Existe uma divergencia material na ferramenta de busca web:
   - Inventario salvo: `souls-native-ast_souls_duckgo_search`
   - Descritor MCP real: `souls-native-ast_souls_duckduckgo_search`
   - Rust server: `souls_duckduckgo_search`
   - Allowlist do gateway: `souls_duckduckgo_search`
   Essa discrepancia precisa ser tratada como risco de cache/artefato derivado antes da refatoracao.

## Regra ADR-026 Aplicada

- Nome de servidor exportado ao cliente: `souls`
- Padrao de ferramenta atomica: `<dominio>_<acao>`
- Sem marca, sem `mcp_`, sem `tool_`, sem `souls_`, sem fornecedor (`duckduckgo`, `github`) no nome.

## Matriz De Renomeacao

| Exposto hoje | Atomo atual | Novo nome proposto | Exposicao final esperada | Origem |
|---|---|---|---|---|
| `lean-ctx_ctx_agent` | `ctx_agent` | `ctx_agent` | `souls_ctx_agent` | `lean-ctx` |
| `lean-ctx_ctx_analyze` | `ctx_analyze` | `ctx_analyze` | `souls_ctx_analyze` | `lean-ctx` |
| `lean-ctx_ctx_architecture` | `ctx_architecture` | `ctx_architecture` | `souls_ctx_architecture` | `lean-ctx` |
| `lean-ctx_ctx_benchmark` | `ctx_benchmark` | `ctx_benchmark` | `souls_ctx_benchmark` | `lean-ctx` |
| `lean-ctx_ctx_cache` | `ctx_cache` | `ctx_cache` | `souls_ctx_cache` | `lean-ctx` |
| `lean-ctx_ctx_callees` | `ctx_callees` | `ctx_callees` | `souls_ctx_callees` | `lean-ctx` |
| `lean-ctx_ctx_callers` | `ctx_callers` | `ctx_callers` | `souls_ctx_callers` | `lean-ctx` |
| `lean-ctx_ctx_compress_memory` | `ctx_compress_memory` | `ctx_compress_memory` | `souls_ctx_compress_memory` | `lean-ctx` |
| `lean-ctx_ctx_compress` | `ctx_compress` | `ctx_compress` | `souls_ctx_compress` | `lean-ctx` |
| `lean-ctx_ctx_context` | `ctx_context` | `ctx_context` | `souls_ctx_context` | `lean-ctx` |
| `lean-ctx_ctx_cost` | `ctx_cost` | `ctx_cost` | `souls_ctx_cost` | `lean-ctx` |
| `lean-ctx_ctx_dedup` | `ctx_dedup` | `ctx_dedup` | `souls_ctx_dedup` | `lean-ctx` |
| `lean-ctx_ctx_delta` | `ctx_delta` | `ctx_delta` | `souls_ctx_delta` | `lean-ctx` |
| `lean-ctx_ctx_discover` | `ctx_discover` | `ctx_discover` | `souls_ctx_discover` | `lean-ctx` |
| `lean-ctx_ctx_edit` | `ctx_edit` | `ctx_edit` | `souls_ctx_edit` | `lean-ctx` |
| `lean-ctx_ctx_execute` | `ctx_execute` | `ctx_execute` | `souls_ctx_execute` | `lean-ctx` |
| `lean-ctx_ctx_feedback` | `ctx_feedback` | `ctx_feedback` | `souls_ctx_feedback` | `lean-ctx` |
| `lean-ctx_ctx_fill` | `ctx_fill` | `ctx_fill` | `souls_ctx_fill` | `lean-ctx` |
| `lean-ctx_ctx_gain` | `ctx_gain` | `ctx_gain` | `souls_ctx_gain` | `lean-ctx` |
| `lean-ctx_ctx_graph_diagram` | `ctx_graph_diagram` | `ctx_graph_diagram` | `souls_ctx_graph_diagram` | `lean-ctx` |
| `lean-ctx_ctx_graph` | `ctx_graph` | `ctx_graph` | `souls_ctx_graph` | `lean-ctx` |
| `lean-ctx_ctx_handoff` | `ctx_handoff` | `ctx_handoff` | `souls_ctx_handoff` | `lean-ctx` |
| `lean-ctx_ctx_heatmap` | `ctx_heatmap` | `ctx_heatmap` | `souls_ctx_heatmap` | `lean-ctx` |
| `lean-ctx_ctx_impact` | `ctx_impact` | `ctx_impact` | `souls_ctx_impact` | `lean-ctx` |
| `lean-ctx_ctx_intent` | `ctx_intent` | `ctx_intent` | `souls_ctx_intent` | `lean-ctx` |
| `lean-ctx_ctx_knowledge` | `ctx_knowledge` | `ctx_knowledge` | `souls_ctx_knowledge` | `lean-ctx` |
| `lean-ctx_ctx_metrics` | `ctx_metrics` | `ctx_metrics` | `souls_ctx_metrics` | `lean-ctx` |
| `lean-ctx_ctx_multi_read` | `ctx_multi_read` | `ctx_multi_read` | `souls_ctx_multi_read` | `lean-ctx` |
| `lean-ctx_ctx_outline` | `ctx_outline` | `ctx_outline` | `souls_ctx_outline` | `lean-ctx` |
| `lean-ctx_ctx_overview` | `ctx_overview` | `ctx_overview` | `souls_ctx_overview` | `lean-ctx` |
| `lean-ctx_ctx_prefetch` | `ctx_prefetch` | `ctx_prefetch` | `souls_ctx_prefetch` | `lean-ctx` |
| `lean-ctx_ctx_preload` | `ctx_preload` | `ctx_preload` | `souls_ctx_preload` | `lean-ctx` |
| `lean-ctx_ctx_read` | `ctx_read` | `ctx_read` | `souls_ctx_read` | `lean-ctx` |
| `lean-ctx_ctx_response` | `ctx_response` | `ctx_response` | `souls_ctx_response` | `lean-ctx` |
| `lean-ctx_ctx_routes` | `ctx_routes` | `ctx_routes` | `souls_ctx_routes` | `lean-ctx` |
| `lean-ctx_ctx_search` | `ctx_search` | `ctx_search` | `souls_ctx_search` | `lean-ctx` |
| `lean-ctx_ctx_semantic_search` | `ctx_semantic_search` | `ctx_semantic_search` | `souls_ctx_semantic_search` | `lean-ctx` |
| `lean-ctx_ctx_session` | `ctx_session` | `ctx_session` | `souls_ctx_session` | `lean-ctx` |
| `lean-ctx_ctx_share` | `ctx_share` | `ctx_share` | `souls_ctx_share` | `lean-ctx` |
| `lean-ctx_ctx_shell` | `ctx_shell` | `ctx_shell` | `souls_ctx_shell` | `lean-ctx` |
| `lean-ctx_ctx_smart_read` | `ctx_smart_read` | `ctx_smart_read` | `souls_ctx_smart_read` | `lean-ctx` |
| `lean-ctx_ctx_symbol` | `ctx_symbol` | `ctx_symbol` | `souls_ctx_symbol` | `lean-ctx` |
| `lean-ctx_ctx_task` | `ctx_task` | `ctx_task` | `souls_ctx_task` | `lean-ctx` |
| `lean-ctx_ctx_tree` | `ctx_tree` | `ctx_tree` | `souls_ctx_tree` | `lean-ctx` |
| `lean-ctx_ctx_workflow` | `ctx_workflow` | `ctx_workflow` | `souls_ctx_workflow` | `lean-ctx` |
| `lean-ctx_ctx_wrapped` | `ctx_wrapped` | `ctx_wrapped` | `souls_ctx_wrapped` | `lean-ctx` |
| `souls-native-ast_souls_get_ast` | `souls_get_ast` | `repo_ast` | `souls_repo_ast` | `souls-native-ast` |
| `souls-native-ast_souls_fetch_web` | `souls_fetch_web` | `web_fetch` | `souls_web_fetch` | `souls-native-ast` |
| `souls-native-ast_souls_get_time` | `souls_get_time` | `sys_time` | `souls_sys_time` | `souls-native-ast` |
| `souls-native-ast_souls_duckduckgo_search` | `souls_duckduckgo_search` | `web_search` | `souls_web_search` | `souls-native-ast` |
| `souls-native-ast_souls_github_meta` | `souls_github_meta` | `repo_meta` | `souls_repo_meta` | `souls-native-ast` |
| `souls-native-ast_souls_sqlite_query` | `souls_sqlite_query` | `db_query` | `souls_db_query` | `souls-native-ast` |

## Blast Radius

### 1. Runtime / Gateway / Exposicao MCP

- `Z:\genesis_mc\gateway-config.yaml`
  - Reescrever regex CEL da allowlist:
    - hoje aceita `souls_get_ast|souls_fetch_web|souls_github_meta|souls_sqlite_query|souls_get_time|souls_duckduckgo_search`
    - alvo ADR-026: `repo_ast|web_fetch|repo_meta|db_query|sys_time|web_search`
  - Revisar tambem a convivencia de `web_search` novo com nome legado antigo na mesma regra.
  - Revisar nomes de backends, se eles influenciam o prefixo exportado ao cliente:
    - `lean-ctx`
    - `sequential_thinking`
    - `memory-mcp-rs`
    - `souls-native-ast`

- `Z:\genesis_mc\src-tauri\src\bin\souls_mcp_server.rs`
  - `serverInfo.name`: `souls-native-ast` -> `souls` ou outro alias atomico decidido para a camada exportada.
  - Tabela `tools/list`:
    - `souls_get_ast` -> `repo_ast`
    - `souls_fetch_web` -> `web_fetch`
    - `souls_get_time` -> `sys_time`
    - `souls_duckduckgo_search` -> `web_search`
    - `souls_github_meta` -> `repo_meta`
    - `souls_sqlite_query` -> `db_query`
  - Dispatch `match tool_name`:
    - trocar as chaves string legadas pelos nomes novos.
  - Simbolos Rust candidatos a rename interno:
    - `run_souls_get_ast` -> `run_repo_ast`
    - `run_souls_fetch_web` -> `run_web_fetch`
    - `run_souls_get_time` -> `run_sys_time`
    - `run_souls_duckduckgo_search` -> `run_web_search`
    - `run_souls_github_meta` -> `run_repo_meta`
    - `run_souls_sqlite_query` -> `run_db_query`

### 2. Skills / Roteamento Semantico

Arquivos com referencias textuais a nomes legados e/ou nomes que precisarao ser sincronizados com a ADR-026:

- `Z:\genesis_mc\.agents\skills\mcp-lean-ctx-master\SKILL.md`
- `Z:\genesis_mc\.agents\skills\mcp-search-master\SKILL.md`
- `Z:\genesis_mc\.agents\skills\mcp-time-master\SKILL.md`
- `Z:\genesis_mc\.agents\skills\mcp-jcodemunch-master\SKILL.md`
- `Z:\genesis_mc\.agents\skills\mcp-sqlite-master\SKILL.md`
- `Z:\genesis_mc\.agents\skills\souls-docs-hydrator\SKILL.md`
- `Z:\genesis_mc\.agents\skills\souls-github-orchestrator\SKILL.md`
- `Z:\genesis_mc\.agents\skills\souls-ralph-loop\SKILL.md`
- `Z:\genesis_mc\.agents\skills\souls-repo-analysis\SKILL.md`
- `Z:\genesis_mc\.agents\skills\skill-creator\SKILL.md`

Renomes textuais esperados nas skills:

- `souls_get_ast` -> `repo_ast`
- `souls_fetch_web` -> `web_fetch`
- `souls_get_time` -> `sys_time`
- `souls_duckduckgo_search` -> `web_search`
- `souls_github_meta` -> `repo_meta`
- `souls_sqlite_query` -> `db_query`
- `lean-ctx_ctx_*` NAO deve aparecer como contrato final de uso; a linguagem da skill deve apontar para os atomos `ctx_*`.

### 3. Artefatos Derivados / Cache / Relatorios

- `Z:\genesis_mc\.souls_scratchpad\reports\_MCP_INVENTORY_souls-agent-gateway.txt`
  - Deve ser regenerado apos a refatoracao para refletir:
    - servidor `souls`
    - nomes atomicos novos
  - Contem uma divergencia atual em `souls-native-ast_souls_duckgo_search`.

- `C:\Users\rosas\.trae\mcps\s_genesis_mc-39a22d09\dev_agent\mcp_souls-agent-gateway\tools\*.json`
  - Artefatos derivados de runtime/IDE.
  - Precisarao ser invalidados/regenarados apos o rename para evitar cache podre.
  - Nao sao SSOT de repositorio, mas entram no blast radius operacional.

## Decisao Operacional Recomendada

- Fase 1: homologar a matriz atomica acima.
- Fase 2: aplicar rename primeiro no servidor Rust e `gateway-config.yaml`.
- Fase 3: regenerar inventario MCP.
- Fase 4: sincronizar todas as `SKILL.md`.
- Fase 5: invalidar cache MCP da IDE e recapturar a exposicao real.
