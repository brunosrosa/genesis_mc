---
id: "ADR-005"
title: "ADR-005-RAG-Temporal"
version: 2.0
status: Ativo_Inegociavel
epic: "Memória"
description: "Erradica GraphRAGs pesados. Adota B-Trees no LanceDB, tags STABLE/EVOLVING e extração matemática O(1) na CPU contra a Cegueira Temporal."
---

### ADR-005: RAG Temporal, Filtros Escalares e Combate à Cegueira Temporal

#### Status
Aceito (Ativo, Inegociável e Fundacional para Arquitetura SODA V4)

#### Contexto Técnico e Ameaça Operacional (A Cegueira Temporal)
A busca vetorial ingênua em RAG (Retrieval-Augmented Generation) convencional sofre de "Cegueira Temporal" e "Recency Bias" (Viés de Recência). Em um RAG puramente baseado em similaridade de cosseno, um documento obsoleto de três anos atrás pode obter uma pontuação semântica maior do que uma regra atualizada ontem, envenenando fatalmente a resposta do agente [2]. 

A literatura acadêmica tenta mitigar isso introduzindo redes neurais monstruosas como *Temporal Graph RAG* (TG-RAG) ou *TimeRAG*, que delegam ao LLM a tarefa de organizar cronologias [1, 2]. Executar enxames de agentes para montar grafos temporais apenas para calcular fusos horários é um *overengineering* patético que asfixia os 6GB de VRAM da RTX 2060m e bloqueia o *Event Loop* do Tokio [1]. O SODA necessita de precisão cronológica cirúrgica, em milissegundos, sem estressar a dGPU.

#### Decisão Arquitetural (A Matriz Temporal O(1))
Fica sumariamente proibido o uso de LLMs ou GraphRAGs pesados para indexação primária de tempo. O SODA transforma o tempo em uma propriedade física filtrável, adotando as seguintes camadas pragmáticas:

**Módulo 1: Extração Temporal Nativa na CPU**
*   Toda inferência de datas a partir de linguagem natural (ex: "semana passada", "ontem") será resolvida estritamente na CPU (Intel i9) utilizando código Rust de altíssima performance.
*   Fica mandatória a adoção de bibliotecas matemáticas nativas como `temps` e `natural-date-parser` para garantir latência de extração de `1ms` em $\mathcal{O}(1)$ [4].
*   O SLM local (ex: Phi-4-mini) será acionado via chamadas de ferramentas (`<|tool|>`) exclusivamente como *fallback* de segurança em casos de ambiguidade extrema que os parsers estáticos não consigam resolver [4].

**Módulo 2: B-Trees Escalares e Bypass Vetorial (LanceDB)**
*   A linha do tempo do usuário será ancorada fisicamente no banco de dados. Fica imposto o uso de índices **BTREE** nas colunas de data do **LanceDB** [4].
*   A busca deverá executar pré-filtros *Hard SQL* escalares *antes* da comparação vetorial.
*   Para fatias de tempo muito restritas, onde a filtragem prévia deixa poucos vetores (causando colapso de índice no ANN), é **obrigatória** a chamada da função `bypass_vector_index()`, forçando uma varredura exata de similaridade (k-NN exato) na fatia restante, garantindo 100% de recall [4, 5].

**Módulo 3: Imunidade ao Viés de Recência (STABLE vs EVOLVING)**
*   Para evitar que fofocas ou informações voláteis sobreponham conhecimentos fundacionais, o sistema de indexação adotará uma categorização dicotômica estrita.
*   Conhecimentos consolidados receberão a tag **`STABLE`**, garantindo permanência no contexto [5].
*   Conhecimentos transitórios ou dinâmicos receberão a tag **`EVOLVING`**, sujeitos ao decaimento temporal e poda orgânica [5].
*   A ingestão de novos documentos utilizará Busca Híbrida e "Contextual Chunks" para enriquecer o payload no banco [4].

#### Consequências Operacionais e Defesa contra o Slop (Trade-offs)
*   **Impacto Positivo:** Consumo de VRAM zerado para cálculos temporais, liberando a placa de vídeo exclusivamente para raciocínio semântico [6]. A precisão de resposta a perguntas temporais atinge o estado da arte sem alucinação, já que a filtragem é matemática e não generativa. O tempo de resposta permanece sub-milissegundo na CPU.
*   **Impacto Negativo (Rigidez de Ingestão):** O *pipeline* de ETL cognitivo (Fase 0/1) se torna brutalmente mais engessado. Para o sistema funcionar, cada documento, arquivo ou *log* obrigatoriamente deve ter suas datas "parseadas" e estruturadas impecavelmente *antes* da inserção no LanceDB. Se o parser falhar na ingestão, o arquivo ficará fora do tempo.
