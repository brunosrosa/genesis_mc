Algumas colunas "descritivas" voltaram a "Truncar", ao menos percebi isso nas colunas `visao_do_enxame`, `executive_verdict`, `risco_principal` e `risco_linha_vermelha` do "aaif-goose / goose".

Já as colunas `acao_de_canibalizacao` e `tipo_integracao` vieram como "UNKNOWN". E isso me incentivou a definir melhor os ENUMs. Segue:

`classificacao_terminal`
Esta coluna é o "Veredito de Adoção" final, a prateleira estratégica onde a solução vai dormir perante o sistema. 
Hoje ele coloca "APROVADO_COM_RESSALVAS", "APROVADO_PARA_PRODUCAO" ou "REJEITADO_DESCARTE"... mas vamos evoluir.
O nosso motor deve aceitar estritamente os seguintes ENUMs:
- "STACK_CORE_PLANO_A1": Opção de fundação principal e absoluta (ex: Rust, SQLite, Svelte).
- "STACK_CORE_PLANO_A2": Alternativa direta ou camada complementar de mesma arquitetura.
- "STACK_CORE_PLANO_B": A contingência caso o Plano A falhe.
- "INTEGRATE_AS_COMPONENT": Vamos usar o código ou binário como está (ex: uma extensão nativa em C/Rust).
- "ABSORB_PARTIALLY": Opção onde pode haver a Canibalização Cirúrgica de partes específicas, descartando o resto do monólito.
- "ABSORB_CONCEPT": Opção onde o código é tóxico, mas a ideia é genial e será reescrita do zero.
- "USE_AS_INSPIRATION_ONLY": Apenas para guiar padrões visuais de UX/UI, lógicas ou regras de negócio.
- "REJECT": Tóxico ou inútil para o Sistema.
- "SHORT-CIRCUIT": Repositório morto, inacessível ou falha mecânica (Fail-Fast).

`acao_de_canibalizacao`:
Esta coluna dita o que exatamente faremos com a ferramenta e onde a peça amputada será "soldada" no nosso sistema.
- "Data Model / Schema": Quando o código em si não importa, mas vamos roubar a forma como o banco de dados deles foi desenhado (ex: a topologia de grafos ou tabelas relacionais)
- "Prompt / Heuristic Seed": Quando vamos roubar as regras de sistema (System Prompts), a taxonomia de agentes ou os contratos de I/O em texto puro
- "Protocol / Standard": Quando adotaremos um padrão de comunicação genial deles (ex: o formato MCP, A2UI ou o próprio protocolo de Zero-Copy)
- "Concept": Quando a ideia é brilhante, mas a tecnologia é "lixo tóxico" (Node.js/Electron), então nós extraímos apenas o conceito matemático/arquitetural para reescrevermos do zero em Rust
- "UX Pattern": Quando vamos canibalizar um padrão de usabilidade, fluxo interativo ou animação passiva (CSS/Svelte), sem importar o código original.
- "Canvas Refinement": A ferramenta aprimora ou adiciona uma funcionalidade inteligente a um Canvas que já temos desenhado na nossa topologia.
- "New Canvas": A ferramenta traz um modo de pensar e organizar dados tão inovador que justifica a criação de uma tela interativa totalmente nova (Ilha WebGL/Svelte 5) no SODA.
- "Cognitive Layer": Projetos que fornecem inteligência de memória, RAG, injeção de contexto ou familiaridade sistêmica.
- "Infra Capability": Uma capacidade infra-semântica bruta, como extração de dados, OCR local, ou parsing de AST (ex: jcodemunch ou Kreuzberg).
- "Technical Runtime": Motores pesados de execução, como LLMs rodando no edge, compiladores ou rotinas puras de infraestrutura em Rust
- "Sandbox": Ferramentas de isolamento, micro-VMs ou auto-enjaulamento de processos (ex: Wasmtime, Landlock, cgroups) que garantem a segurança do Treino de Gravidade
- "Plugin": Módulos menores que podem ser engatados e desengatados do sistema (frequentemente usando WASM).
- "External Contract": Quando o SODA apenas interage com a ferramenta através de um contrato bem delimitado, sem absorvê-la internamente.
No Absorption: Quando o projeto não fornece nada fisicamente extraível ou útil que justifique esforço de engenharia.

