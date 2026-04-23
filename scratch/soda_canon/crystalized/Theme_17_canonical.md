# SODA: Manual Canônico de Arquitetura de Dados Soberanos (Genesis MC)

## 1. Introdução

Este manual descreve a arquitetura SODA (Sovereign Operating Data Architecture), um sistema projetado para operar com eficiência e segurança. A SODA se baseia em um conjunto rigoroso de tecnologias e princípios para garantir desempenho, modularidade e manutenibilidade.

## 2. Princípios Fundamentais da Arquitetura

### 2.1. Backend em Rust (Tokio)

O backend da SODA é inteiramente construído em Rust, utilizando o framework assíncrono Tokio. Esta escolha garante:

*   **Performance Bare-Metal:** Rust oferece controle de baixo nível sobre a memória e a execução, permitindo otimizações que se aproximam do desempenho de código nativo.
*   **Segurança de Memória:** O sistema de tipos e o modelo de propriedade do Rust previnem erros comuns de memória em tempo de compilação, aumentando a robustez.
*   **Concorrência Eficiente:** Tokio fornece um runtime assíncrono robusto para lidar com operações de I/O intensivas e concorrência de forma eficiente, sem a sobrecarga de threads tradicionais.

### 2.2. Frontend em Svelte 5 + Tauri v2

O frontend da SODA é desenvolvido com Svelte 5 para a interface do usuário e Tauri v2 para a camada de desktop.

*   **Svelte 5:** Utiliza um modelo reativo compilado que gera código JavaScript altamente otimizado, evitando a necessidade de um Virtual DOM (VDOM) e proporcionando um desempenho superior.
*   **Tauri v2:** Permite a criação de aplicações desktop nativas utilizando tecnologias web (HTML, CSS, JavaScript/TypeScript) de forma segura e eficiente. Ele se integra diretamente com o backend Rust, permitindo a comunicação via IPC (Inter-Process Communication) de cópia zero.
*   **Interface Passiva:** O frontend é estritamente uma interface passiva. Toda a lógica de negócios, processamento de dados e tomada de decisão reside no backend Rust. O frontend é responsável apenas pela apresentação visual e pela captura de interações do usuário, que são então repassadas ao backend.

### 2.3. Comunicação Backend-Frontend (IPC Zero-Copy)

A comunicação entre o backend Rust e o frontend Svelte/Tauri é realizada através de um mecanismo de IPC (Inter-Process Communication) de cópia zero. Isso significa que os dados são transferidos entre os processos sem a necessidade de serialização e desserialização intermediárias, minimizando a latência e o consumo de recursos.

## 3. Conceito de "Agent Skills" e sua Implementação na SODA

A arquitetura SODA adota o padrão "Agent Skills", um conceito onde funcionalidades especializadas são encapsuladas em módulos reutilizáveis. Este padrão, popularizado por plataformas como Antigravity e Claude Code, é fundamental para a modularidade e extensibilidade da SODA.

### 3.1. O Padrão `SKILL.md`

As "skills" são definidas utilizando o formato `SKILL.md`, que consiste em:

*   **Metadados (YAML Frontmatter):**
    *   `name`: Um identificador único para a skill.
    *   `description`: Uma descrição concisa que o agente utiliza para determinar a relevância da skill para uma determinada tarefa. Esta é a chave para a ativação automática da skill.
    *   Outros metadados opcionais como `allowed-tools`, `ontology`, `preconditions`, `postconditions`.
*   **Corpo da Skill (Markdown):** Contém as instruções detalhadas, lógica de execução, exemplos e referências para scripts ou outros recursos.

### 3.2. Mecanismo de Carregamento Progressivo (Progressive Disclosure)

As skills operam sob o princípio de "Progressive Disclosure":

1.  **Descoberta (Level 1):** Na inicialização, o agente carrega apenas os metadados (`name` e `description`) de todas as skills disponíveis. Isso mantém o contexto inicial leve e rápido.
2.  **Ativação (Level 2):** Quando o agente determina que uma skill é relevante para a tarefa atual com base na `description`, ele carrega o corpo completo do `SKILL.md`.
3.  **Execução (Level 3):** Se o corpo da skill referenciar scripts ou arquivos de referência, estes são carregados sob demanda, garantindo que apenas o necessário seja processado.

### 3.3. Estrutura de Diretórios das Skills na SODA

As skills na SODA seguem uma estrutura de diretórios padronizada:

```
.agents/skills/
└── <nome-da-skill>/
    ├── SKILL.md       # Definição principal da skill (obrigatório)
    ├── scripts/       # Scripts executáveis (Python, Bash, etc.)
    │   └── meu_script.py
    ├── references/    # Documentação, templates ou dados de referência
    │   └── guia.md
    └── assets/        # Recursos estáticos (imagens, logos)
        └── logo.png
```

*   **Escopo:** As skills podem ser definidas em dois escopos:
    *   **Workspace-specific:** Dentro do diretório do projeto (`.agents/skills/`). Ideal para fluxos de trabalho específicos do projeto.
    *   **Global:** Em um local centralizado acessível por todos os workspaces (ex: `~/.soda/skills/`). Adequado para utilitários de uso geral.

### 3.4. Integração com Ferramentas e MCP (Model Context Protocol)

