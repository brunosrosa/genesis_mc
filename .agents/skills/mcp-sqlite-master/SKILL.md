---
name: mcp-sqlite-master
description: Manual estrito para leitura e interrogação da Memória L2 (SQLite FTS5) do SODA. Proíbe alucinação de esquemas e força o uso de paginação (LIMIT) para proteger a VRAM contra extrações massivas de dados.
triggers: ["mcp-sqlite-master", "consultar banco", "ler sqlite", "ver tabela", "banco de dados", "histórico de logs", "memória l2", "buscar memória", "pesquisar histórico"]
---

# Skill: MCP SQLite Master (Acesso à Memória L2)

## Goal
Atuar como o Arquiteto de Dados para a Memória Transacional e Episódica (L2) do SODA. O banco SQLite local (`genesis.db`) é a Única Fonte da Verdade (SSOT) do sistema, abrigando logs de eventos, o *GraphRAG* relacional e as heurísticas do *Life Coach*. O objetivo inegociável desta habilidade é ensinar o agente a interrogar este banco de forma segura e incremental usando as ferramentas do servidor `sqlite_soda`, prevenindo o colapso da VRAM por excesso de tokens de retorno e erradicando a alucinação de tabelas.

## Instructions
Sempre que você precisar resgatar um contexto histórico, ler logs do sistema ou consultar a memória relacional do SODA, você DEVE utilizar exclusivamente o MCP `sqlite_soda` seguindo esta máquina de estados de leitura:

1. **Reconhecimento de Terreno (Proibição de Alucinação):**
   - NUNCA tente adivinhar o nome de uma tabela.
   - O seu primeiro passo DEVE ser sempre invocar a ferramenta `sqlite_soda_list_tables` para mapear a topologia atual do banco de dados.

2. **Auditoria de Esquema (Schema Parsing):**
   - Após identificar a tabela alvo (ex: `event_logs` ou `heuristics`), você está PROIBIDO de fazer uma consulta cega.
   - Invoque OBRIGATORIAMENTE a ferramenta `sqlite_soda_describe_table` passando o nome da tabela. Você precisa entender os tipos de dados e, principalmente, se a tabela suporta índices de busca lexical rápida (FTS5).

3. **Extração Cirúrgica (Read-Only Query):**
   - Formule a sua requisição utilizando a ferramenta `sqlite_soda_read_query`.
   - Se a tabela for virtual (FTS5), priorize o uso da cláusula `MATCH` em vez de `LIKE` para consultas semânticas em $\mathcal{O}(1)$.
   - A sua query DEVE conter limites e ordenação lógica.

4. **Consciência de Isolamento (Read-Only State):**
   - Lembre-se: por questões de segurança de concorrência (WAL Mode) e integridade ACID, você não possui ferramentas MCP para executar `INSERT`, `UPDATE`, `DELETE` ou `DROP`. Modificações de estado são tratadas via IPC pelo Daemon Rust. Use o MCP exclusivamente para "lembrar" e "analisar".

## Constraints
* **A LEI DO LIMITADOR INEGOCIÁVEL:** NUNCA submeta uma query como `SELECT * FROM tabela;` sem cláusula de limite. Você DEVE SEMPRE anexar um `LIMIT X` (ex: `LIMIT 5` ou `LIMIT 10`). Se você injetar um log inteiro de 10.000 linhas no seu contexto, o hardware (6GB VRAM) colapsará em *Out-Of-Memory* (OOM).
* **ECONOMIA DE COLUNAS:** Extraia apenas as colunas necessárias para a sua análise. Não peça chaves estrangeiras ou hashes criptográficos se a sua intenção for apenas ler o texto do log.

## Examples

**Entrada do Usuário:** 
"SODA, o que nós decidimos na semana passada sobre a arquitetura do Wasmtime? Dá uma olhada na nossa memória."

**Ação do Agente:**
1. Invoca `sqlite_soda_list_tables` e localiza a tabela `architecture_decisions_fts`.
2. Invoca `sqlite_soda_describe_table` na tabela para descobrir que as colunas são `date`, `topic`, e `resolution`.
3. Invoca `sqlite_soda_read_query` com: `SELECT date, resolution FROM architecture_decisions_fts WHERE architecture_decisions_fts MATCH 'Wasmtime OR Sandbox' ORDER BY date DESC LIMIT 3;`
4. Lê os poucos tokens de resposta, absorve o contexto e responde ao usuário com base na decisão resgatada, mantendo a VRAM limpa.