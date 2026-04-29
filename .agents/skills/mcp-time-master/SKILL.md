---
name: mcp-time-master
description: O Relógio Absoluto do SODA. Impõe a Ditadura do UTC e Snapshot com Micro-incrementos Causais para lotes (SQLite). Aplica regras estritas de Anti-Hidratação no Svelte 5 (delegando a renderização visual ao JS) e exige aritmética exata (1 dia = 86400s) para o valid_to da taxonomia EVOLVING.
triggers: ["mcp-time-master", "ver hora", "que dia é hoje", "data atual", "ancoragem temporal", "agendar", "timestamp", "fuso horário"]
---

### skill: MCP Time Master (A Âncora Cronológica V5.0)

#### Goal
Atuar como a âncora matemática de realidade temporal do SODA. O objetivo inegociável é erradicar alucinações temporais, garantir ordenação causal em bancos de dados (SQLite/LanceDB) via Micro-incrementos, otimizar a Busca Híbrida via Contextual Chunks e proteger a interface do usuário (Svelte 5) contra bugs de hidratação de fuso horário.

#### Instructions
Sempre que uma tarefa exigir marcação de tempo, registro de logs, inserções em lote no banco ou codificação de componentes de UI que exibem datas, você DEVE utilizar o MCP de Tempo sob esta máquina de estados:

1. **Invocação e Snapshot Causal:**
   * Invoque a ferramenta EXATA `get_current_time` (respeitando o Firewall L7 CEL).
   * **Snapshot Único:** Faça a consulta APENAS UMA VEZ no início da tarefa e memorize o Epoch Int64 UTC.
   * **Micro-Incremento (Batch Insert):** Se for inserir múltiplas linhas no SQLite/LanceDB baseadas neste mesmo Snapshot, você DEVE adicionar seqüencialmente `+1`, `+2`, `+3` ao Epoch de cada linha para garantir o ordenamento causal no `ORDER BY timestamp DESC`.

2. **A Ditadura do UTC e a Lei da Anti-Hidratação (Svelte 5):**
   * Você está PROIBIDO de *hardcodar* strings de datas formatadas (ex: "17 de Abril") no código fonte do frontend. Isso quebra o fuso horário local do usuário e causa bugs de hidratação no DOM.
   * **No Frontend:** Passe sempre o número inteiro (Epoch UNIX) como estado (`$state`). Use nativamente o `Intl.DateTimeFormat` do JavaScript para formatar a visualização na máquina do cliente.
   * **No Backend (SQLite/LanceDB):** Grave OBRIGATORIAMENTE em UNIX Epoch Int64.
   * **Em Frontmatters (Markdown):** Grave OBRIGATORIAMENTE em ISO 8601 UTC (ex: `2026-04-17T15:45:00Z`).

3. **Matemática da Obsolescência Estrita (`valid_to`):**
   * Se o dado for `EVOLVING` (perecível), você DEVE preencher o metadado `valid_to`.
   * **Proibição de Adivinhação:** Não tente gerar a data de validade de forma estocástica. Use a constante universal: **1 dia = 86400 segundos**.
   * Calcule o `valid_to` somando o Epoch capturado + `(dias * 86400)`.

4. **A Lei do Contextual Chunk (Busca BM25):**
   * Ao redigir ou atualizar relatórios Markdown persistentes, você DEVE concatenar a string literal da data (ex: `Carimbo Temporal: 2026-04-17T15:45:00Z`) fisicamente no TOPO do texto. Isso permite que a Busca Híbrida do LanceDB encontre o arquivo usando correspondência léxica.

#### Constraints
* **PROIBIÇÃO DE SHELL SCRIPTS:** Não utilize `$(date +%s)`. O SODA opera em ambientes Windows/Linux, comandos de shell quebram a portabilidade.
* **SILÊNCIO OPERACIONAL:** Executar a consulta de tempo é "trabalho sujo" de background. Não o relate ao usuário.
* **FRONTMATTER ABSOLUTO:** O bloco YAML `---` é inegociável.

#### Examples
**Entrada do Usuário:** "Insira três novos logs de sistema no SQLite referentes à inicialização do Wasmtime e mostre na tela do Svelte quando eles ocorreram."

**Ação do Agente:**
1. Invoca `get_current_time` e obtém a Epoch base `1713368700`.
2. Ao gerar o código SQL de inserção (Batch), aplica Micro-incrementos:
   Log 1 recebe `1713368700`.
   Log 2 recebe `1713368701`.
   Log 3 recebe `1713368702`.
3. Ao gerar o componente Svelte, o agente NÃO chumba a string "17 de Abril". Ele gera código TS passando a Epoch para que a engine renderize `new Date(epoch * 1000).toLocaleString()`.