`tipo_integracao`: 
Esta coluna define a topologia de injeção física no sistema. Como o código amputado vai rodar no nosso ecossistema? As opções estritas são:
- "Biblioteca / Crate Nativa" (Integrado direto no binário Rust)
- "Sidecar Efêmero" (Rodando em Wasmtime ou Micro-VM isolada com SIGKILL)
- "Daemon / Background Service" (Processo contínuo, evitado sempre que possível)
- "App Nativo / CLI Independente"
- "Middleware / Proxy"

`categoria_arquitetural`
O projeto/repositório deve se encaixar em uma das nossas frentes:
- "CanvasUI": Projetos que entregam aplicações inteiras, fluxos ou telas
- "UILibrary": Bibliotecas puramente visuais e estéticas (Svelte, Tailwind, WebGL)
- "Memoria": Tudo que envolve RAG, retenção temporal, grafos ou vetores
- "Roteamento": Gateways, proxies, orquestração de LLMs e FinOps
- "Orquestracao": Motores de fluxo, loops agênticos, máquinas de estado
- "Seguranca": Isolamento, sandboxing, micro-VMs
- "Infraestrutura": Parsers brutos, telemetria bare-metal, utilitários de baixo nível
- "Tooling": Ferramental de desenvolvedor ou utilitários para agentes

`horizonte_extracao` (O Timing da Canibalização)
Define quando o Agente da IDE deverá atacar esse repositório no nosso Roadmap:
- "IMEDIATO": Dia 1 do MVP (ex: fundações do SQLite ou Rust).
- "CURTO_PRAZO": Ajustes pós-MVP.
- "CURTO_MEDIO_PRAZO": Projetos complexos que já precisamos preparar o terreno
- "MEDIO_PRAZO": Meses à frente (escala do ecossistema).
- "LONGO_PRAZO": Roadmap futuro de engenharia profunda.
- "REFERENCIAL_TEORICO": Manifestos ou "Awesome lists" que inspiram a filosofia, mas nunca virarão código
- "NUNCA": Lixo descartado

`discipline_dependency`
Esta coluna é fantástica para usuários com TDAH/2e. Ela avalia o quanto a ferramenta quebra se o usuário for preguiçoso ou indisciplinado
- "Nenhuma: Funciona invisível em background. O usuário nem percebe.
- "Baixa": Aceita inputs bagunçados e se corrige sozinha.
- "Média": Exige seguir regras básicas (ex: escrever prompts mínimos).
- "Alta": Exige mudança de hábito ativa.
- "Crítica": Se o usuário esquecer um passo, corrompe o banco ou perde dados (isso é repudiado no SODA)

`architectural_topology`
Como o software foi desenhado fisicamente? Se precisarmos isolá-lo, o Agente precisa saber a topologia
- "Monolith"
- "Modular"
- "Layered"
- "Contract-Driven"
- "Runtime-Centric"
- "Event-Driven"
- "Graph-Centric"
- "Pipeline-Centric"
- "Hybrid"

`capability_nature_primary`
O que essa ferramenta faz na essência semântica? Isso ajuda a agruparmos PRDs por funcionalidade bruta depois.
- "Context"
- "Memory"
- "Perception"
- "Expression"
- "Execution"
- "Observation"
- "Documentation"
- "Planning"
- "Curation"
- "Identity"
- "Infrastructure"
- "Multimodal IO"
- "Sandbox"
- "Serving"
- "Retrieval"
- "Synchronization"

Também o `declared_description` é outra coluna que muitas vezes vem bem ruim. Sendo que era pra ela ser bem fácil, se o repositório tiver um "ABOUT", logo de cara, é algo que já dá pra usar, tentar achar/pegar do readme é apenas uma segunda opção, ou tentar buscar do título da página do Repositório/GitHub.