---
trigger: always_on
---

###### CONSTITUIÇÃO VISUAL SODA
**Paradigma:** Cyber-Neuro Synthesis + Nothing Design (UX Neuro-Inclusiva).
**Objetivo:** Erradicar "Flow-Debt" (desorientação espacial) e poupar a VRAM/iGPU estritamente para a inferência matemática. A interface (Svelte 5) não calcula lógicas de negócio; atua apenas como renderizador passivo em tempo real.

###### 1. MOSAICO COMPOSICIONAL E PLANARIDADE ABSOLUTA
O frontend é fracionado matematicamente via CSS Grid, repudiando janelas flutuantes caóticas.
*   **A Morte do Eixo Z Livre:** É EXPRESSAMENTE PROIBIDO o uso de `backdrop-filter: blur()` (*Liquid Glass*) no background primário da aplicação (asfixia a iGPU e o Tauri v2). O vidro translúcido é restrito EXCLUSIVAMENTE a modais efêmeros superiores.
*   **As 4 Zonas Inegociáveis:** 1) HUD Telemetria (Topo fixo, atualiza sem *reflow*); 2) Governor Rail (Menu esquerdo imutável `w-16`); 3) Bottom Bar (Rodapé de *Ghost Telemetry*); 4) Flips (Painéis efêmeros deslizando via `@starting-style`).
*   **Focus Rack:** O ambiente suporta o MÁXIMO de 5 abas/slots ativos simultaneamente. Ao invocar o 6º, desmonte fisicamente o mais antigo para combater a paralisia de análise.

###### 2. FRICÇÃO ADAPTATIVA E RITMOS NEUROLÓGICOS
O tempo de resposta diverge propositalmente para evitar o Viés de Automação (*Automation Bias*):
*   **Instância Mecânica:** Ações humanas diretas (cliques, *hovers*) DEVERÃO responder entre **50ms e 150ms**, mimetizando chaves físicas.
*   **Fricção Cognitiva Estruturada:** Ações agênticas autônomas da IA (Refatorações profundas, Deleções massivas) EXIGEM **Atraso Sintético de 800ms a 1500ms** na UI, forçando a validação neocortical consciente do usuário.
*   **Zero Layout Shift:** Em ações mecânicas, é PROIBIDO alterar CSS que cause *Reflow* letal (width, height, margin, padding). Animações OBRIGATORIAMENTE operam sobre `transform` (escala, translação) e `opacity`.

###### 3. O PARADOXO DO TOMBSTONE (REFLOW ORGÂNICO)
Nós deletados do DOM (Tombstones) NUNCA devem sumir instantaneamente causando susto espacial:
1.  **Decaimento (Fase 1):** Runa `$derived` aplica *grayscale(100%)* e reduz opacidade.
2.  **Esmagamento (Fase 2):** Anime a propriedade `grid-template-rows` de `1fr` para `0fr` no CSS (curva `cubic-bezier`), achatando o texto organicamente.
3.  **Aniquilação Atômica (Fase 3):** A runa `$effect` DEVE retornar uma *cleanup function* para destruir fisicamente o componente da RAM **APENAS** ao término exato da animação (Garbage Collection cirúrgico).

###### 4. SINALIZAÇÃO SUBLIMINAR E GHOST TELEMETRY
*   **A Morte dos Spinners:** Ícones giratórios de carregamento estão SUMARIAMENTE BANIDOS (causam pânico visual).
*   **Ghost Telemetry:** O trabalho de background da IA é relatado como texto estático monoespaçado na *Bottom Bar* (ex: `-> Indexando AST -> OK`).
*   **Agent Inbox e Recompensa:** IAs não modificam o layout agressivamente. Sugestões vão como *Pull Requests* à Agent Inbox. Aprovar os lotes gera a **Glow Revelation Transition** (brilho térmico sutil nas bordas, sem alterar o layout).
*   **Ghost Borders:** Use bordas de vidro subliminares (`box-shadow: inset`) com pulsos lentos baseados em variáveis CSS (`--pulse-frequency`) para indicar leitura, sem poluir o centro da fóvea.
*   **Trava de Soberania (GenUI Lock):** Se o usuário ativar o cadeado ao lado de um ajuste, a IA fica matematicamente bloqueada de alterar aquele comportamento via *hot-swapping* de pesos.

###### 5. DICIONÁRIO VISUAL (TAILWIND V4)
*   **CSS-First:** O arquivo `tailwind.config.js` está banido. A configuração ocorre diretamente no CSS via diretiva `@theme`.
*   **Substrato e Cor:** Fundo preto absoluto (`oklch(0.12 0 0)`). O uso de OKLCH é obrigatório para manter consistência espectral nos gradientes de luminosidade.
*   **Tipografia Híbrida:** *Space Grotesk* (Autoridade/Títulos), *Inter* (Leitura limpa), e *JetBrains Mono / Doto* (Dados/Logs com alinhamento inquebrável).
*   **Unidades:** Uso OBRIGATÓRIO de `ch` (horizontal) e `rem` (vertical).