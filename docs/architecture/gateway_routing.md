# 02_AGENT_GATEWAY_ROUTING: O Sistema Nervoso e Protocolo MCP

**Versão:** 3.1 (Definitiva - Zero-Trust Network & FinOps)
**Status:** ATIVO E INEGOCIÁVEL
**Alvo da Leitura:** Agentes Orquestradores, Engenheiros de Rede, DevOps e Arquitetos de Segurança.

## 1. O FIM DAS APIs REST: A ERA DO MODEL CONTEXT PROTOCOL (MCP)

No ecossistema Genesis Mission Control (SODA), a comunicação estática via APIs RESTful tradicionais foi descontinuada. Sistemas Agênticos necessitam de conexões bidirecionais, persistência de estado (Stateful Continuation) e passagem de contexto dinâmico.

Para alcançar isso, o SODA adota o **Model Context Protocol (MCP)** como sua "Pedra de Roseta" e barramento universal (o "USB-C" da Inteligência Artificial). Todas as integrações — sejam buscas em bancos de dados (SQLite/LanceDB), automação de navegador, varredura de diretórios ou acesso a CLIs em nuvem — devem obrigatoriamente operar encapsuladas como servidores MCP.

## 2. O PROXY L7 EM RUST: AGENTGATEWAY NATIVO

Deixar o tráfego MCP ser gerenciado por middlewares escritos em Node.js ou Python (como o antigo `mcpv`) cria contenção de rede e asfixia a thread principal (Event Loop Starvation). O SODA exige controle absoluto de rede no _Bare-Metal_.

- **O Roteador Maestro (AgentGateway):** Integramos a lógica do projeto open-source _AgentGateway_ (CNCF) compilada nativamente dentro do nosso daemon Rust.
- **Modo Standalone:** Ele opera como um Proxy de Nível 7 (L7) isolado, escutando tráfego inter-processos (IPC) na porta TCP 3000, com portas de diagnóstico e telemetria (Prometheus) separadas.
- **Zero-Dependência de Nuvem:** O gateway despacha mensagens assíncronas diretamente pelas vias do sistema operacional (usando os canais `std::io` do Tokio) sem depender de resolvedores externos.

## 3. O PARADIGMA DA AMARRAÇÃO TARDIA (LATE-BINDING) E PREVENÇÃO DE "TOOL BLOAT"

A prática ingênua de inicializar um LLM injetando os esquemas JSON de 50 a 100 ferramentas simultaneamente no prompt causa a **Saturação de Ferramentas (Tool Bloat)**. Isso devora a estreita margem de 6GB de VRAM da RTX 2060m e fragmenta a atenção do modelo (Context Rot), gerando alucinações sistêmicas.

- **O Catálogo Cego:** O agente SODA inicia com um inventário vazio de ferramentas na VRAM. Todas as descrições de ferramentas residem passivamente em disco, indexadas em uma tabela **SQLite FTS5 (BM25)** acoplada ao roteador em Rust.
- **Divulgação Progressiva:** Quando o modelo deduz uma intenção primária (ex: "Preciso pesquisar um PDF"), o Rust faz uma busca lexical sub-milissegundo em $\mathcal{O}(1)$, recupera apenas a assinatura esquemática daquela ferramenta específica (Top-1 ou Top-3) e injeta o JSON dinamicamente na janela de contexto.
- **Expurgo Atômico (Context Clearing):** Tão logo o LLM utiliza a ferramenta e obtém o resultado, o esquema da ferramenta é apagado do contexto ativo, devolvendo a memória à GPU imediatamente.

## 4. BLINDAGEM ZERO-TRUST E A VÁLVULA INTELIGENTE (CEL)

O protocolo MCP permite _Sampling_ (onde o servidor pode enviar requisições reversas para o cliente). Assumimos que todo script e servidor externo é inerentemente malicioso.

- **Typestate Pattern (Rust):** Transições de estado de rede não usam booleanos (`is_safe = true`). Utilizamos tipagem estrita do compilador Rust. Uma conexão não validada é um tipo de dado que _fisicamente_ não possui os métodos para acessar o disco.
- **Válvula Inteligente via CEL:** O manifesto do AgentGateway (`gateway-config.yaml`) embute o motor **Common Expression Language (CEL)**. Configura-se um firewall interno que impede chamadas iterativas em loop infinito ou bloqueia a execução de parâmetros textuais perigosos antes mesmo que eles cheguem à Sandbox do Wasmtime.

## 5. ROTEAMENTO HÍBRIDO FINOPS: A EQUAÇÃO PARETO-BANDIT

A GPU RTX 2060m (6GB VRAM) é incapaz de processar tarefas de raciocínio profundo de longo horizonte com contextos de 100.000 tokens. O envio inconsequente de todas as tarefas para APIs comerciais pagas (REST) gera o _Inference Bill Shock_ (falência financeira corporativa).

O SODA implementa uma **Máquina de Estados Finitos (FSM)** no Rust para Roteamento Híbrido, baseada no algoritmo **ParetoBandit**:

1. **Nível 0 (Triagem Mecanicista CPU):** O prompt é avaliado em $< 50ms$ por um modelo ultraleve (ex: Qwen3-0.6B) ou via _Separabilidade de Fisher_ na CPU i9. Se for trivial, resolve-se localmente a custo US$ 0,00.
2. **Nível 1 (Edge Node GPU):** Tarefas de código seguras e contextos curtos ($< 8000$ tokens) são roteadas para a RTX 2060m local.
3. **Nível 2 (Subscription Hacking / Cloud Fallback):** Se a complexidade for extrema ou a VRAM estiver cheia (risco de Out-Of-Memory), o roteador Rust ativa **Sidecars Efêmeros em Docker** conectados aos clientes oficiais via linha de comando (_Gemini CLI_ ou _Claude Code CLI_).
    - _A Tática:_ O SODA "abusa" das assinaturas mensais Flat-Rate pagas pelo usuário (ex: Google One AI Premium), empurrando a carga de trabalho massiva pela CLI em background via MCP, poupando a máquina local de derreter e reduzindo o faturamento marginal por token a zero.

_Fim da Especificação de Roteamento L7. A barreira de rede e a economia sistêmica estão seladas._