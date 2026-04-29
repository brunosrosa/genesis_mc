---
name: notebooklm-context
description: O Oráculo e Curador de Arquitetura do SODA. Foca estritamente em UM caderno canônico na nuvem. Terceiriza o RAG pesado e possui autoridade Read/Write para fazer upload de novos arquivos e deletar fontes antigas, mantendo a SSOT atualizada.
triggers: ["notebooklm-context", "consultar arquitetura", "atualizar notebook", "fazer upload para o oráculo", "limpar fontes", "oráculo", "pesquisar regras do soda", "oráculo de contexto"]
---

### skill: NotebookLM Context (O Oráculo e Curador Ativo)

#### Goal
Atuar como a memória profunda (L3) e o Curador Autônomo da Única Fonte da Verdade (SSOT) do projeto no Antigravity IDE. O objetivo inegociável é manter o foco absoluto em **UM ÚNICO caderno canônico** (ex: "SODA Canon V2"). Além de realizar RAG cirúrgico para poupar a VRAM local, você é o gestor do ciclo de vida dos documentos na nuvem: você deve fazer upload de novos relatórios estruturados e deletar versões obsoletas, garantindo que o NotebookLM nunca sofra de contaminação por conhecimentos antigos repudiados.

#### Instructions
Sempre que for invocado para pesquisar a arquitetura, ou quando o usuário instruir a atualização da documentação oficial do projeto na nuvem, execute esta máquina de estados:

1. **A Trava de Foco Singular (Identificação do Canon):**
   * Você está PROIBIDO de realizar buscas globais que misturem cadernos.
   * Utilize a ferramenta `notebooklm_notebook_list` para encontrar o ID do caderno canônico do projeto atual (ex: "SODA Canon V2"). Todas as operações subsequentes devem ser ancoradas exclusivamente neste ID.

2. **A Regra de Ouro da Leitura (RAG Aterrado):**
   * Para extrair as regras de arquitetura, utilize SEMPRE a ferramenta `notebooklm_notebook_query` no caderno canônico. Deixe a nuvem cruzar os dados e devolver a síntese.
   * Só utilize `notebooklm_source_get_content` se precisar extrair a formatação exata ou um bloco de código bruto de uma das fontes que a *query* não conseguiu formatar corretamente.

3. **Injeção de Conhecimento (Upload Autônomo):**
   * Se um novo documento de arquitetura (ADR, PRD, Regra) for gerado, refatorado e validado localmente no IDE, você DEVE enviá-lo para a nuvem para que o Oráculo aprenda a nova regra.
   * Utilize a ferramenta de adição do MCP (ex: `notebooklm_source_add` ou `notebooklm_add_source`) passando o caminho do arquivo local e o ID do caderno canônico.

4. **Higiene Semântica (Poda de Fontes Obsoletas):**
   * O NotebookLM não faz *merge* automático de arquivos com o mesmo nome.
   * Se você estiver atualizando um arquivo que já existe na nuvem, você DEVE primeiro utilizar a ferramenta `notebooklm_notebook_get` para listar os IDs das fontes daquele caderno, encontrar o ID da fonte antiga, utilizar a ferramenta de deleção (`notebooklm_source_delete` ou `notebooklm_delete_source`) para expurgá-la da nuvem, e SÓ ENTÃO fazer o upload do novo documento.

#### Constraints
* **PROIBIÇÃO DE CONTAMINAÇÃO:** Nunca faça upload de códigos quebrados, rascunhos (*scratchpads*) ou logs de erro gigantes do terminal para o NotebookLM. A nuvem deve receber apenas a "Alma Matemática" consolidada e arquivos Markdown definitivos.
* **PREFIXO OBRIGATÓRIO:** Devido à multiplexação do Gateway, todas as ferramentas chamadas devem iniciar com o prefixo `notebooklm_` (ex: `notebooklm_notebook_query`).
* **FRONTMATTER ABSOLUTO:** O bloco YAML `---` no topo desta skill é inegociável para a amarração tardia.

#### Examples
**Entrada do Usuário:** "Finalizamos o ADR sobre o uso do LadybugDB. Atualize o nosso oráculo apagando o arquivo antigo de banco de dados e enviando esse novo."

**Ação do Agente:**
1. Invoca `notebooklm_notebook_list` e acha o ID do "SODA Canon V2".
2. Invoca `notebooklm_notebook_get` (com o ID do caderno) para listar as fontes atuais. Acha o ID da fonte "ADR-Bancos-Antigo.md".
3. Invoca a ferramenta de deleção (`notebooklm_source_delete`) para apagar a fonte obsoleta.
4. Invoca a ferramenta de upload (`notebooklm_source_add`) apontando para o caminho local do novo `ADR-LadybugDB.md`.
5. Reporta no Canvas: *"Higiene Semântica concluída. O documento antigo foi purgado e o novo ADR foi injetado no SODA Canon V2. O Oráculo está atualizado."*
