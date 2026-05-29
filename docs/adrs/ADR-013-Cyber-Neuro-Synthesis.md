# ADR-013-Cyber-Neuro-Synthesis

## Status
Aceito (Ativo e Inegociável)

## Contexto
Interfaces gráficas convencionais saturadas de animações supérfluas, popups intrusivos e cores berrantes causam sobrecarga cognitiva severa e desorientação espacial ("Flow-Debt"), o que é altamente prejudicial para usuários neurodivergentes (2e/TDAH) em momentos de hiperfoco. Adicionalmente, efeitos gráficos complexos de renderização (ex: Liquid Glass, `backdrop-filter: blur()`) asfixiam a iGPU integrada do host (Intel UHD 630) e provocam atrasos de desenho no Tauri v2, drenando ciclos que deveriam pertencer à inferência matemática.

## Decisão
Implementar rigidamente o paradigma visual **Cyber-Neuro Synthesis** sob a filosofia do **Nothing Design** no frontend em Svelte 5:
1. **Planaridade Absoluta e Fobia de Z-Axis:** Fica expressamente proibido o uso de filtros de desfoque de fundo (`backdrop-filter`) no background primário da aplicação. A interface gráfica é planar e estruturada matematicamente via CSS Grid.
2. **As 4 Zonas Topológicas Inegociáveis:**
   - *HUD Telemetria (Topo):* Fixo e com largura constante, atualiza telemetria de performance sem acionar reflows de layout.
   - *Governor Rail (Menu Esquerdo):* Menu imutável com largura fixa de `w-16` (`4rem`).
   - *Bottom Bar (Rodapé):* Exibe a *Ghost Telemetry* em fonte monoespaçada estática mono (ex: `-> Indexando AST -> OK`), banindo spinners rotativos causadores de ansiedade visual.
   - *Flips (Painéis Deslizantes):* Deslizam suavemente na horizontal utilizando `@starting-style` no CSS nativo.
3. **Focus Rack (Teto de 5 Slots):** A área de trabalho suporta o máximo de **5 abas/slots ativos** simultâneos. A invocação do 6º slot desmonta fisicamente a aba inativa mais antiga da RAM, mitigando a paralisia de análise por sobrecarga de abas.
4. **Instância Mecânica e Zero Layout Shift:**
   - Cliques e interações do usuário devem responder com atrito tátil imitando interruptores mecânicos em menos de **50ms a 150ms**.
   - Animações de alteração gráfica de layout são proibidas de modificar propriedades causadoras de *Reflow* (width, height, margin, padding). Modificações operam unicamente sobre `transform` e `opacity` aceleradas por hardware.
5. **O Paradoxo do Tombstone (Deleção Suave):** A exclusão física de um elemento do DOM segue 3 fases:
   - *Decaimento:* Runa `$derived` reduz opacidade e aplica *grayscale(100%)*.
   - *Esmagamento:* Anime a propriedade CSS `grid-template-rows` de `1fr` para `0fr` via curva `cubic-bezier`.
   - *Aniquilação:* Runa `$effect` executa a limpeza e destrói o nó do DOM e da RAM somente após o término físico da transição.
6. **Dicionário Visual Tailwind v4:** Configuração pura direto no CSS via diretiva `@theme` (arquivo tailwind.config.js banido). Fundo preto absoluto (`oklch(0.12 0 0)`), fontes Space Grotesk (títulos), Inter (leitura) e JetBrains Mono/Doto (logs e monocomprimento) com unidades exclusivas `ch` (horizontal) e `rem` (vertical).

## Consequências
- **Foco Imperturbável:** Raciocínio centrado sem distrações visuais ou quebras espaciais bruscas de componentes.
- **Pegada Térmica Nula na GPU:** A renderização da UI rodando na UHD 630 UHD consome menos de 5% de energia térmica, preservando a RTX 2060m dGPU inteiramente para cálculos matriciais de IA.
- **Consistência Visual Total:** O design estrito e as 4 zonas garantem harmonia visual inquebrável em qualquer resolução de tela.

## Restrições Bare-Metal
- **Latência de Instância Mecânica:** Tempo de resposta visual a eventos de cliques limitado a no máximo **150ms**.
- **Limite Rígido do Focus Rack:** Máximo absoluto de **5 abas** ativas concorrentes em memória.
- **Desfoque de Vidro:** Restrito unicamente a modais superiores efêmeros flutuantes de Blast Radius de alta segurança (High Z-Index).
- **Telemetria Zero-Copy:** Streams de telemetria de alto throughput devem usar **iceoryx2** (POSIX Shared Memory) no core; é proibido Arrow FFI com dupla serialização no caminho Rust $\rightarrow$ UI.
- **Purificação Reativa (Svelte 5):** Antes de entrar no pipeline visual, objetos complexos devem ser materializados via `$state.snapshot()`; o pipeline de UI consome buffers já purificados (sem Proxies reativos).
- **Batching Visual (rAF + Transferables):** A entrega de buffers via Web Workers deve ocorrer com **Transferable Objects** e o commit visual deve ser batelado estritamente via `requestAnimationFrame` (rAF) para evitar frame drops.
