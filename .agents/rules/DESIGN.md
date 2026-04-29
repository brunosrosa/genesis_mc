---
trigger: always_on
---

### 🏛️ CONSTITUIÇÃO VISUAL SODA: Genesis Mission Control V2
**Versão:** 2.2 (Canon Definitivo - UX/UI Neuro-Inclusiva) | **Paradigma:** Cyber-Neuro Synthesis + Nothing Design

#### 1. A Filosofia Topológica e o Mosaico Composicional (UX para 2e/TDAH)
O SODA atua como uma prótese de função executiva. O design é estruturado para erradicar o "Flow-Debt" (desorientação espacial) e poupar a VRAM/iGPU estritamente para a inferência matemática.
*   **Planaridade Absoluta (Morte do Z-Axis Livre):** O "Liquid Glass" contínuo (backdrop-filter: blur) está ESTRITAMENTE PROIBIDO no background principal, pois asfixia o Compositing do Tauri v2. Reserve o vidro translúcido exclusivamente para modais transitórios efêmeros.
*   **As 4 Zonas Inegociáveis do Mosaico (Tiling Window):** A tela é fracionada matematicamente via CSS Grid sem sobreposições:
    1. **HUD de Telemetria:** Topo fixo (`fixed top-0`). Usa Server-Sent Events e `$state` para atualizar texto bruto sem reflow.
    2. **Governor Rail:** Menu esquerdo imutável (`w-16`). Expansões são proibidas. 
    3. **Bottom Bar:** Rodapé de logs que usa o estado `:popover-open` da API HTML5 para não empurrar o conteúdo principal.
    4. **Flips (Painéis Efêmeros):** Deslizam do eixo X usando `@starting-style` do Tailwind v4.
*   **Focus Rack:** O ambiente de trabalho suporta no máximo 5 abas/slots ativos simultaneamente.

#### 2. Fricção Adaptativa e o Paradoxo do Tombstone (CRDT)
O dogma temporal exige ritmos biocomputacionais distintos para evitar o Viés de Automação (*Automation Bias*):
*   **Instância Mecânica (Navegação):** Cliques e *hovers* humanos reagem em **50ms a 150ms** usando `opacity` ou `color-mix()`. O layout NUNCA deve pular (Zero Layout Shift absoluto em ações mecânicas).
*   **Fricção Cognitiva Estruturada (Ações da IA):** Ações agênticas pesadas (Refatorações, Deleção via RAG) exigem um **Atraso Sintético de 800ms a 1500ms** na UI, forçando o neocórtex do usuário a validar a segurança da ação da IA.
*   **Reflow Orgânico (Resolução de Deleção):** Ao deletar blocos de texto (Tombstones), é proibido sumir com o nó instantaneamente (causa susto). 
    *   *Fase 1:* Use `$derived` para aplicar *grayscale(100%)* e opacidade reduzida.
    *   *Fase 2:* Anime a propriedade `grid-template-rows` de `1fr` para `0fr` no CSS (Cubic-bezier(0.25, 1, 0.5, 1)).
    *   *Fase 3 (Garbage Collection):* A runa `$effect` do Svelte 5 **DEVE** retornar uma *cleanup function* para destruir fisicamente o componente do DOM apenas após a animação acabar, limpando a RAM.

#### 3. Telemetria Fantasma (Ambient Status) e GenUI
Spinners de carregamento convencionais estão SUMARIAMENTE BANIDOS (induzem ansiedade). O status do agente de background opera por:
*   **Agent Inbox & Ghost Telemetry:** A IA propõe edições por "Pull Requests" assíncronos na gaveta lateral. O status de raciocínio aparece no terminal do rodapé em formato linear: `-> Extraindo AST -> OK`.
*   **Estados Marginais (Ghost Borders):** Injetados via pseudo-elementos (`::before`/`::after`). Toda animação DEVE usar `translateZ(0)` e `opacity` para delegar o trabalho à iGPU compositora sem recalcular CSS:
    *   *Pulso Subliminar (Espera):* Borda translúcida de 1px piscando a 2s (ritmo de mamífero em repouso).
    *   *Respiração Translúcida (Inferência Ativa):* Oscilação minúscula do `backdrop-filter` recuada.
    *   *Glow Revelation (Sucesso):* Emissão de calor perimetral sinalizando a entrega do dado.
*   **Cadeado de Soberania (GenUI Lock):** Micro-painéis (Flips) substituem configurações. Se o usuário clicar no ícone de cadeado (`not-locked:hover:bg-grey-100`), o Svelte dispara IPC para o Rust congelar aquele peso na base LadybugDB, impedindo a IA de auto-ajustar a métrica via LoRA Hot-Swapping.

#### 4. Dicionário Visual Tailwind v4
*   **Substrato:** Preto estrito `oklch(0.12 0 0)` com malha estrutural geométrica.
*   **Ghost Borders:** `shadow-[inset_0_0_0_1px_rgba(255,255,255,0.1)]`.
*   **Tipografia:** `Space Grotesk` (Autoridade/Headers), `Inter` (Leitura), `JetBrains Mono` ou `Doto` (Métricas em unidades fixas `ch`).