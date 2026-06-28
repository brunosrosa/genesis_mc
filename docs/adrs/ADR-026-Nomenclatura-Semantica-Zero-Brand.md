# ADR-026-Nomenclatura-Semantica-Zero-Brand

## Status
Aceito (Ativo e Inegociável)

## Contexto
A integração de ferramentas via Model Context Protocol (MCP) com AI-IDEs (Cursor, Trae, Antigravity) revelou uma falha severa de obesidade semântica e "Tool Bloat". Clientes MCP possuem limites rígidos de caracteres para a exibição de ferramentas (frequentemente < 60 caracteres) e concatenam automaticamente o nome do servidor com o nome da ferramenta (ex: `soda-native-ast_soda_duckgo_search`). 

Esse lixo nominal causa três problemas letais:
1. **Quebra de Protocolo (Fail-Closed Indesejado):** Nomes longos estouram o limite de exibição e parsing do cliente.
2. **Hemorragia FinOps (Token Bloat):** Cada caractere inútil como `tool-`, `mcp-` ou `soda-` consome tokens preciosos a cada iteração de Raciocínio (Chain of Thought) do LLM.
3. **Context Drift (Acoplamento de Marca):** Nomear ferramentas com bases em fornecedores temporários (ex: `duckduckgo` ou `jcodemunch`) acopla a mente do Agente à ferramenta. Se o backend for trocado para `brave_search` ou `tree_sitter` nativo, a assinatura quebra e as *Skills* do agente sofrem de amnésia.

## Decisão
Fica decretada a **Lei da Nomenclatura Semântica e Zero-Brand** para todo o ecossistema SODA/Souls MC:

1. **Servidor Atômico (Zero Redundância):** O nome do Agent Gateway exportado para os clientes MCP será estritamente **`souls`**. O prefixo `soda-agent-gateway` está obsoleto.
2. **Agnosticismo de Marca (Zero-Brand):** É expressamente proibido nomear ferramentas com marcas, bibliotecas de terceiros ou o próprio nome do projeto. A ferramenta deve refletir a *Ação Matemática*, não o executor.
   * *Incorreto:* `soda_duckgo_search`, `soda_get_ast`.
   * *Correto:* `web_search`, `repo_ast`.
3. **Topologia de Nomenclatura (`<dominio>_<acao>`):** Nomes de ferramentas devem ter no máximo duas ou três palavras, sempre no formato `dominio_acao`.
   * **Contexto/Memória:** `ctx_read`, `ctx_tree`, `ctx_workflow`.
   * **Web:** `web_search`, `web_fetch`.
   * **Código/AST:** `repo_ast`, `repo_meta`.
   * **Sistema:** `sys_time`, `db_query`.
4. **Guilhotina de Pleonasmos:** É terminantemente proibido o uso de prefixos explicativos como `tool_`, `mcp_` ou `action_`. O protocolo JSON-RPC (`tools/call`) já garante essa semântica.

## Consequências
* **Resiliência de Roteamento:** As assinaturas exportadas (ex: `souls_web_search`) terão em média 16 caracteres, operando com folga absoluta perante os limites das IDEs.
* **FinOps e Clareza Cognitiva:** A Lente de orquestração gastará menos tokens para ler e invocar ferramentas, reduzindo a fadiga do LLM e acelerando a inferência.
* **Imunidade a Migrações:** O backend em Rust pode trocar o motor de busca ou o parser AST à vontade sem que a inteligência do Agente perceba a mudança, garantindo isolamento total (Interface vs Implementação).

## Restrições Bare-Metal e Blast Radius
* **Válvula Inteligente L7 (`gateway-config.yaml`):** As expressões de bloqueio em CEL (`mcp.tool.name.matches`) devem ser reescritas para refletir os nomes atômicos.
* **Memória das Skills (`SKILL.md`):** Todos os prompts e gatilhos na pasta `.agents/skills/` devem sofrer *Search and Replace* cirúrgico para as novas assinaturas, sob risco de quebrar o roteamento das IAs.