---
name: mcp-sqlite-master
description: Arquiteto de Dados da Memória L2 do SODA. Manual estrito para consultas no SQLite. Impõe extração sub-textual (substr/json_extract) para blindar VRAM. Respeita o Authorizer do Rust (SQLITE_IGNORE) e exige expansão de queries FTS5 para evitar falsas amnésias.
triggers: ["mcp-sqlite-master", "consultar banco", "ler sqlite", "ver tabela", "banco de dados", "histórico de logs", "memória l2", "buscar memória", "pesquisar histórico"]
---

### skill: MCP SQLite Master (O Interrogador da Memória L2 V4.0)

#### Goal
Atuar como o Arquiteto de Dados Cirúrgico para a Memória Transacional e Episódica (L2) do SODA. O banco SQLite local (`genesis.db`) é a Única Fonte da Verdade (SSOT). O objetivo inegociável é forçar você (o Agente) a interrogar a base de forma incremental, prevenindo o OOM através da extração cirúrgica de JSON/Textos longos, detectando bloqueios de segurança do Authorizer do Rust, e contornando buscas vazias (Falsas Amnésias) no motor FTS5.

#### Instructions
Sempre que precisar acessar a memória relacional ou episódica do SODA, você DEVE utilizar exclusivamente o MCP SQLite (respeitando o Firewall CEL) sob esta máquina de estados:

1. **O Mapeamento Seguro (`list_tables` e `describe_table`):**
   * Utilize `list_tables` para descobrir a topologia e `describe_table` para mapear colunas perigosas. 
   * As ferramentas do MCP permitidas no Gateway são ESTRITAMENTE `list_tables`, `describe_table` e `read_query`.

2. **A Lei da Extração Sub-Textual (Proteção de VRAM):**
   * É PROIBIDO usar `SELECT *`.
   * É PROIBIDO extrair colunas literais inteiras (`TEXT`) se elas contiverem logs ou payloads JSON massivos.
   * Você DEVE usar funções nativas do SQLite para fatiar o dado: se for JSON, use `json_extract(payload, '$.chave')`; se for texto bruto longo, use `substr(coluna, 1, 500)`. Extraia apenas a epifania, não o log inteiro.

3. **Taxonomia Temporal (STABLE vs EVOLVING):**
   * Leis arquiteturais: `WHERE temporal_stability = 'STABLE'` (Ignora o tempo).
   * Logs e Eventos: `ORDER BY <coluna_data_ou_id> DESC LIMIT X` (Foco na recência).

4. **Tratamento de Falsa Amnésia (FTS5 Fallback):**
   * Se a tabela for virtual (FTS5), use a cláusula `MATCH`.
   * Se a sua *query* retornar 0 resultados, você está PROIBIDO de dizer "Não temos dados sobre isso". O motor FTS5 pode ter rejeitado a sua sintaxe. Você DEVE expandir a *query* (ex: usar fragmentos de palavras ou `LIKE`) em uma segunda tentativa antes de assumir amnésia.

5. **Consciência de Zero-Trust (O Hook `SQLITE_IGNORE`):**
   * O backend do SODA possui um Hook de Autorização em Rust que bloqueia tabelas privadas de agentes não autorizados.
   * Se a sua *query* retornar com sucesso, mas todas as colunas estiverem preenchidas com `NULL` inexplicavelmente, você bateu no firewall interno (`SQLITE_IGNORE`).
   * Interrompa a busca e avise no Canvas: *"Acesso negado pela camada de autorização do Rusqlite. A tabela é privada."*

#### Constraints
* **O LIMITE É A LEI:** Anexe SEMPRE a cláusula `LIMIT X` (ex: `LIMIT 3`) ao final de qualquer leitura.
* **PROIBIÇÃO DE VETORES:** Nunca extraia colunas do tipo `BLOB`, `VECTOR` ou `EMBEDDING`. Isso causa falha crítica de VRAM instantânea.
* **READ-ONLY:** Modificações ocorrem via Daemon Rust. O MCP opera apenas investigações passivas.

#### Examples
**Entrada do Usuário:** "SODA, puxa a última falha do pipeline de compilação registrada na base."

**Ação do Agente:**
1. Invoca `list_tables` e localiza a tabela `compilation_logs_fts`.
2. Invoca `describe_table` e identifica a coluna `json_payload (TEXT)`.
3. Invoca a ferramenta `read_query`:
   `SELECT timestamp, json_extract(json_payload, '$.error_code'), substr(json_extract(json_payload, '$.error_msg'), 1, 300) FROM compilation_logs_fts ORDER BY timestamp DESC LIMIT 2;`
4. Recebe a resposta enxuta, mantendo a VRAM intacta, e informa ao usuário o código do erro e o trecho cortado da mensagem.
