---
name: notebooklm-context
description: O Oráculo e Curador Ativo de Arquitetura do SOULS. Foca em UM caderno canônico na nuvem (SSOT). Respeita rigorosamente a Válvula CEL do Gateway, aplica extração bruta (source_get_content) contra alucinações e garante Higiene Semântica.
triggers: ["notebooklm-context", "consultar arquitetura", "atualizar notebook", "fazer upload para o oráculo", "limpar fontes", "oráculo", "pesquisar regras do souls", "oráculo de contexto"]
---

### skill: NotebookLM Context (O Oráculo e Curador Ativo V4.0)

#### Goal
Atuar como a interface da Memória Semântica Profunda (L3) e como Curador Autônomo da Única Fonte da Verdade (SSOT) do SOULS no Google NotebookLM. O objetivo inegociável é manter o foco absoluto em **UM ÚNICO caderno canônico**. Para evitar bloqueios do Firewall L7 do AgentGateway e alucinações endêmicas do LLM da nuvem, você deve usar a nomenclatura exata de ferramentas e priorizar a extração de texto bruto para avaliação local. A ferramenta atua como um *Sidecar Efêmero*: extraia, valide e abandone a conexão RPC para blindar a privacidade local.

#### Instructions
Sempre que for invocado para pesquisar fundamentos da arquitetura ou atualizar a nuvem, OBRIGATORIAMENTE execute esta máquina de estados:

1. **Validação de Autenticação e Firewall Compliance:**
   * Inicie o contato utilizando **EXATAMENTE** a ferramenta `notebook_list` para encontrar o ID do caderno canônico. 
   * **Fail-Closed:** Se retornar falha de autenticação ou *timeout*, ABORTE IMEDIATAMENTE. Não adivinhe regras de arquitetura se a nuvem cair. Notifique o usuário no Canvas: *"Sessão do NotebookLM expirada. Execute a autenticação da CLI localmente."*

2. **A Trava de Foco Singular e Higiene Semântica:**
   * Todas as operações subsequentes devem ser ancoradas exclusivamente no `notebook_id` validado. 
   * **A Morte da Duplicação:** O NotebookLM não faz *merge*. Se for instruído a atualizar uma regra (ex: um ADR), liste as fontes usando `notebook_get`. Localize o ID do arquivo antigo e aniquile-o via `source_delete` **ANTES** de inserir a nova versão via `source_add`.

3. **Extração Anti-Alucinação (O Bypass do RAG):**
   * Em obediência às leis de restrição de memória, evite delegar o entendimento de códigos longos, logs técnicos e regras pesadas para o processamento abstrato da nuvem.
   * Ao buscar documentos técnicos, evite a interpretação generativa (`notebook_query`). Priorize **SEMPRE** o uso de `source_get_content` para trazer o "texto fonte bruto" para a memória de curto prazo do Antigravity IDE. Faça o raciocínio determinístico e analítico no seu ambiente *bare-metal* local.

4. **Isolamento de Sidecar Efêmero:**
   * Trate este MCP como radioativo. Assim que a extração ou a injeção (upload/delete) for concluída e matematicamente validada, suspenda o uso de ferramentas relacionadas ao NotebookLM no mesmo turno de pensamento para evitar vazamento de contexto RPC.

#### Constraints
* **PROIBIÇÃO DE PREFIXOS ALUCINADOS:** O Gateway SOULS possui uma trava CEL rigorosa. É EXPRESSAMENTE PROIBIDO usar prefixos inventados como `notebooklm_query`. As ferramentas válidas são unicamente: `notebook_query`, `notebook_list`, `notebook_get`, `notebook_create`, `notebook_delete`, `source_get_content`, `source_add`, `source_delete`, `add_source`, `delete_source`.
* **PROIBIÇÃO DE LIXO TÓXICO:** Nunca faça upload de *scratchpads* temporários ou logs de erros do *Ralph Loop*. Suba apenas a "Alma Matemática" cristalizada para o oráculo.
* **FRONTMATTER ABSOLUTO:** O bloco YAML `---` contido no topo desta skill é a fundação do roteamento de *Amarração Tardia*.

#### Examples
**Entrada do Usuário:** "SOULS, aprovamos o ADR do LadybugDB. Atualize nosso Oráculo apagando o ADR antigo de grafos e subindo este novo para que não haja contradições."

**Ação do Agente:**
1. Testa a conexão com `notebook_list` (sem o prefixo errado). (Sessão OK).
2. Isola o ID do caderno canônico.
3. Invoca `notebook_get` e varre a array de fontes. Identifica o ID da fonte "ADR_Grafos_KuzuDB_Obsoleto.md".
4. Executa a faxina: `source_delete(notebook_id, source_id)`.
5. Injeta o novo estado da arte: `source_add(notebook_id, file_path: "docs/decisions/adrs/ADR_LadybugDB.md")`.
6. Retorna silenciosamente no Canvas: *-> Higiene Semântica concluída. ADR antigo expurgado e a nova SSOT do LadybugDB foi cimentada na nuvem. Fechando conexão RPC.*
