---
sticker: lucide//component
---
# 05_UI_UX_DESIGN_SYSTEM: Cyber-Neuro Synthesis e Protocolo A2UI

**Versão:** 3.2 (Definitiva - Neuro-Inclusão & Zero-Lag)
**Status:** ATIVO E INEGOCIÁVEL
**Alvo da Leitura:** Engenheiros de Frontend (React/Tauri), Designers de Interface, Agentes de UI Generativa (@vibe-designer).

## 1. O NORTE CRIATIVO: "CYBER-NEURO SYNTHESIS"

O Genesis Mission Control (SODA) **não é um site ou um SaaS corporativo genérico B2B**. É um _Cockpit Neural Bare-Metal_. A interface atua como uma prótese de função executiva para mentes neurodivergentes (2e/TDAH).

Rejeitamos o "brutalismo estático" e a "sopa de metáforas visuais" (bordas excessivamente arredondadas, sombras difusas e cores berrantes). A estética oficial é a **Cyber-Neuro Synthesis**, que funde o utilitarismo industrial do _Nothing Design_ com a profundidade ótica e o feedback mecânico tátil.

## 2. AS 4 LEIS INEGOCIÁVEIS DE UX (ANTI-RSD & TDAH)

A carga cognitiva imposta por interfaces desordenadas resulta em fadiga e paralisia de decisão. O SODA rege-se por quatro leis de neuro-acessibilidade:

1. **Progressive Disclosure (Divulgação Progressiva):** O Canvas (Svelte Flow) começa em "Estado Zen" (vazio). A complexidade só se desdobra sob demanda explícita. Menus não possuem 20 opções empilhadas; eles possuem gavetas lógicas de 1 nível.
2. **Mechanical Instancy (Zero Lag e Dopamina):** O cérebro TDAH necessita de fechamento de loops de recompensa. O feedback tátil a um clique (mudança de estado `:active` ou `:hover`) deve ocorrer em $< 50ms$. A IA pode demorar 10 segundos para inferir uma resposta pesada no Rust, mas a UI _jamais_ pode parecer congelada.
3. **Spatial Predictability (Bússola Fixa):** O Header (Compass) e o Footer (Engine Room/Terminal) são âncoras inamovíveis. Eles garantem a orientação espacial do usuário durante estados de hiperfoco profundo. Se o usuário se perder no Canvas infinito, a visão periférica o ancora de volta à realidade do sistema.
4. **Zero Layout Shift:** A expansão ou retração de menus e painéis deve ser obrigatoriamente manipulada via transformações de GPU (`transform: translateX` ou `scale`). Alterar larguras via `width` (causando flexbox reflow) gera engasgos na renderização (frame-drops) e poluição visual, sendo uma prática expressamente proibida.

## 3. O ESQUELETO E A PELE: TAILWIND V4 E GEOMETRIA

A IA geradora de código visual deve obedecer estritamente a este dicionário:

- **O Substrato Base:** Nunca usar preto absoluto flat (`#000000`). O fundo deve possuir textura ou gradiente radial sutil (ex: `bg-[radial-gradient(...)] from-purple-950/20 to-[#05050A]`) para criar noção de espaço sem disputar a atenção.
- **Vidro Tátil (Glassmorphism Utilitário):** Painéis e barras sobrepõem o fundo com desfoque exato (`bg-white/5 backdrop-blur-md`), separando o conteúdo ativo do passivo sem criar barreiras opacas agressivas.
- **Ghost Borders e Glows Direcionais:** Bordas sólidas tradicionais estão banidas. Limites são desenhados via sombras internas (`shadow-[inset_0_0_0_1px_rgba(255,255,255,0.1)]`). O foco é sinalizado por um brilho funcional roxo/neon subjacente (ex: `shadow-[0_0_15px_rgba(168,85,247,0.15)]`).
- **Tipografia Hierárquica:** Uso estrito de fontes matemáticas (ex: _Doto_, _Space Mono_ ou _Geist Mono_).

## 4. PROTOCOLO A2UI (AGENT-TO-USER INTERFACE)

Um erro fatal de sistemas legados é permitir que o LLM gere e injete HTML/JSX cru no DOM, o que abre vetores de ataque (XSS) e destrói o layout.

- **Macro-Componentes:** O frontend React possui um dicionário pré-aprovado de Macro-Componentes (wrappers de _Shadcn UI_).
- **Comunicação Segura:** A IA residente no Rust **não escreve UI**. Ela emite "Envelopes Semânticos" (JSON) via a ponte IPC Binária Zero-Copy do Tauri. Exemplo: `{"intent": "createSurface", "type": "DataTable", "data": [...]}`.
- **Renderização Passiva:** O React apenas lê o JSON e hidrata o Macro-Componente correspondente em tempo de complexidade $\mathcal{O}(1)$. Se a IA alucinar um componente inexistente, o React descarta o pacote silenciosamente, protegendo a integridade da tela.

## 5. A TOPOLOGIA MACRO: FOCUS RACK E OMNICHAT

Para organizar o caos informacional (brain-dumps):

- **The Governor Rail (Menu Esquerdo):** Concentra a navegação primária.
- **O Focus Rack:** Um limite duro (hard constraint) imposto na UI. O usuário só pode ter **5** abas/contextos de trabalho ativos simultaneamente. Para abrir um 6º projeto, ele é forçado a fechar um. Isso é um escudo comportamental contra a acumulação infinita de abas.
- **OmniChat (O Spotlight Efêmero):** Capturar telas a 60fps destrói a VRAM de 6GB da RTX 2060m. O SODA opera sob "Fricção Zero". O usuário aperta um atalho global (ex: `Alt + Espaço`) de qualquer lugar do Windows. Uma barra minimalista flutuante aparece. Ele despeja o áudio ou texto, o daemon em Rust tira um snapshot cirúrgico dos metadados da janela ativa, envia a intenção, e a barra desaparece. Sem abrir janelas pesadas. Sem interromper o fluxo.

_Fim da Especificação de Interface. O Cockpit Neural está pronto para a simbiose humana._