---
name: mcp-google-sheets-master
description: O Oráculo do SOULS e leitor do Blueprint (SSOT). Resolve o bloqueio L7 do Gateway. Bane a extração visual ('include_grid_data': false). Impõe o agrupamento em memória global para evadir o limite de 60 RPM (Erro 503). Exige alinhamento com Decodificação Restrita (llguidance) e orquestra a inserção atômica destrutiva nas 4 abas canônicas (MASTER_SOLUTIONS, SOULS_GRAPH_TOPOLOGY, ACTION_MATRIX, QUARANTINE_RADAR).
triggers: ["mcp-google-sheets-master", "ler planilha", "consultar blueprint", "verificar adr", "get_sheet_data", "google sheets", "ssot", "atualizar matriz"]
---

### skill: MCP Google Sheets Master (O Oráculo e Disjuntor FinOps V6.0)

#### Goal
Atuar como a interface cirúrgica para a nossa Única Fonte de Verdade (SSOT) no Google Sheets. Você é a ponte do *Spec-Driven Development* (SDD). O seu objetivo inegociável é contornar o Firewall L7, paginar leituras em $\mathcal{O}(1)$ via Notação A1 para salvar VRAM, e PROTEGER a cota de 60 requisições por minuto da API do Google. Você deve agrupar toda intenção de leitura/escrita em memória global e dispará-la através de lotes atômicos consolidados, distribuindo os dados estruturados nas quatro abas sistêmicas do projeto.

#### Instructions
Sempre que for instruído a ler heurísticas, extrair ADRs ou atualizar o Blueprint do projeto SOULS, engatilhe esta máquina de estados:

1. **Firewall Compliance (O Bypass L7):**
   * **Lei do Fail-Closed:** O multiplexador usa `mcp-google-sheets_`. IGNORE O PREFIXO. Use EXATAMENTE: `get_sheet_data`, `batch_update_cells` ou `add_rows`. 

2. **O Disjuntor de Quota (A Evasão do Erro 503):**
   * É TERMINANTEMENTE PROIBIDO realizar múltiplas chamadas à API para atualizar ou ler linhas avulsas consecutivamente (limite letal de 60 RPM do Google) [1].
   * Reteia as saídas JSON na sua memória global (Contexto) e invoque o `batch_update_cells` enviando todo o *payload* empacotado em uma única tacada atômica [1].

3. **Arquitetura Multi-Aba (O Desmembramento):**
   * O Sheets não é uma tabela plana, é um banco de dados relacional visual. Ao enviar dados, distribua as fatias nas abas correspondentes [2]:
     * `MASTER_SOLUTIONS`: A matriz principal de 85 colunas.
     * `SOULS_GRAPH_TOPOLOGY`: Para dependências e stack_base.
     * `ACTION_MATRIX`: Apenas para itens de `acao_de_canibalizacao` que geram tickets de código em Rust.
     * `QUARANTINE_RADAR`: Para componentes com alto risco ético ou de design (design_misuse_risk). Estes exigirão HITL manual futuro [2].

4. **Blindagem de VRAM e Formatação:**
   * Na extração (`get_sheet_data`): Force SEMPRE `include_grid_data: False` para não asfixiar a sua VRAM com códigos de cores e bordas HTML. Use paginação A1 curta.
   * Na gravação (`batch_update_cells`): O payload deve OBRIGATORIAMENTE obedecer ao schema restrito validado (Constrained Decoding via `llguidance`/Pydantic) [4]. NUNCA envie arrays mal formatados ou Enums alucinados.

#### Constraints
* **ZERO ALUCINAÇÃO TABULAR:** Respeite a taxonomia das 45 colunas [5]. Gravar dados fora dos Enums (ex: inventar "Medium" em vez de um inteiro ou constante aprovada) ativa o Kill-Switch.
* **SEM DEPENDÊNCIA DE UI:** O Sheets dita as regras do sistema, não questione a planilha.
* **FRONTMATTER ABSOLUTO:** O bloco YAML `---` contido no topo é a âncora inegociável do roteamento L7.

#### Examples
**Entrada do Usuário:** "SOULS, atualize a planilha com as 5 análises de repórter que fizemos e separe quem vai pra quarentena."
**Ação do Agente:**
1. Aborta o instinto de usar loops `add_rows` consecutivos para evitar o Erro 503 de *Rate Limit*.
2. Empacota os 5 JSONs na memória global local, garantindo que passaram pela decodificação restrita (`llguidance`).
3. Estrutura o payload do `batch_update_cells` fatiando os dados: joga a matriz primária na `MASTER_SOLUTIONS`, os algoritmos perigosos na `QUARANTINE_RADAR` e as tarefas braçais de Rust na `ACTION_MATRIX`.
4. Dispara a chamada atômica O(1) para o Gateway.
5. Retorna no Canvas: *"-> SSOT atualizada via batchUpdate atômico. Disjuntor 60 RPM respeitado. Componentes perigosos movidos para a Aba de Quarentena."*