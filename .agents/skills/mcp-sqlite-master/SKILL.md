---
name: mcp-sqlite-master
description: Arquiteto de Dados da Memória L2 do SODA. Manual estrito para o FrankenSQLite. Respeita a Tríade de Memória (Proibição de buscas vetoriais no L2). Impõe Rebase Semântico (Tombstones, Anti-UPDATE/DELETE), bloqueia comandos PRAGMA/VACUUM e respeita o Authorizer nativo em Rust.
triggers: ["mcp-sqlite-master", "consultar banco", "ler sqlite", "ver tabela", "banco de dados", "histórico de logs", "memória l2", "buscar memória", "pesquisar histórico"]
---

### skill: MCP SQLite Master (O Interrogador da Memória L2 V6.0)

#### Goal
Atuar como o Arquiteto de Dados Cirúrgico para a Memória Transacional e Episódica (L2) do SODA. O banco FrankenSQLite local (`genesis.db`) é a Única Fonte da Verdade (SSOT) para eventos e estados estruturados. Seu objetivo inegociável é proteger a Tríade de Memória (L2 nunca processa vetores, isso é papel do L3/LanceDB), prevenir o OOM através da extração sub-textual de JSONs, abolir comandos destrutivos protegendo o *Event Sourcing* e jamais tocar nas otimizações de *hardware* implementadas pelo kernel Rust.

#### Instructions
Sempre que precisar acessar a memória relacional do SODA, utilize exclusivamente este MCP sob esta máquina de estados restritiva:

1. **Firewall Compliance e Reconhecimento Seguro:**
   * Utilize OBRIGATORIAMENTE os nomes exatos permitidos pela válvula CEL do Gateway: `sqlite_list_tables`, `sqlite_describe_table` e `sqlite_read_query`.
   * Mapeie a topologia com `list_tables` antes de agir.

2. **A Lei da Tríade de Memória (Fobia Vetorial L2):**
   * **PROIBIÇÃO DE VETORES:** O SQLite é a sua memória relacional (L2). O LanceDB (L3) é a sua memória vetorial. Você está SUMARIAMENTE PROIBIDO de tentar realizar buscas matemáticas (ex: `vec_distance_L2`) no SQLite. Confie apenas em chaves estrangeiras, joins e filtros de tempo (Epoch).

3. **Imutabilidade e Rebase Semântico (A Morte do DELETE):**
   * O SODA utiliza a arquitetura de *Event Sourcing*. O histórico é sagrado e mantido pelo `gitoxide`.
   * Se você precisar alterar o estado de um registro via MCP (ex: mover um cartão Kanban), é TERMINANTEMENTE PROIBIDO usar os comandos SQL `UPDATE` ou `DELETE` clássicos.
   * Você deve realizar inserções de transição de estado ou usar lógicas de **Tombstone** (inserir a flag `is_deleted = 1` no registro novo correspondente). 

4. **Blindagem do FrankenSQLite (Anti-PRAGMA):**
   * O backend em Rust configura este banco cirurgicamente com MVCC e *Write-Merge Ladder* em modo WAL.
   * É PROIBIDO executar comandos administrativos como `VACUUM`, `PRAGMA journal_mode`, `PRAGMA synchronous` ou manipulações de trava. Tentar "otimizar" a base via MCP resultará no travamento do *Event Loop* do Tokio.

5. **Interrogação Temporal (Epoch) e Proteção de VRAM:**
   * O tempo é registrado em **UNIX Epoch Int64 UTC**. Converta strings mentalmente para matemática Epoch (ex: `timestamp > 1713368700`).
   * Fatie textos longos para proteger o motor V8 do Svelte 5: use OBRIGATORIAMENTE `json_extract(payload, '$.chave')` ou `substr(coluna, 1, 300)` nas colunas densas.
   * Força Bruta Banida: Exija sempre a cláusula `LIMIT`.

6. **Consciência de Zero-Trust (O Hook `SQLITE_IGNORE`):**
   * O Rusqlite Authorizer em Rust bloqueia acesso a tabelas privadas secretamente. Se sua query retornar `NULL` em colunas que deveriam ter dados, você bateu na defesa do Kernel. Recue e notifique o usuário para elevação de privilégios (HITL).

#### Constraints
* **FALSA AMNÉSIA FTS5:** Se uma consulta em tabela virtual FTS5 voltar vazia (`MATCH`), relaxe a busca com curingas antes de assumir que o dado não existe.
* **FRONTMATTER ABSOLUTO:** O bloco YAML `---` no topo desta skill é a fundação inegociável do roteamento.

#### Examples
**Entrada do Usuário:** "SODA, dá uma olhada no SQLite e veja as últimas 3 tarefas que o Agente Financeiro concluiu hoje. Quero só os resumos."

**Ação do Agente:**
1. Invoca `sqlite_list_tables` e localiza a tabela `task_events`.
2. Calcula matematicamente o limite Epoch UTC para "hoje".
3. Constrói a query blindada abstendo-se de `SELECT *` e garantindo o limite:
   `SELECT task_id, substr(resultado, 1, 200) as resumo FROM task_events WHERE status = 'CONCLUIDO' AND timestamp > 1715040000 ORDER BY timestamp DESC LIMIT 3;`
4. Retorna a síntese destilada no Canvas, garantindo que o SQLite continuou atuando apenas como L2 Relacional e a VRAM local foi preservada.