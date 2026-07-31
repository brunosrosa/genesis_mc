---
name: mcp-sqlite-master
description: Arquiteto de Dados da Memória L2 do SOULS. Manual estrito para o FrankenSQLite servido intra-processo pelo Gateway Rust via `db_query`, com bloqueio de mutações e consultas somente leitura.
triggers: ["mcp-sqlite-master", "consultar banco", "ler sqlite", "ver tabela", "banco de dados", "histórico de logs", "memória l2", "buscar memória", "pesquisar histórico"]
---

### skill: MCP SQLite Master (O Interrogador da Memória L2 V7.0)

#### Goal
Atuar como o Arquiteto de Dados Cirúrgico para a Memória Transacional e Episódica (L2) do SOULS. Os bancos locais `souls_state.db` e `souls_heuristic_vault.db` são a Única Fonte da Verdade (SSOT) para eventos e estados estruturados. Seu objetivo inegociável é proteger a Tríade de Memória (L2 nunca processa vetores, isso é papel do L3/LanceDB), prevenir o OOM através da extração sub-textual de JSONs, abolir comandos destrutivos protegendo o *Event Sourcing* e operar via `db_query` em modo somente leitura.

#### Instructions
Sempre que precisar acessar a memória relacional do SOULS, utilize exclusivamente este MCP sob esta máquina de estados restritiva:

1. **Firewall Compliance e Reconhecimento Seguro:**
   * Utilize OBRIGATORIAMENTE `db_query` para leituras locais no cofre SQLite.
   * Informe o `db_name` correto (`souls_state.db` ou `souls_heuristic_vault.db`) e mantenha a query estritamente informacional.

2. **A Lei da Tríade de Memória (Fobia Vetorial L2):**
   * **PROIBIÇÃO DE VETORES:** O SQLite é a sua memória relacional (L2). O LanceDB (L3) é a sua memória vetorial. Você está SUMARIAMENTE PROIBIDO de tentar realizar buscas matemáticas (ex: `vec_distance_L2`) no SQLite. Confie apenas em chaves estrangeiras, joins e filtros de tempo (Epoch).

3. **Imutabilidade e Rebase Semântico (A Morte do DELETE):**
   * O SOULS utiliza a arquitetura de *Event Sourcing*. O histórico é sagrado e mantido pelo `gitoxide`.
   * Se você precisar alterar o estado de um registro via MCP (ex: mover um cartão Kanban), é TERMINANTEMENTE PROIBIDO usar os comandos SQL `UPDATE` ou `DELETE` clássicos.
   * Você deve realizar inserções de transição de estado ou usar lógicas de **Tombstone** (inserir a flag `is_deleted = 1` no registro novo correspondente). 

4. **Blindagem do FrankenSQLite (Anti-Mutação):**
   * O backend em Rust configura este banco cirurgicamente com MVCC e *Write-Merge Ladder* em modo WAL.
   * É PROIBIDO executar comandos administrativos como `VACUUM`, `PRAGMA journal_mode`, `PRAGMA synchronous` ou manipulações de trava. `db_query` barra mutações e PRAGMAs alteradores por design.

5. **Interrogação Temporal (Epoch) e Proteção de VRAM:**
   * O tempo é registrado em **UNIX Epoch Int64 UTC**. Converta strings mentalmente para matemática Epoch (ex: `timestamp > 1713368700`).
   * Fatie textos longos para proteger o motor V8 do Svelte 5: use OBRIGATORIAMENTE `json_extract(payload, '$.chave')` ou `substr(coluna, 1, 300)` nas colunas densas.
   * Força Bruta Banida: Exija sempre a cláusula `LIMIT`. O Gateway ainda truncará a resposta em 200 linhas.

6. **Consciência de Zero-Trust (O Hook `SQLITE_IGNORE`):**
   * O Rusqlite Authorizer em Rust bloqueia acesso a tabelas privadas secretamente. Se sua query retornar `NULL` em colunas que deveriam ter dados, você bateu na defesa do Kernel. Recue e notifique o usuário para elevação de privilégios (HITL).

#### Constraints
* **FALSA AMNÉSIA FTS5:** Se uma consulta em tabela virtual FTS5 voltar vazia (`MATCH`), relaxe a busca com curingas antes de assumir que o dado não existe.
* **FRONTMATTER ABSOLUTO:** O bloco YAML `---` no topo desta skill é a fundação inegociável do roteamento.

#### Examples
**Entrada do Usuário:** "SOULS, dá uma olhada no SQLite e veja as últimas 3 tarefas que o Agente Financeiro concluiu hoje. Quero só os resumos."

**Ação do Agente:**
1. Invoca `db_query` com `db_name: "souls_state.db"` e uma query limitada.
2. Calcula matematicamente o limite Epoch UTC para "hoje".
3. Constrói a query blindada abstendo-se de `SELECT *` e garantindo o limite:
   `SELECT task_id, substr(resultado, 1, 200) as resumo FROM task_events WHERE status = 'CONCLUIDO' AND timestamp > 1715040000 ORDER BY timestamp DESC LIMIT 3;`
4. Retorna a síntese destilada no Canvas, garantindo que o SQLite continuou atuando apenas como L2 Relacional e a VRAM local foi preservada.

