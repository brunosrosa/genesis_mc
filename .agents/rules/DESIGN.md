---
trigger: always_on
---

# 🏛️ CONSTITUIÇÃO VISUAL SODA: Genesis Mission Control
**Versão:** 1.0 (Definitiva) | **Paradigma:** Cyber-Neuro Synthesis + Nothing Design

## 1. A Filosofia Visual (UX para 2e/TDAH)
O SODA não é um SaaS corporativo genérico; é um **Cockpit Neural**. A interface atua como uma prótese de função executiva.
* **O Esqueleto (Nothing Design):** Rigidez industrial, matemática e tipográfica. Grids absolutos, espaçamentos matemáticos em múltiplos de 4px, e hierarquia de informação implacável.
* **A Pele (Cyber-Neuro Synthesis):** Imersão tátil e profunda. Cores abissais, feedback dopaminérgico em 50ms, e "Glassmorphism" utilitário. 

## 2. Leis Inegociáveis (Uncodixfy Anti-Patterns)
É ESTRITAMENTE PROIBIDO à IA ou aos desenvolvedores aplicarem:
- 🚫 **"SaaS Slop":** Painéis brancos chapados, botões azuis genéricos ou excesso de bordas super arredondadas (evite `rounded-3xl` em painéis maiores, use `rounded-lg` ou `rounded-xl` com precisão).
- 🚫 **Sombras Pesadas:** Fim do `box-shadow` cinza/preto chapado.
- 🚫 **Fundos #000 Puros:** Fim do preto absoluto e sem vida, pois causa fadiga visual de contraste.
- 🚫 **Animações Lentas:** Elementos não flutuam. O TDAH precisa de resposta imediata. Proibidas animações que alteram largura/altura (causando Layout Shift).

## 3. Dicionário Tailwind v4 e Parametrização Cyber-Purple
Utilize EXCLUSIVAMENTE estas classes para forçar a estética SODA:

* **O Substrato (Background do App):** 
  - `bg-[radial-gradient(ellipse_at_center,_var(--tw-gradient-stops))] from-zinc-900 via-[#0a0a0e] to-black`
  - Uma malha cibernética ultraleve por cima: *Cyber Grid 2% opacity*.
* **Vidro Tátil (Painéis e Modais):** 
  - `bg-black/40 backdrop-blur-xl` (Garante que a interface "respira" sobre o fundo).
* **Ghost Borders (Separação Espacial):** 
  - `border border-white/5` ou `border-primary/20` (As bordas definem a geometria, não sombras).
* **Primary Glow (O Pulso Neural):** 
  - Para nós ativos, botões acionados ou o *Spotlight*: `shadow-[0_0_15px_rgba(186,158,255,0.15)]` e `ring-1 ring-purple-500/30`.
* **Instância Mecânica (Tato):** 
  - `transition-all duration-75 ease-out`. Os hovers devem "estalar" em 50ms.
* **Tipografia:** 
  - **Space Grotesk:** Para títulos, painéis estruturais e "Autoridade Editorial".
  - **Space Mono / Doto:** Para exibição de códigos, logs, telemetria (FinOps) e metadados.

## 4. Estrutura de Layout (Construção por Fases de Locking)
A arquitetura de componentes exige renderização modular para preservar a estabilidade da árvore DOM:
1. **The Compass (Header):** Fixo, abriga a trilha de navegação (Breadcrumbs) e a *OmniBar* de pesquisa. Altura de `h-14`.
2. **The Engine Room (Footer):** Subliminar (`h-8`), contraste quase nulo. Exibe telemetria de consumo de tokens, status do *Cron* e a *Emotion/Perplexity Wave* da IA.
3. **Governor Rail (Menu Esquerdo):** Retrátil via `transform: translateX` (Zero Layout Shift). Contém as divisões: FLOW (Agora), VAULT (Passado), FORGE (Fábrica) e CORE (Configurações).
4. **Nexus (O Palco Central):** Onde os Canvases (Architecture, Kanban, Diff) habitam. 
5. **Focus Rack:** Submenu no Nexus ou Governor Rail com um limite rígido de **5 slots ativos**. Fixa contextos essenciais; ao abrir o 6º, o usuário deve fechar um (Blindagem TDAH contra abas infinitas).
6. **Spotlight Efêmero:** Barra invisível acionada por atalho (ex: Alt+Espaço) para entrada de ideias com atrito zero, centralizada na tela, que invoca o Agente Mestre antes de abrir a UI pesada.

## 5. Compute-Aware UX (Estados Transitórios e Inteligência Sensorial)
A UI deve refletir o estado assíncrono do backend (Rust) passivamente:
* **Optimistic Transient State:** Alterações piscam roxo (Neural Pulse) com 70% de opacidade. Ao confirmar no SQLite (Rust), recebem 100% de opacidade e snap visual. Erros pulsam em vermelho escuro.
* **Accordion Scratchpad:** O raciocínio do modelo RLM (Recursive Language Model) fica oculto em uma gaveta sanfonada com borda em pulso roxo. O TDAH só vê o resultado limpo, a não ser que abra o acordeão.
* **Skeleton Loaders Paramétricos:** Esqueletos geométricos rígidos usando um *shimmering gradient* translúcido calculado antes dos dados chegarem.
* **Ghost Cursor:** Um cursor roxo cibernético (`opacity-50`) que flutua na tela guiado pelo Agente Mestre para apontar erros ou focar a atenção do usuário colaborativamente.
* **Graph Anti-Hairball:** Clusters topológicos colapsam automaticamente. Nós de arquitetura só se expandem (zoom semântico) no clique, evitando poluição visual extrema.