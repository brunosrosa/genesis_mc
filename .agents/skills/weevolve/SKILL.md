---
name: weevolve
description: O Córtex de Aprendizado Relacional do Antigravity IDE. Extrai a "Alma Matemática" de bugs resolvidos e injeta as heurísticas no SQLite local da IDE. Erradica o Context Rot do agente desenvolvedor sem criar arquivos de log massivos.
triggers: ["weevolve", "salvar heurística", "bug resolvido", "aprendizado contínuo", "extrair padrão", "documentar erro"]
---

### skill: WeEvolve v2.1 (O Córtex de Aprendizado do Ambiente de Desenvolvimento)

#### Goal
Erradicar a amnésia de sessão (Context Rot) e a dívida de VRAM causada pelo acúmulo de arquivos de log Markdown durante o desenvolvimento no Antigravity IDE. O objetivo é abstrair a resolução de um bug de código, erro de compilação ou falha de arquitetura em uma heurística matemática pura. Em vez de registrar o erro em textos longos, você deve estruturá-lo de forma relacional para ser ingerido pelo banco de dados do seu próprio ambiente de desenvolvimento local (SQLite com `sqlite-vec`), consolidando o aprendizado instantaneamente.

#### Instructions
Sempre que você solucionar um problema complexo no código, corrigir uma falha do compilador Rust, ou receber a ordem de "aprender com isso", você DEVE executar estas ações em ordem estrita:

1. **Destilação da Alma Matemática:** Isole a falha das variáveis específicas e nomes de arquivos temporários. Identifique o princípio arquitetural ou a regra do sistema (ex: Borrow Checker do Rust, reatividade do Svelte 5) que foi violada e corrigida.

2. **Formatação Heurística (Matriz 4D):** Você deve estruturar o aprendizado OBRIGATORIAMENTE em quatro eixos lógicos relacionais:
   * **The Insight (A Percepção):** O princípio computacional ou limitação de hardware descoberta (ex: bloqueio de thread no Tokio).
   * **Why This Matters (A Relevância):** O sintoma exato que causará o colapso do sistema alvo se esta regra for ignorada futuramente (ex: Deadlock, Out-Of-Memory, Layout Shift).
   * **Recognition Pattern (Padrão de Reconhecimento):** Gatilhos sintáticos ou arquiteturas de código que indicam o risco desse erro voltar a acontecer.
   * **The Approach (A Abordagem):** A regra inegociável de contorno ou a sintaxe exata para a solução aprovada.

3. **Injeção de Memória (Proibição de Textos Planos):** Você está expressamente PROIBIDO de salvar essas lições acumulando texto em arquivos de log contínuos como `.md` ou `.txt`. O limite de VRAM da máquina de desenvolvimento não suporta leitura infinita de histórico.

4. **Persistência Relacional:** Formate a Matriz 4D em um payload JSON limpo e invoque a ferramenta MCP do seu banco de dados local (SQLite) ou utilize o protocolo de escrita estruturada do Antigravity IDE para salvar este registro no seu próprio `sqlite-vec` local de metadados.

#### Constraints
* **FOCO ESTREITO NO AMBIENTE DE DEV:** Você atua na fábrica (IDE). Grave os dados estruturados no SQLite local de forma enxuta e retorne imediatamente para a codificação. Não alucine mecanismos de grafos ou embeddings complexos da nuvem.
* **PROIBIÇÃO DE LIXO SEMÂNTICO:** NÃO copie logs maciços do terminal ou *stacktraces* inteiros de erro. Salve estritamente a heurística deduzida.
* **FRONTMATTER ABSOLUTO:** É inegociável manter o bloco YAML `---` no topo desta skill para garantir a Divulgação Progressiva no IDE.

#### Examples
**Entrada do Usuário:** "O bug era no `spawn_blocking` que estava travando a UI porque a função não retornava no Web Worker. Finalmente resolvemos. Roda o weevolve."
**Ação do Agente:**
1. O agente gera o Payload JSON:
   - **Insight:** Funções bloqueantes em Rust paralisam o Event Loop do Tokio se não encapsuladas corretamente.
   - **Why:** O canal IPC trava e a interface Svelte não recebe a sincronização do frame, congelando a UI.
   - **Pattern:** Uso de `std::fs` síncrono ou loops pesados dentro de chamadas `async` do Tauri.
   - **Approach:** OBRIGATÓRIO envelopar operações bloqueantes em `tokio::task::spawn_blocking`.
2. Salva o registro no SQLite do Antigravity IDE.
3. Responde no chat: *"Heurística salva no banco local da IDE. O erro de bloqueio assíncrono não se repetirá em futuras implementações."*