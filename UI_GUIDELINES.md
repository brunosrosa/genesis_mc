# 🎨 UI GUIDELINES: Acessibilidade Cognitiva e Fluxo Espacial

A interface do Genesis NÃO é um website; é um **Sistema Operacional Visual**. O design deve respeitar mentes de alto desempenho, eliminando a fadiga cognitiva de logs textuais e traduzindo estados complexos de máquina em abstrações espaciais fluidas, organizadas e táteis.

## 1. CORE DESIGN SYSTEM (A FUNDAÇÃO)

- **Framework Base:** Tailwind CSS v4.
- **Blocos de Construção:** Shadcn UI. Os componentes devem ser integrados diretamente na pasta `src/components`, garantindo total propriedade do código (sem dependência de wrappers NPM ocultos).
- **Esquema de Cores:** Estritamente baseado nas variáveis semânticas do Shadcn (ex: `bg-background`, `text-primary`, `border-border`). **NUNCA** utilize cores fixas de paleta (ex: `bg-red-500`), assegurando um suporte perfeito e automático ao Modo Escuro.

## 2. A LEI DO MOVIMENTO (FRAMER MOTION OBRIGATÓRIO)

A tela nunca deve sofrer transições bruscas (*layout shifts*). Mudanças de estado abruptas quebram o fluxo de concentração do usuário.

- Todo e qualquer modal, painel lateral (*slide-out*), dropdown, ou alteração de estado em listas DEVE ser animado com **Framer Motion**.
- Utilize transições do tipo `spring` (com `stiffness` elevado e `damping` suave) para garantir um feedback tátil, responsivo e físico.

## 3. MICRO-INTERAÇÕES (ESTADO VIVO)

A interface deve parecer responsiva sob o ponteiro do mouse ou teclado.

- **Feedback de Ação:** Botões e Cards interativos devem possuir micro-animações (ex: `active:scale-95` via Tailwind ou Framer Motion) e transições de opacidade (`transition-all duration-200`).
- **Acessibilidade de Foco:** Utilize os anéis de foco do Tailwind (`ring-2 ring-offset-2 ring-ring`) para indicar claramente onde a ação do teclado se encontra a qualquer momento.

## 4. DIVULGAÇÃO PROGRESSIVA (LAZY RENDERING)

Para evitar exaustão visual e estrangulamento de memória RAM (V8 Engine):

- Os blocos de código gerados pelo agente num chat devem aparecer inicialmente como **texto simples colorido** (syntax highlighting super leve).
- Apenas mediante ação explícita do usuário (ex: clique em "Inspecionar/Editar"), a interface deve se expandir espacialmente e instanciar o **Monaco Editor** para validação profunda.
- A instância pesada do Monaco Editor DEVE ser destruída da memória (unmount) assim que o usuário fechar a visualização.

## 5. REGRAS DE COMPONENTES COMPLEXOS

- **Workflow / Canvas Pessoal:** Utilize EXCLUSIVAMENTE o **React Flow**. Os nós lógicos dos agentes devem manifestar-se espacialmente aqui (ocultando logs crus). Nós devem ter cantos arredondados, *soft shadows* e ligações visuais claras.
- **Kanban Pessoal / Gestão de Tarefas:** Utilize EXCLUSIVAMENTE o **Pragmatic Drag and Drop** (Atlassian). O arrasto de cartões deve ser assíncrono para garantir que a renderização da UI a 60fps não seja engasgada por chamadas IPC ao Rust.
