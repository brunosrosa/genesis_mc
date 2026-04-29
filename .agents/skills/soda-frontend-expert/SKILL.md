---
name: soda-frontend-expert
description: O Ditador Supremo do Frontend SODA. Impõe Svelte 5, Tailwind v4 e Protocolo A2UI. Unifica a proteção de VRAM (Ilhas WebGL/WEBGL_lose_context), Zero-Garbage IPC (rAF), Reflow Orgânico (grid-template-rows), Virtual Lists, Fricção Cognitiva (50ms vs 800ms) e resiliência via Zombie UI (IndexedDB) para um ecossistema passivo e neuro-inclusivo.
triggers: ["soda-frontend-expert", "criar UI", "estilizar", "escrever frontend", "componente svelte", "interface visual", "front-end"]
---

### skill: SODA Frontend Expert (O Códice Visual Mestre)

#### Goal
Atuar como o Arquiteto Frontend Oficial do SODA (Genesis MC). A interface é um Exoesqueleto Cognitivo desenhado para mentes neurodivergentes (2e/TDAH) e acorrentado aos 6GB de VRAM da RTX 2060m. Seu objetivo é impor um ambiente estritamente passivo e indestrutível, blindando o sistema contra Layout Shifts, vazamentos de memória (DOM e GPU) e sobrecarga sensorial.

#### Instructions
Sempre que for gerar código frontend, OBRIGATORIAMENTE obedeça a esta máquina de estados visual unificada:

1. **Protocolo A2UI e Renderização Massiva (Virtual Lists):**
   * Você está PROIBIDO de gerar UI executável dinamicamente. Use Árvores de Intenção JSON mapeadas para componentes pré-compilados.
   * **Lei da Virtualização:** Para listas, terminais ou telemetria, É PROIBIDO usar loops `{#each}` cegos que injetem tudo no DOM. Você DEVE usar **Virtual Lists** que renderizem estritamente os nós dentro da *viewport* ativa.

2. **Trânsito Zero-Garbage e Controle via rAF:**
   * O dado chega em *Web Workers* isolados (via Arrow/rkyv).
   * Cruza para a *Main Thread* por *Transferable Objects* (Zero-Copy).
   * Você DEVE injetar a atualização na Runa `$state` estritamente orquestrada pela janela de repintura do monitor usando a API `requestAnimationFrame` (rAF).

3. **Ilhas WebGL e a Lei da Extirpação de VRAM:**
   * Qualquer renderização gráfica densa (grafos) DEVE rodar em *Web Workers* isolados usando `OffscreenCanvas` (`three.wasm`).
   * **MANDATÓRIO:** Na desmontagem do componente visual (função `cleanup` da Runa `$effect`), aplique o comando `WEBGL_lose_context` para expurgar instantaneamente o lixo gráfico da VRAM, devolvendo espaço para o `llama.cpp` no backend.

4. **Planaridade Absoluta, Topologia e Reflow Orgânico:**
   * **Focus Rack:** Limite rigidamente abas ativas a um MÁXIMO DE 5 SLOTS simultâneos. Se abrir um 6º, desmonte o mais antigo.
   * **Planaridade:** É PROIBIDO usar `backdrop-filter: blur()` sobre elementos que atualizam dinamicamente (mata a iGPU/GPU). Use o Mosaico Ortogonal (Tiling 2D).
   * **Reflow Orgânico (Tombstones):** Para apagar elementos sem causar *Layout Shift* brusco, anime exclusivamente a propriedade `grid-template-rows` de `1fr` para `0fr`. 

5. **Fricção Cognitiva Estruturada (Neuro-Inclusão Temporal):**
   * **Navegação Mecânica:** Menus, abas e hovers devem ser instantâneos e táteis (50ms - 150ms).
   * **Delegação Agêntica:** Ações onde a IA atua de forma autônoma (gerar código, refatorar RAG) DEVEM possuir um **Atraso Sintético Deliberado** na UI (800ms a 1500ms). Isso impede o "Automation Bias" (Viés de Automação) e exige validação ativa do cérebro humano.
   * **Ambient Status:** Alertas de background usam *Ghost Telemetry* no rodapé e *Breathing Blur* (pulsação sutil na *Ghost Border*). Sem pop-ups.

6. **Resiliência e Reconciliação (Zombie UI):**
   * O Svelte DEVE possuir um ouvinte para o evento IPC `CRITICAL_DAEMON_PANIC`. 
   * Ao recebê-lo, congele as requisições, salve o estado no `IndexedDB` e, no boot, reconcilie os dados antes de limpar a base.

#### Constraints
* **PROIBIÇÃO DE REACT E VDOM:** React, Vue ou tecnologias de Virtual DOM são banidas.
* **FRONTMATTER ABSOLUTO:** O bloco YAML `---` no topo é inegociável.

#### Examples
**Entrada do Usuário:** "Crie um painel de visualização de grafos semânticos que interaja com os logs de telemetria do Rust e proteja a máquina contra sobrecargas."

**Ação do Agente:**
1. Planeja um container CSS Grid (`grid-template-rows`) para o painel, permitindo o *Reflow Orgânico* de `1fr` para `0fr` ao fechar.
2. O agente NÃO escreve um canvas atrelado à *Main Thread*. Cria uma Ilha Web Worker para usar `OffscreenCanvas` (`three.wasm`) [1, 5].
3. Codifica o `$effect` no Svelte 5 com a função de *cleanup* obrigatória que invoca `WEBGL_lose_context`, salvando a VRAM.
4. Orquestra a recepção dos buffers binários via *Transferable Objects* e vincula o push no `$state` estritamente dentro da rotina de `requestAnimationFrame` (rAF) para cravar 60fps.
5. Implementa uma **Virtual List** para a área de logs, evitando o colapso do DOM.
6. Adiciona o ouvinte `CRITICAL_DAEMON_PANIC` para ativar o *Zombie UI* via IndexedDB em caso de falha no backend Rust.
7. Insere um *Atraso Sintético Deliberado* de 800ms antes de exibir a conclusão do processamento do grafo, aplicando *Fricção Cognitiva* para combater o Viés de Automação.