*   **Scripts:** Scripts (Python, Bash, etc.) dentro do diretório `scripts/` podem ser executados diretamente pelo agente, permitindo a automação de tarefas complexas.
*   **MCP:** Embora a SODA não utilize MCP diretamente em sua arquitetura base (preferindo a comunicação Rust-Tauri), o conceito de "ferramentas" e "protocolos" para interagir com sistemas externos é análogo. Skills podem ser projetadas para orquestrar chamadas a APIs externas ou serviços, que seriam o equivalente funcional do MCP.

## 4. Otimizações e Considerações de Hardware

A arquitetura SODA é projetada para ser consciente do hardware subjacente, visando maximizar o desempenho:

*   **CPU (AVX2):** O código Rust no backend pode ser compilado com otimizações específicas para a CPU, como o uso de instruções AVX2, quando aplicável, para acelerar cálculos intensivos.
*   **GPU (RTX 2060m / llama.cpp mmap):** Para cargas de trabalho que envolvem inferência de modelos de linguagem localmente (embora a lógica principal da SODA resida em Rust), a utilização de `llama.cpp` com `mmap` pode ser explorada para otimizar o uso da memória da GPU (como uma RTX 2060m), especialmente para carregar modelos maiores de forma eficiente.
*   **Gargalos de Barramento (iGPU):** É crucial estar ciente das limitações de desempenho impostas pelo barramento de dados, especialmente ao utilizar a iGPU (Integrated Graphics Processing Unit). Transferências de dados excessivas entre a CPU e a iGPU podem se tornar um gargalo. A arquitetura SODA minimiza essas transferências através da lógica centralizada em Rust e comunicação IPC eficiente.

## 5. Auditoria Crítica e Pontos de Atenção

### 5.1. Eliminação de Arquiteturas Não Conformes

Qualquer menção ou proposta de arquiteturas que utilizem:

*   React (exceto como base para Svelte, se aplicável em contextos de aprendizado, mas não como framework principal)
*   Node.js daemons
*   Electron
*   Virtual DOM (VDOM)
*   Server-Side Rendering (SSR) como Next.js

deve ser sumariamente ignorada e removida do escopo da SODA. A SODA adere estritamente à pilha Rust/Svelte/Tauri.

### 5.2. Fragilidades Potenciais e Mitigações

*   **Complexidade da Descrição da Skill:** A eficácia da ativação automática das skills depende criticamente da qualidade e especificidade da `description` no `SKILL.md`. Descrições vagas ou mal formuladas podem levar à falha na ativação.
    *   **Mitigação:** Enfatizar a escrita de descrições claras, que incluam exemplos de frases de gatilho e descrevam precisamente o que a skill faz e quando deve ser usada.
*   **Gerenciamento de Dependências de Scripts:** Skills que dependem de scripts externos (Python, Bash) podem introduzir complexidade no gerenciamento de dependências e ambientes de execução.
    *   **Mitigação:** Utilizar ambientes virtuais (como `venv` para Python) ou contêineres (Docker, se aplicável em cenários de desenvolvimento/deploy mais complexos) para isolar as dependências dos scripts. A documentação da skill deve especificar claramente os pré-requisitos.
*   **Segurança de Skills:** Skills podem conter código executável e, portanto, representam um vetor de segurança. A instalação de skills de fontes não confiáveis pode ser perigosa.
    *   **Mitigação:** A SODA deve priorizar a instalação de skills de fontes confiáveis e auditadas. Uma política de segurança clara deve ser estabelecida, desencorajando a execução de scripts arbitrários e a exposição de segredos. A funcionalidade `allowed-tools` (se implementada ou adaptada) pode ser usada para restringir as capacidades de uma skill.
*   **Escalabilidade do Frontend:** Embora o frontend seja passivo, um grande número de atualizações de UI ou interações complexas pode, teoricamente, sobrecarregar o canal IPC ou o próprio Svelte.
    *   **Mitigação:** Manter a lógica de apresentação o mais leve possível. O processamento pesado deve sempre ser delegado ao backend Rust. O uso de Svelte 5 com seu modelo de compilação otimizado ajuda a mitigar isso.

## 6. O "Porquê" da Arquitetura SODA

A arquitetura SODA é motivada pela necessidade de um sistema de dados soberano que seja simultaneamente:

*   **Performático:** Através do uso de Rust e otimizações de baixo nível, a SODA visa alcançar o máximo desempenho possível, minimizando a latência e o consumo de recursos.
*   **Modular e Extensível:** O padrão "Agent Skills" permite que novas funcionalidades sejam adicionadas de forma isolada e reutilizável, sem impactar o núcleo do sistema. Isso facilita a manutenção e a evolução contínua.
*   **Seguro e Robusto:** A segurança de memória do Rust e a arquitetura de comunicação IPC de cópia zero reduzem a superfície de ataque e a probabilidade de erros críticos.
*   **Eficiente em Contexto:** O carregamento progressivo das skills garante que o modelo de IA (se utilizado em conjunto com a SODA) não seja sobrecarregado com informações irrelevantes, otimizando o uso de tokens e a precisão do raciocínio.
*   **Consciente do Hardware:** Ao considerar as limitações e capacidades do hardware subjacente (CPU, GPU, barramentos), a SODA busca extrair o máximo de desempenho de forma inteligente.

Em essência, a SODA representa uma convergência de tecnologias de ponta para criar uma plataforma de dados soberana que prioriza desempenho, segurança e modularidade, utilizando um modelo de extensão baseado em "skills" para uma adaptabilidade sem precedentes.