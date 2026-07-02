# ADR-023-Grafo-De-Proximidade

## Status
Aceito (Ativo e Inegociável)

## Contexto
Durante investigações em repositórios complexos e acompanhamento de tarefas causais multifacetadas, agentes autônomos de IA frequentemente cometem erros de desorientação conceitual (alucinação lógica). Ao buscar referências no RAG, o LLM falha ao conectar dependências estruturais distantes no código ou pula para arquivos homônimos sem qualquer associação causal real no sistema. Isso ocorre pela falta de uma representação de grafo rigorosa que amarre a proximidade semântica, conceitual e sintática das lógicas locais do projeto.

## Decisão
Estabelecer o **LadybugDB** como a camada nativa de grafos local do SODA, atuando ativamente como o **Proximity Agent (Agente de Proximidade)** governado pelo ecossistema Rust:
1. **Ativação por Proximidade Relacional:** As buscas de contexto não dependem puramente de busca vetorial densa. O LadybugDB calcula caminhos geodésicos no grafo de conhecimento local para ativar nós que possuem dependências causais diretas com o assunto abordado, montando uma árvore relacional inquebrável.
2. **Especialização CROW (Code Reasoning Ontological Weaver):** Focado na costura ontológica profunda do código-fonte. Mapeia a hierarquia de herança, cadeias de chamadas de métodos e imports da AST gerados pelo parser AST nativo, blindando o raciocínio sintático profundo.
3. **Especialização FALCON (Fast Access Logical Context Network):** Rede de indexação ultrarrápida na CPU focada no mapeamento de intenções de conversação do usuário, links locais e tarefas em andamento. Executa varreduras sub-milissegundo cruzando dados com o SQLite L2 para sanar dúvidas instantâneas e bloquear desvios focais.
4. **Resiliência Bayesiana contra Poisoning:** A costura e criação de novos nós de memória semântica no LadybugDB passam por uma avaliação Bayesiana de coerência antes de serem gravados definitivamente, impedindo o envenenamento do RAG por alucinações repetidas dos modelos de inferência estocásticos da nuvem.

## Consequências
- **Erradicação de Alucinações de Dependência:** O agente de IA compreende perfeitamente a cadeia causal sistêmica, descobrindo arquivos e funções dependentes que seriam invisíveis em RAGs tradicionais.
- **Isolamento de Blast Radius:** Mutações e planejamentos arquiteturais medem o impacto direto nos nós vizinhos do grafo antes de realizar alterações no disco, prevenindo efeitos colaterais indesejados.
- **Higiene Concorrente:** A estrutura do LadybugDB permite travamento de segurança em nós específicos do grafo durante a execução concorrente de enxames de subagentes.

## Restrições Bare-Metal
- **Profundidade Rígida de Consulta (max_depth):** Varreduras de relacionamentos no grafo LadybugDB são limitadas a uma profundidade máxima de **3 saltos (hops)** na dGPU/CPU para impedir estouro de memória (Out-Of-Memory) e contenção de processamento.
- **Latência de Retorno FALCON:** O resgate rápido de dados cruzados FALCON na CPU i9 deve executar em menos de **5ms**.
- **Consistência Semântica:** A criação de nós efêmeros e relacionamentos transientes de sessão é limpa na ociosidade, preservando apenas arestas de relevância matemática estável.
