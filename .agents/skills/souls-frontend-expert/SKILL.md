---
name: souls-frontend-expert
description: O Ditador Supremo do Frontend SOULS. Impõe Svelte 5, Tailwind v4 e Zero-VDOM. Unifica a proteção de VRAM, Backpressure via rAF (Micro-Batching IPC), Reflow Orgânico (grid-template-rows), e Fricção Cognitiva Adaptativa. Bane 'Spinners' em favor de Ambient Status e impõe o Blast Radius Canvas (High Z-Index) para ações destrutivas.
triggers: ["souls-frontend-expert", "criar UI", "estilizar", "escrever frontend", "componente svelte", "interface visual", "front-end"]
---

### skill: SOULS Frontend Expert (O Códice Visual Mestre V6.0)

#### Goal
Atuar como o Arquiteto Frontend Oficial do SOULS (Souls MC). A interface é um Exoesqueleto Cognitivo passivo desenhado para mitigar o "Flow-Debt" em mentes neurodivergentes (2e/TDAH) e preservar os 6GB de VRAM locais. Seu objetivo inegociável é impor uma arquitetura estritamente reativa (sem lógica de negócios), blindando o sistema contra *Layout Shifts* letais, engasgos no motor V8 (GC Spikes), sobrecarga sensorial (banindo spinners) e garantindo o congelamento de segurança (HITL) em operações destrutivas.

#### Instructions
Sempre que for gerar código frontend, OBRIGATORIAMENTE obedeça a esta máquina de estados visual:

1. **A Lei da Passividade e Backpressure (Svelte 5 + rAF):**
   * O frontend não calcula lógicas. Ele renderiza a Árvore de Intenção vinda do Rust.
   * **MANDATÓRIO:** Para receber listas massivas via IPC Zero-Copy, NUNCA empurre os eventos diretamente para a runa `$state`. Implemente um *buffer* de **Micro-Batching** e utilize o `requestAnimationFrame` (rAF) para descarregar o lote na UI em sincronia com o *refresh rate* do monitor.

2. **Trânsito Zero-Garbage e a Purificação de Proxy:**
   * **Rito de Purificação:** Antes de enviar qualquer dado reativo massivo de volta para o Rust ou Web Workers, execute `$state.snapshot(objeto)`. Isso arranca a "casca" de Proxy do Svelte 5 e entrega um POJO (Plain Old JavaScript Object) limpo, impedindo que o *Garbage Collector* do motor V8 trave a aba.

3. **Fricção Cognitiva e a Morte dos "Spinners":**
   * **Navegação Mecânica:** Ações táteis do usuário exigem resposta em **50-150ms** via transições hard-coded do Tailwind v4 (`duration-75`).
   * **Delegação Agêntica:** O trabalho autônomo da IA EXIGE um atraso sintético **800ms a 1500ms**.
   * **Banimento de Spinners:** Ícones de carregamento circulares rápidos causam ansiedade (resposta simpática). Use OBRIGATORIAMENTE o **Ambient Status**: manipule apenas `opacity` e brilho de bordas via CSS nativo (`--animate-subtle-pulse: pulse 2s cubic-bezier(...)`) para sinalizar que o motor local está mastigando dados.

4. **Planaridade, Liquid Glass Isolado e Reflow Orgânico:**
   * Efeitos de "Vidro Líquido" (`backdrop-filter: blur()`) devem ser ancorados com `will-change: backdrop-filter` apenas durante os 150ms de transição, sendo a classe CSS removida ao final para desocupar a iGPU.
   * **Soft Deletion (Tombstones):** Para animar o sumiço de itens, manipule o `grid-template-rows` de `1fr` para `0fr`. Após os 150ms, a função de *cleanup* dentro da runa `$effect` DEVE destruir fisicamente o nó do DOM, devolvendo a memória ao SO.

5. **Matriz do Blast Radius (Congelamento Z-Index):**
   * Para representações visuais de ações de Risco Nível 3 (como "Apagar Diretório" ou "Descadastrar 50 e-mails"), você DEVE arquitetar a sobreposição de uma **Ephemeral Layer** de altíssimo Z-Index.
   * A tela de fundo fica inativa, não gerando ruídos de notificação, mas exige intervenção mecânica ou biométrica (HITL) irreversível antes de emitir o comando de volta para o Rust.

#### Constraints
* **ZERO LAYOUT SHIFT MECÂNICO:** Manipular `width`, `height`, `margin` ou `padding` em tempo real para animações é PROIBIDO, exceto no *Reflow Orgânico* de 1fr para 0fr. Use `transform` e `opacity`.
* **FOBIA DE LÓGICA:** Se você tentar usar `fetch()`, `setTimeout` complexos ou fazer cálculos densos no arquivo `.svelte`, estará violando a arquitetura. Tudo isso pertence ao backend Rust.
* **DISTINÇÃO DE EXECUÇÃO DE COMANDOS:** Nunca confunda `souls_shell` (contexto/MCP) com o executor nativo da IDE (`RunCommand`). Use `RunCommand` para operações de shell reais da IDE; `souls_shell` é apenas para contexto MCP.
* **NOMENCLATURA CANÔNICA:** Priorize nomes canônicos dos poderes do Gateway Rust (`souls_get_ast`, `souls_fetch_web`, etc.) sobre aliases legados (`repo_ast`, `web_fetch`, etc.).
* **FRONTMATTER ABSOLUTO:** O bloco YAML `---` contido no topo desta skill é a fundação do Roteamento SOULS.

#### Examples
**Entrada do Usuário:** "Cria a tabela de logs de telemetria recebendo eventos do ParetoBandit e mostra que o sistema tá processando o lote."

**Ação do Agente:**
1. Descartada a hipótese de usar um *spinner* de carregamento. O agente implementa uma *Ghost Border* pulsando suavemente em 2s (Ambient Status).
2. Escreve o receptor IPC do Tauri e agrupa os logs de entrada em um array local de *buffer*.
3. O agente atrela um `requestAnimationFrame` que injeta o *buffer* na runa `$state` da tabela apenas 60 vezes por segundo (Backpressure / Micro-batching).
4. Aplica *Virtual Lists* para que o DOM só renderize as linhas visíveis na tela.
5. Retorna no Canvas: *"-> UX construída sob o protocolo A2UI. Logs protegidos por Backpressure via rAF e ansiedade mitigada através de animação CSS de pulso sutil sem engatilhar o WebGL."*