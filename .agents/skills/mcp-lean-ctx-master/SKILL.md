---
name: mcp-lean-ctx-master
description: O Navegador Token-Aware e Escudo Anti-Context Rot do SODA. Impõe compressão LEAN (Lossless 48%). Bane comandos nativos crus. Exige Sandboxing (Landlock/AppContainer) para qualquer 'ctx_shell'. Impõe a 'Poda Ativa de Cache' para evitar vazamento de RAM e blinda as edições ('ctx_edit') com Hard Links (snapsafe) e Mutex atômico contra Corrupção Silenciosa de Dados (SDC).
triggers: ["mcp-lean-ctx-master", "ler arquivo", "lean-ctx", "buscar no texto", "listar diretório", "ctx_read", "context engineering"]
---

### skill: MCP LEAN Context Master (Engenharia LEAN e Sandboxing V6.0)

#### Goal
Atuar como o Escudo Anti-Context Rot e o Navegador Supremo do Antigravity IDE. O seu objetivo inegociável é proteger os 6GB de VRAM e o host local. Você está **TERMINANTEMENTE PROIBIDO** de utilizar comandos nativos crus (`cat`, `grep`, `ls`). Toda a varredura deve passar pelas ferramentas do `lean-ctx` no formato LEAN. Além da economia de 48% de tokens, você DEVE enjaular execuções de shell (Landlock), aplicar a higiene de RAM (poda de cache) e proteger as edições contra a Corrupção Silenciosa de Dados (SDC) usando Mutex e *Hard Links* $\mathcal{O}(1)$.

#### Instructions
Sempre que precisar investigar arquivos locais, explorar diretórios ou vasculhar logs, engatilhe esta Máquina de Estados:

1. **A Lei da Compressão LEAN (Morte aos comandos puros):**
   * PROIBIDO usar `cat` ou `grep`.
   * Use OBRIGATORIAMENTE `ctx_read(path, mode)` para leituras e `ctx_search(pattern, path)` para buscas. Os dados retornarão na notação LEAN altamente comprimida.
   * PROIBIDO usar `ls`. Use `ctx_tree(path, depth)`.

2. **A Guilhotina de Shell (Prevenção Anti-RCE):**
   * Se precisar invocar processos no terminal usando `ctx_shell(command)`, você opera sob restrição de privilégio mínimo.
   * **Lei do Sandboxing:** O comando DEVE rodar sob contenção do Kernel Host (Landlock no Linux ou AppContainer/LPAC no Windows). O comando gerado não pode ter acesso global de escrita no disco nem portas de rede abertas sem autorização explícita (Fail-Closed).

3. **Higiene de RAM (A Lei do Descarte):**
   * A leitura via `lean-ctx` faz cache agressivo. Se você abrir múltiplos arquivos durante o *Ralph Loop*, a RAM do host sangrará.
   * Assim que a utilidade de um arquivo lido for esgotada (ex: você percebeu que o bug não está no `header.rs`), dê o comando lógico de **Poda de Cache** no servidor e limpe seu próprio *context window* mental.

4. **Blindagem de Edição (Anti-SDC):**
   * A ferramenta `ctx_edit(path, old_string, new_string)` **NÃO PODE** ser invocada cegamente.
   * Antes de acionar qualquer edição estrutural baseada no contexto LEAN que você leu, garanta que o arquivo está protegido por um *Hard Link* em $\mathcal{O}(1)$ (`snapsafe`).
   * A edição deve respeitar as travas de concorrência (Mutex do Tokio). Falhas na gravação devem preservar o *inode* original intacto.

#### Constraints
* **PROIBIÇÃO DE PARÂMETROS NATIVOS:** É EXPRESSAMENTE PROIBIDO injetar parâmetros nativos (StartLine, AbsolutePath, EndLine, etc.) no `lean_ctx_read` (ou `ctx_read`). A assinatura da ferramenta exige estritamente `path` (caminho do arquivo) e `mode` (ex: 'full', 'signatures', etc.). Valide o schema da ferramenta antes de chamar.
* **PROIBIÇÃO DO MODO TASK ANTES DE EDIÇÃO:** Nunca use `mode="task"` para arquivos que planeja modificar; ele embaralha o arquivo estruturalmente.
* **INVALIDAÇÃO DE CACHE (fresh: true):** Se um arquivo for modificado em disco por compiladores ou testes externos, force a re-leitura usando `fresh: true`.
* **UNICIDADE NO CTX_EDIT:** A string `old_string` no `ctx_edit` deve conter 2-3 linhas de contexto adjacentes para garantir correspondência única no arquivo.
* **LEITURAS FATIADAS (lines:N-M):** Para arquivos grandes, prefira `mode="lines:N-M"` em vez de `full` para economizar tokens.
* **CTX_SHELL APENAS DE LEITURA:** Use `ctx_shell` apenas para comandos diagnósticos passivos; nunca para mutação de arquivos (como `sed`/`awk`).
* **O FORMATO LEAN É INEGOCIÁVEL:** Rejeite TOON ou JSON denso para leituras extensas. O formato LEAN garante 93% de precisão de Recall consumindo 48% menos tokens.
* **SEM ALUCINAÇÕES DE PATH:** Use `ctx_tree` antes de `ctx_read`.
* **FRONTMATTER ABSOLUTO:** O bloco YAML `---` contido no topo desta skill é a fundação do roteamento O(1).

#### Examples
**Entrada do Usuário:** "SODA, busca onde está declarada a função de roteamento IPC, lê a lógica dela e aplica a correção."
**Ação do Agente:**
1. Aborta o instinto de usar `grep`. Invoca `ctx_search(pattern: "transmit_ipc", path: "src/")`.
2. Localiza o arquivo e usa `ctx_read`. O servidor devolve a função em formato LEAN comprimido.
3. O agente descobre o erro. Imediatamente faz a *Poda de Cache* dos arquivos não relacionados que abriu no caminho.
4. Aciona a trava O(1) de `snapsafe` sobre o arquivo alvo.
5. Invoca `ctx_edit` para corrigir a assinatura.
6. Reporta: *"-> Busca e edição concluídas via notação LEAN em O(1). Cache RAM podado e arquivo blindado via snapsafe."*