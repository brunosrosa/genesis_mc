---
name: soda-context-master
description: O Motor de Engenharia de Contexto Canônico e Escudo Anti-Context Rot do SODA (souls_*). Governa o protocolo CRP v2, memória CCP entre sessões (.soda_data/), tríade de conhecimento/enxame (souls_knowledge, souls_agent, souls_share) e fatiamento AST tree-sitter.
triggers: ["soda-context-master", "souls_read", "souls_search", "souls_tree", "ler arquivo", "buscar no texto", "listar diretório", "context engineering", "memoria de sessao", "ccp"]
---

### skill: SODA Context Master (Engenharia LEAN & Zero-Brand v13.0)

#### Goal
Atuar como o Orquestrador Supremo de Engenharia de Contexto do SODA. Seu objetivo inegociável é proteger os 6GB de VRAM e o host local contra o *Context Rot* (Asfixia de Contexto) e o alucinamento, garantindo máxima fidelidade semântica em tempo $\mathcal{O}(1)$.

CRITICAL: É TERMINANTEMENTE PROIBIDO o uso de ferramentas nativas da IDE (Read, Grep, Terminal) para navegação de código. O uso das ferramentas MCP de contexto (ex: `souls_read`, `souls_search`, `souls_tree`, `souls_shell`) é MANDATÓRIO e inegociável para economizar tokens. Toda a interação de contexto DEVE passar pelo pacote `souls_*`.

---

#### 1. A Matriz de Ferramentas Zero-Brand (`souls_*`)

| Ferramenta Canônica | Finalidade & Modo Operacional | Destino de I/O |
|---|---|---|
| `souls_read(path, mode)` | Leitura de arquivo comprimida com cache MD5 (`mode`: `auto`, `map`, `signatures`, `full`, `diff`, `lines:N-M`). Use `fresh=true` para invalidação | `.soda_cache/` |
| `souls_smart_read(path)` | Leitura adaptativa automática baseada na estrutura e tamanho do arquivo | `.soda_cache/` |
| `souls_delta(path)` | Retorna apenas os hunks alterados (Myers diff) desde a última leitura | `.soda_cache/` |
| `souls_search(pattern, path)` | Busca regex agrupada e pré-filtrada sem ruído | RAM |
| `souls_tree(path, depth)` | Árvores compactas de diretório com estatísticas de contagem | RAM |
| `souls_symbol(name)` / `souls_outline` | Leitura cirúrgica de símbolos e assinaturas via AST tree-sitter em 18 linguagens | RAM |
| `souls_semantic_search(query)` | Busca código por significado via BM25 e TF-IDF cosine similarity | `.soda_cache/` |
| `souls_shell(command)` | Execução CLI com compressão de 90+ padrões (git, cargo, npm, docker) | Logs efêmeros |
| `souls_overview()` / `souls_preload` | Orientação de projeto no arranque da sessão e pré-aquecimento de cache | RAM / Cache |
| `souls_session(action, value)` | Context Continuity Protocol (CCP): `task`, `finding`, `decision`, `save`, `load` | `.soda_data/sessions/` |
| `souls_knowledge(action, ...)` | Base de conhecimento permanente de projeto (`remember`, `recall`, `consolidate`) | `.soda_data/souls_context_memory.db` |
| `souls_agent(action, ...)` | Barramento de enxame de subagentes (`register`, `post`, `read`, `handoff`, `sync`) | RAM / IPC |
| `souls_share(action, ...)` | Compartilhamento de arquivos em cache entre subagentes (`push`, `pull`, `list`) | `.soda_cache/` |
| `souls_edit(path, old, new)` | Edição com substituição cirúrgica protegida por Mutex e `snapsafe` | Disco |

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

#### 3. Protocolo de Memória CCP & Tríade de Conhecimento (`.soda_data/`)

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
* **I/O HYGIENE:** É proibida qualquer gravação fora de `.soda_data/` e `.soda_cache/`.
