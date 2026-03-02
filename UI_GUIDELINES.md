# 🎨 UI GUIDELINES: Acessibilidade Cognitiva e Fluxo Espacial

A interface do Genesis não é um website; é um **Sistema Operacional Visual**. O design deve respeitar mentes de alto desempenho, eliminando a fadiga de logs e traduzindo estados de máquina em abstrações espaciais fluidas.

## 1. CORE DESIGN SYSTEM (A FUNDAÇÃO)

- **Framework Base:** Tailwind CSS v3/v4.
- **Blocos de Construção:** Shadcn UI (copiado diretamente para `src/components`, mantendo total propriedade do código).
- **Esquema de Cores:** Estritamente baseado nas variáveis semânticas do Shadcn (ex: `bg-background`, `text-primary`, `border-border`). **NUNCA** utilizar cores fixas (ex: `bg-red-500`), assegurando suporte perfeito ao Modo Escuro.

## 2. A LEI DO MOVIMENTO (FRAMER MOTION OBRIGATÓRIO)

O ecrã nunca deve sofrer transições bruscas (layout shifts). As mudanças de estado abruptas quebram o fluxo de concentração.

- TODO E QUALQUER modal, painel lateral (slide-out), dropdown ou alteração de estado no Kanban DEVE ser animado com **Framer Motion**.
- Utilizar a transição do tipo `spring` (com `stiffness` elevado e `damping` suave) para garantir um sentimento físico e tátil.

## 3. MICRO-INTERAÇÕES (ORIGIN UI)

A interface deve parecer responsiva e "viva" sob o rato do utilizador.

- **Botões e Cards:** Devem possuir a classe `active:scale-95` e transições suaves de opacidade (`transition-all duration-200`).
- **Foco:** Utilizar os anéis de foco do Tailwind (`ring-2 ring-offset-2 ring-ring`) para indicar claramente onde a ação do teclado se encontra.

## 4. DIVULGAÇÃO PROGRESSIVA (LAZY RENDERING)

Para evitar exaustão visual e derretimento da RAM:

- Os blocos de código gerados pelo agente num chat devem aparecer inicialmente como texto simples colorido (syntax highlighting leve).
- Apenas mediante ação do utilizador (clique em "Editar/Diff"), a interface deve expandir-se espacialmente e instanciar o **Monaco Editor** / CodeMirror para validação profunda.
- Destruir a instância pesada do editor da memória assim que o utilizador fechar a visualização.

## 5. REGRAS DOS COMPONENTES COMPLEXOS

- **Workflow / Canvas:** Utilizar exclusivamente o `React Flow`. Os nós devem possuir cantos arredondados, sombras difusas (soft shadows) e ligação visual clara. As operações lógicas dos agentes devem manifestar-se espacialmente aqui, ocultando os logs crus.
- **Kanban Pessoal:** Utilizar exclusivamente o `Pragmatic Drag and Drop`. Assegurar que os cartões não encravam a renderização da interface, permitindo arrasto assíncrono.
