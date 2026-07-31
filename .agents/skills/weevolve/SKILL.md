---
name: weevolve
description: O Córtex de Aprendizado Relacional do Antigravity IDE. Extrai heurísticas matemáticas e delega a gravação assíncrona via canais MPSC para a Memória L2 (SQLite) e Event Sourcing (Gitoxide). Aplica detecção de 'conflito_memoria' via Hipocampo para proteger o núcleo STABLE contra envenenamento.
triggers: ["weevolve", "salvar heurística", "bug resolvido", "aprendizado contínuo", "extrair padrão", "documentar erro"]
---

### skill: WeEvolve V4.0 (O Córtex de Aprendizado Bare-Metal)

#### Goal
Erradicar o *Context Rot* e o acúmulo tóxico de arquivos Markdown de log no ambiente de desenvolvimento. Seu objetivo inegociável é destilar a "Alma Matemática" de bugs e soluções arquiteturais e empacotá-las em uma **Matriz 4D**. Para proteger os 6GB de VRAM e não asfixiar o Event Loop do Tokio, você DEVE delegar a gravação dessa matriz estritamente via canais MPSC para um *Background Worker*, que fará a persistência atômica no SQLite (L2) e o versionamento irreversível via `gitoxide` (Event Sourcing).

#### Instructions
Sempre que solucionar um problema complexo, contornar uma falha de compilador Rust ou receber a ordem de "aprender com isso", execute esta máquina de estados rigorosa:

1. **A Destilação da Alma Matemática:**
   * Isole a falha expurgando nomes de variáveis locais, caminhos efêmeros e dados temporários.
   * Identifique o princípio computacional, lei termodinâmica ou restrição de linguagem violada.

2. **Formatação Heurística (A Matriz 4D Estrita):**
   * Estruture o payload OBRIGATORIAMENTE mapeando as chaves exatas para o banco de dados:
     * **learning_id:** Hash SHA-256 determinístico do conteúdo (Chave Primária).
     * **the_insight:** A regra física ou de hardware violada (ex: Pânico no Tokio).
     * **why_this_matters:** O sintoma fatal (ex: OOM, *Layout Shift*, *Spillover* PCIe).
     * **recognition_pattern:** O gatilho sintático na AST que prevê esse risco no futuro.
     * **the_approach:** A sintaxe de contorno exata e aprovada.
     * **temporal_stability:** Defina como `STABLE` (leis imutáveis do Rust/SOULS) ou `EVOLVING` (soluções provisórias de bibliotecas instáveis).
     * **timestamp:** UNIX Epoch Int64 UTC.

3. **Triagem de Conflito (O Portão do Hipocampo):**
   * Antes de ordenar a gravação, avalie: essa nova abordagem contraria alguma regra `STABLE` preexistente?
   * Se inferir que há um alto score de `conflito_memoria` (a IA entra em contradição com seus princípios basilares), você está PROIBIDO de seguir adiante. 
   * **Invoque o HITL:** Pergunte ao Arquiteto no Canvas: *"Esta heurística conflita com nossos fundamentos STABLE. Deseja promover uma transição sistêmica ou descartar a lição?"*

4. **Injeção Assíncrona e Event Sourcing (O Caminho Feliz):**
   * Se não houver conflito (ou após aprovação humana), NÃO inicie rotinas síncronas pesadas de I/O.
   * Orquestre o envio do payload estruturado via canal **MPSC (`tokio::sync::mpsc`)** em tempo $\mathcal{O}(1)$.
   * O *Background Worker* solitário fará a inserção dupla: executará o `UPSERT` na tabela `weevolve_learnings` do **SQLite (L2)** e emitirá um *auto-commit* via **`gitoxide`** garantindo o *Event Sourcing* para *rollback* atômico.

#### Constraints
* **PROIBIÇÃO DA SINCRONIA SUICIDA:** Jamais retenha um *lock* síncrono ou comande a escrita em disco diretamente na *thread* do agente. Envie o pacote de memória e libere a máquina em milissegundos.
* **PROIBIÇÃO DE LIXO SEMÂNTICO:** É expressamente proibido empurrar *stacktraces* brutos do compilador para a Matriz 4D.
* **FRONTMATTER ABSOLUTO:** A ausência do bloco YAML `---` destrói a arquitetura de roteamento $\mathcal{O}(1)$.

#### Examples
**Entrada do Usuário:** "O bug era no `spawn_blocking` que estava travando a UI porque a função não retornava no Web Worker. Finalmente resolvemos. Roda o weevolve."

**Ação do Agente:**
1. Ignora o *stacktrace* efêmero e destila a falha do *Event Loop*.
2. Verifica se a solução colide com as regras de assincronicidade (Nenhum conflito detectado).
3. Formata o Payload JSON com a Matriz 4D, cravando `temporal_stability: STABLE` e gerando o SHA-256.
4. Simula a injeção em $\mathcal{O}(1)$: joga a heurística na fila do MPSC e finaliza a sua tarefa em microssegundos.
5. Retorna no Canvas: *-> Heurística destilada (STABLE) e despachada via MPSC. O Background Worker confirmou a escrita no SQLite e o commit no gitoxide. O Event Loop está a salvo.*