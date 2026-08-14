---
name: souls-context-master
description: O Motor de Engenharia de Contexto Canônico e Escudo Anti-Context Rot do SOULS (souls_*). Governa o protocolo CRP v2, memória CCP entre sessões (.souls_data/), tríade de conhecimento/enxame (souls_knowledge, souls_agent, souls_share) e fatiamento AST tree-sitter.
triggers: ["souls-context-master", "souls_read", "souls_search", "souls_tree", "ler arquivo", "buscar no texto", "listar diretório", "context engineering", "memoria de sessao", "ccp"]
---

### skill: SOULS Context Master (Engenharia LEAN & Zero-Brand v13.0)

#### Goal
Atuar como o Orquestrador Supremo de Engenharia de Contexto do SOULS. Seu objetivo inegociável é proteger os 6GB de VRAM e o host local contra o *Context Rot* (Asfixia de Contexto) e o alucinamento, garantindo máxima fidelidade semântica em tempo $\mathcal{O}(1)$.

CRITICAL: É TERMINANTEMENTE PROIBIDO o uso de ferramentas nativas da IDE (Read, Grep, Terminal) para navegação de código. O uso das ferramentas MCP de contexto (`read`, `search`, `tree`, `shell`, `semantic_search` / aliases: `souls_*`, `ctx_*`) é MANDATÓRIO e inegociável para economizar tokens. O servidor MCP normaliza e aceita qualquer uma dessas variações.

---

#### 1. A Matriz de Ferramentas Zero-Brand (Nomes Canônicos e Aliases)

| Ferramenta Canônica | Aliases Suportados | Finalidade & Modo Operacional | Destino de I/O |
|---|---|---|---|
| `read(path)` | `souls_read`, `ctx_read` | Leitura de arquivo comprimida com cache MD5 e Saco a Vácuo lossless | `.souls_cache/` |
| `smart_read(path)` | `souls_smart_read` | Leitura adaptativa automática baseada na estrutura e orçamento de tokens | `.souls_cache/` |
| `delta_diff(before, after)` | `delta`, `souls_delta` | Retorna apenas os hunks alterados (Myers diff estrutural) | RAM |
| `search(query, path)` | `souls_search`, `ctx_search` | Busca regex agrupada e pré-filtrada sem ruído | RAM |
| `tree(path, depth)` | `souls_tree`, `ctx_tree` | Árvores compactas de diretório com Dot-Flattening estrito | RAM |
| `symbol(name)` | `souls_symbol` | Localização física exata de símbolos via AST tree-sitter | RAM |
| `outline(file_path)` | `souls_outline` | Extração de assinaturas e contratos sem corpos de funções | RAM |
| `semantic_search(query)` | `souls_semantic_search` | Busca híbrida RRF local (FTS5 + LanceDB) com sanitização de grafos | RAM / SSD |
| `shell(command)` | `souls_shell`, `ctx_shell` | Execução CLI com compressão e poda de logs de terminal | Logs efêmeros |
| `session(action)` | `souls_session` | Context Continuity Protocol (CCP): `task`, `finding`, `decision`, `clear` | `.souls_data/` |
| `knowledge(key, ...)` | `souls_knowledge` | Base de conhecimento permanente de projeto L2 (SQLite) | `.souls_data/` |
| `sub_agent(...)` | `souls_sub_agent` | Barramento e telemetria de subagentes | `.souls_data/` |
| `edit(path, old, new)` | `souls_edit`, `ctx_edit` | Edição com substituição cirúrgica protegida por Mutex e verificação sintática | Disco |
| `thinking(...)` | `core_think`, `sequentialthinking` | Raciocínio socrático estruturado com freio cognitivo | RAM |

---

#### 2. Protocolo de Resposta Compacta (CRP v2)

Every token costs money and context space. Applies to input, output AND thinking tokens:

##### Thinking Reduction (30–60% economia de tokens de raciocínio)
1. **Hipótese Única:** Formule uma hipótese por vez e teste-a no silício. Não enumere 5 abordagens abstratas.
2. **Parada Prematura:** Encerre o pensamento imediatamente assim que a resposta técnica for encontrada.
3. **Rastreamento por Referências:** Use referências persistentes `F1=src/main.rs`, `F2=Cargo.toml` em toda a sessão.

##### Output Reduction (50–80% economia de tokens de saída)
1. **Prosa Zero:** Forneça apenas código, tabelas e resumos executivos. Proibido ecoar trechos lidos.
2. **Resumo de 1 Linha:** Ferramentas e builds devem ser ressumidos em 1 linha max: `-> Built: 0 errors`.
3. **Notação Compacta:**
   - `F:path` — lendo arquivo
   - `+file` — criado / adicionado
   - `~file` — modificado
   - `!file` — erro / falha
4. **Estrutura:** Bullets > Parágrafos. Tabelas > Listas.

---

#### 3. Protocolo de Memória CCP & Tríade de Conhecimento (`.souls_data/`)

1. **Arranque do Chat / Sessão:**
   - Execute `souls_session(action="load")` para restaurar o estado da tarefa anterior (`task`, `findings`, `decisions`).
   - Se for o primeiro acesso no projeto, rode `souls_overview()` para carregar o mapa AST básico.
2. **Durante a Execução:**
   - Ao descobrir um Gotcha ou bug estrutural, registre via `souls_knowledge(action="remember", category="gotcha", key="...", value="...")`.
   - Ao definir uma arquitetura, salve via `souls_session(action="decision", value="...")`.
3. **Enxame Agêntico (Subagentes):**
   - Subagentes devem rodar `souls_agent(action="register", role="worker")`.
   - Para evitar re-ler arquivos do disco, subagentes compartilham contexto via `souls_share(action="push", paths="...")` e `pull`.

---

#### 4. Cascata de Fallback Gracioso

1. **Nível 1 (MANDATÓRIO):** Ferramentas MCP `souls_*`.
2. **Nível 2 (RETAGUARDA CLI):** Se o MCP server responder com erro de conexão, utilize o CLI envelopado `lean-ctx -c "<comando>"`.
3. **Nível 3 (ÚLTIMO RECURSO):** Utilize ferramentas nativas da IDE (`view_file`, `grep_search`, `list_dir`, `run_command`) **APENAS** se os Níveis 1 e 2 falharem catastroficamente.

---

#### Constraints
* **PROIBIÇÃO DE PARÂMETROS NATIVOS:** É EXPRESSAMENTE PROIBIDO injetar parâmetros nativos (StartLine, AbsolutePath) no `souls_read`. A assinatura exige estritamente `path` e `mode`.
* **PROIBIDO BUSCAS CEGAS:** Nunca use `souls_search` passando a raiz do projeto como `path`. Você DEVE mapear o terreno com `souls_tree` e apontar o `search` para a sub-pasta exata (ex: `src/modules/`). O descumprimento causará truncamento algorítmico.
* **SUBAGENTES (fresh=true):** Ao instanciar subagentes ou após compilações externas (`cargo build`), force `fresh=true` na releitura de arquivos modificados.
* **I/O HYGIENE:** É proibida qualquer gravação fora de `.souls_data/` e `.souls_cache/`.
