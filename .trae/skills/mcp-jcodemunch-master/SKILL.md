---
name: mcp-jcodemunch-master
description: O manual inegociável para leitura cirúrgica de código (AST) no SODA. Resolve o bloqueio L7 do Gateway (Firewall). Proíbe a leitura de arquivos por força bruta. Força o fatiamento O(1) usando 'get_file_outline' e 'get_symbol_source' para blindar a VRAM de 6GB contra o Context Rot.
triggers: ["mcp-jcodemunch-master", "ler código", "buscar função", "analisar classe", "jcodemunch", "explorar código", "AST", "extrair lógica"]
---

### skill: MCP JCodeMunch Master (Leitura Cirúrgica O(1) e Bypass L7 V6.0)

#### Goal
Atuar como o "Bisturi Semântico" do Antigravity IDE. O seu objetivo inegociável é proteger a janela de contexto e a VRAM local (limite de 6GB) contra o *Context Rot* (amnésia induzida por textos gigantes). Você está TERMINANTEMENTE PROIBIDO de ler arquivos inteiros por força bruta. Você deve navegar na Árvore de Sintaxe Abstrata (AST) em tempo constante $\mathcal{O}(1)$ e aplicar táticas de *Bypass* na nomenclatura L7 para não ser bloqueado pelo Gateway do SODA.

#### Instructions
Sempre que precisar ler código local, investigar a origem de um bug ou extrair lógica para canibalização, execute ESTRITAMENTE esta máquina de estados:

1. **Firewall Compliance (O Bypass L7):**
   * Você DEVE usar OBRIGATORIAMENTE as ferramentas exatas permitidas pelo Gateway: `index_folder`, `index_repo`, `get_file_tree`, `get_file_outline`, `get_symbol_source`, `search_symbols`, `get_file_content`.
   * **Fail-Closed:** Se o ambiente listar as ferramentas com prefixos do multiplexador (ex: `jcodemunch_get_file_outline`), **IGNORE O PREFIXO** na sua chamada. A válvula CEL bloqueia qualquer nome que não corresponda à lista exata acima.

2. **A Lei da Leitura em O(1) (Fobia de Força Bruta):**
   * É PROIBIDO iniciar a investigação com `get_file_content` se você não souber o tamanho do arquivo. Isso asfixiará a VRAM local.
   * **Passo A:** Use `get_file_outline` no arquivo suspeito. A engine *tree-sitter* retornará apenas a "Alma Matemática" (as assinaturas de funções, structs, traits e dependências), poupando 95% dos tokens.
   * **Passo B:** Identifique o ID exato do símbolo (função) quebrado na resposta do outline e use `get_symbol_source` para extrair EXCLUSIVAMENTE o bloco de código defeituoso.

3. **Paradigma NextPlaid e Poda de RAM:**
   * Após extrair o bloco de código, retenha apenas a lógica operacional (matrizes, iterações, chamadas AVX2).
   * Dê o comando de "Context Purge" mental para esquecer quaisquer metadados frívolos lidos durante a exploração que não sejam essenciais para a correção do bug.

4. **Tratamento de Ponto Cego (Indexação Tardia):**
   * Se a busca ou a extração retornar vazio, não alucine o código. É provável que o diretório não esteja indexado.
   * Execute `index_folder` no diretório de trabalho alvo antes de tentar a extração de AST novamente.

#### Constraints
* **PROIBIÇÃO DE ADIVINHAÇÃO:** Nunca invente um `symbol_id`. Ele deve ser extraído deterministicamente do `get_file_outline`.
* **SILÊNCIO OPERACIONAL:** O *JCodeMunch* atua em background. Não polua o Canvas descrevendo passo a passo como você extraiu a AST. Apenas informe ao Arquiteto o diagnóstico final.
* **FRONTMATTER ABSOLUTO:** O bloco YAML `---` contido no topo desta skill é a fundação inegociável do roteamento.

#### Examples
**Entrada do Usuário:** "SODA, dá uma olhada na função de roteamento do ParetoBandit no arquivo `router.rs` e extrai a lógica para eu ver."
**Ação do Agente:**
1. Invoca estritamente `get_file_outline(path: "router.rs")` (ignorando prefixos letais do Gateway).
2. O servidor retorna a árvore AST do arquivo de 2.000 linhas usando apenas 150 tokens. O agente identifica o símbolo da função de roteamento.
3. Invoca `get_symbol_source(symbol_id: "X")` e extrai apenas as 30 linhas pertinentes.
4. Raciocina sobre a lógica, descarta o resto da árvore e retorna no Canvas: *"Lógica extraída em O(1) via AST. A VRAM da máquina host foi preservada."*