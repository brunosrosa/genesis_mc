---
name: mcp-jcodemunch-master
description: O manual de instrução inegociável para leitura cirúrgica de código no Antigravity IDE. Proíbe a leitura de arquivos por força bruta. Força a extração da Árvore de Sintaxe Abstrata (AST) em O(1) usando jcodemunch restrito.
triggers: ["mcp-jcodemunch-master", "ler código", "buscar função", "analisar classe", "jcodemunch", "explorar código", "AST", "extrair lógica"]
---

### skill: MCP JCodeMunch Master (Leitura Cirúrgica em O(1))

#### Goal
Proteger a janela de contexto do Antigravity IDE e a VRAM da máquina host (limite absoluto de 6GB) contra o *Context Rot* induzido pela leitura massiva de arquivos de código fonte. O objetivo inegociável é doutrinar o agente a operar sob a filosofia *Zero-Bloat*: utilizar ferramentas de Sistema Operacional (`lean-ctx`) para descobrir onde o arquivo está, e o servidor MCP `jcodemunch` EXCLUSIVAMENTE como um "bisturi" semântico para extrair a Árvore de Sintaxe Abstrata (AST) em tempo constante $\mathcal{O}(1)$.

#### Instructions
Sempre que precisar ler, refatorar ou auditar arquivos locais de código-fonte no seu Ambiente de Desenvolvimento, você DEVE operar sob o protocolo de "Extração Cirúrgica" em 4 Fases:

1. **Reconhecimento de Terreno (O Binóculo `lean-ctx`):**
   * Você está PROIBIDO de usar as ferramentas de listagem de arquivos do jcodemunch (ex: `jcodemunch_get_file_tree`) para varredura básica.
   * Invoque OBRIGATORIAMENTE as ferramentas `ctx_tree` ou `ctx_search` (do pacote `lean-ctx`) para encontrar o caminho do arquivo alvo de forma instantânea e leve.

2. **Indexação Escopada (A Preparação):**
   * PROIBIDO usar `jcodemunch_index_repo` na raiz do projeto (causa colapso de memória local).
   * Acione OBRIGATORIAMENTE a ferramenta `jcodemunch_index_folder` restrita APENAS à subpasta onde o arquivo alvo reside.

3. **Mapeamento Estrutural AST (O Escaneamento):**
   * Não invoque `cat`, `ctx_read` ou `jcodemunch_get_file_content` se o arquivo for maior que 100 linhas.
   * Chame a ferramenta `jcodemunch_get_file_outline` passando o caminho do arquivo. Isso retornará os metadados brutos (nomes de classes, assinaturas de funções e argumentos), custando uma fração irrisória da VRAM.

4. **Extração O(1) por Byte-Offset (O Bisturi):**
   * Leia o Outline retornado e copie o identificador exato (`symbol_id`) da função que você precisa.
   * Acione a ferramenta `jcodemunch_get_symbol_source` fornecendo este `symbol_ids[]`. O MCP retornará EXCLUSIVAMENTE o bloco de código exato daquela função.

#### Constraints
* **INTERDIÇÃO DE FORÇA BRUTA:** O uso de comandos como `cat`, `type` ou a invocação de `jcodemunch_get_file_content` para ler arquivos gigantes sem cortes é uma violação severa da arquitetura da IDE.
* **SEM ALUCINAÇÃO DE IDs:** Você é proibido de tentar adivinhar o `symbol_id`. O uso da ferramenta `get_file_outline` antes da extração final é obrigatório.
* **FRONTMATTER ABSOLUTO:** O bloco YAML `---` no topo desta skill é inegociável para a amarração tardia (Late-Binding) do gateway.

#### Examples
**Entrada do Usuário:** "SODA, preciso que você conserte o buffer da função de despacho no `ipc_router.rs`."

**Ação do Agente:**
1. O Agente NÃO lê o arquivo inteiro.
2. Invoca `ctx_search` para confirmar o caminho do arquivo.
3. Invoca `jcodemunch_index_folder(folder_path: "src/ipc/")` para indexar a pasta específica, evitando OOM.
4. Invoca `jcodemunch_get_file_outline(file_path: "src/ipc/ipc_router.rs")`. A ferramenta devolve as assinaturas, revelando o `symbol_id` da função `fn dispatch_binary_buffer`.
5. Invoca `jcodemunch_get_symbol_source(symbol_ids: ["dispatch_binary_buffer"])`. O Agente recebe apenas as linhas exatas daquela função.
6. O Agente elabora a correção baseada unicamente no contexto extraído.