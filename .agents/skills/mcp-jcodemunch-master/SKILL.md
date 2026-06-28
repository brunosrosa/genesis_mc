---
name: mcp-jcodemunch-master
description: Manual de compatibilidade para leitura cirúrgica de código (AST) no SODA. Reencaminha a disciplina de fatiamento O(1) para o poder intrínseco `repo_ast`, blindando a VRAM de 6GB contra Context Rot sem depender de sidecar legado.
triggers: ["mcp-jcodemunch-master", "ler código", "buscar função", "analisar classe", "explorar código", "AST", "extrair lógica", "repo_ast"]
---

### skill: MCP JCodeMunch Master (Compat Layer para AST Nativo V7.0)

#### Goal
Atuar como o "Bisturi Semântico" do Antigravity IDE. O seu objetivo inegociável é proteger a janela de contexto e a VRAM local (limite de 6GB) contra o *Context Rot* (amnésia induzida por textos gigantes). Você está TERMINANTEMENTE PROIBIDO de ler arquivos inteiros por força bruta. Você deve navegar na Árvore de Sintaxe Abstrata (AST) em tempo constante $\mathcal{O}(1)$ usando o poder intrínseco `repo_ast`, sem depender do sidecar AST histórico.

#### Instructions
Sempre que precisar ler código local, investigar a origem de um bug ou extrair lógica para canibalização, execute ESTRITAMENTE esta máquina de estados:

1. **Firewall Compliance Bare-Metal:**
   * Você DEVE usar OBRIGATORIAMENTE `repo_ast` quando a missão for obter visão estrutural de repositórios ou diretórios.
   * O sidecar AST histórico foi aposentado. Qualquer instrução antiga que dependa dele deve ser reinterpretada como chamada ao poder intrínseco `repo_ast`.

2. **A Lei da Leitura em O(1) (Fobia de Força Bruta):**
   * É PROIBIDO iniciar a investigação lendo arquivos inteiros sem prova de necessidade. Isso asfixiará a VRAM local.
   * **Passo A:** Use `repo_ast` no diretório ou repositório suspeito. O parser nativo retornará apenas a "Alma Matemática" (outline estrutural, mapa de arquitetura e relatório de saúde), poupando contexto.
   * **Passo B:** Só após a visão estrutural, aproxime-se do arquivo exato com ferramentas de leitura local enxutas.

3. **Paradigma NextPlaid e Poda de RAM:**
   * Após extrair o bloco de código, retenha apenas a lógica operacional (matrizes, iterações, chamadas AVX2).
   * Dê o comando de "Context Purge" mental para esquecer quaisquer metadados frívolos lidos durante a exploração que não sejam essenciais para a correção do bug.

4. **Tratamento de Ponto Cego (Fail-Closed):**
   * Se `repo_ast` retornar falha ou estrutura vazia, não alucine o código.
   * Recue, valide o caminho do repositório e só então peça nova extração estrutural.

#### Constraints
* **PROIBIÇÃO DE ADIVINHAÇÃO:** Nunca invente um `symbol_id`. Ele deve ser extraído deterministicamente do `get_file_outline`.
* **SILÊNCIO OPERACIONAL:** O *JCodeMunch* atua em background. Não polua o Canvas descrevendo passo a passo como você extraiu a AST. Apenas informe ao Arquiteto o diagnóstico final.
* **FRONTMATTER ABSOLUTO:** O bloco YAML `---` contido no topo desta skill é a fundação inegociável do roteamento.

#### Examples
**Entrada do Usuário:** "SODA, dá uma olhada na função de roteamento do ParetoBandit no arquivo `router.rs` e extrai a lógica para eu ver."
**Ação do Agente:**
1. Invoca `repo_ast` no diretório do repositório.
2. O servidor Rust retorna a visão estrutural do arquivo de 2.000 linhas sem despejar o conteúdo bruto. O agente identifica a função de roteamento a partir do outline.
3. Lê apenas o arquivo ou símbolo estritamente necessário.
4. Raciocina sobre a lógica, descarta o resto da árvore e retorna no Canvas: *"Lógica extraída em O(1) via AST nativa. A VRAM da máquina host foi preservada."*

