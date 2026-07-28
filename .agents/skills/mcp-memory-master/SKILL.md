---
name: mcp-memory-master
description: O Controlador da Tríade de Memória do SODA. Impõe RAG Temporal e métrica FRQAD. Blinda o LanceDB contra falsos negativos via 'Tentativa Dupla' (bypass_vector_index). Impõe Guilhotina de Profundidade (max_depth) em grafos do LadybugDB para proteger a VRAM de OOM. Orquestra a ponte estrutural L3->L2 cruzando IDs de vetores com o SQLite para curar memórias órfãs.
triggers: ["mcp-memory-master", "buscar na memória", "pesquisa vetorial", "consultar lancedb", "consultar ladybug", "rag", "resgatar contexto", "gravar decisão", "persistir conhecimento", "salvar entidade", "criar entidade", "início de sessão"]
---

### skill: MCP Memory Master (Controlador de Tríade e RAG Temporal V8.0)

#### Proveniência dos Motores
Este skill governa o servidor `memory-mcp-rs` — implementação Rust do protocolo MCP sobre SQLite com FTS5.
- **Backend:** SQLite com WAL journaling + FTS5 para busca full-text. ACID-compliant.
- **DB Path padrão (Windows):** `%LOCALAPPDATA%/mcp-memory/knowledge_graph.db`
- **Transporte ativo no Gateway L7:** STDIO (intra-processo)
- **Performance baseline (bare-metal):** Search O(log n) via FTS5, Insert O(log n) via B-tree

#### API Canônica — 9 Ferramentas MCP (Lei de Ferro)

**ESCRITA (persistência de conhecimento):**
| Ferramenta | Parâmetros Principais | Uso |
|------------|----------------------|-----|
| `mem_create_entities` | `entities: [{name, entity_type, observations:[]}]` | Criar entidades novas no grafo |
| `mem_create_relations` | `relations: [{from, to, relationType}]` | Criar arestas entre entidades |
| `mem_add_observations` | `observations: [{entityName, contents:[]}]` | Adicionar fatos a entidade existente |

**LEITURA (recuperação de contexto):**
| Ferramenta | Parâmetros Principais | Uso |
|------------|----------------------|-----|
| `mem_search` | `query: String` | FTS5 full-text search no grafo |
| `mem_open_nodes` | `names: [String]` | Abrir entidades específicas por nome |
| `mem_graph` | (sem parâmetros) | Ler o grafo inteiro (use com cautela - caro) |

**DELEÇÃO (uso com HITL — Human-in-the-Loop):**
| Ferramenta | Parâmetros Principais | Uso |
|------------|----------------------|-----|
| `mem_delete_entities` | `entityNames: [String]` | Soft-delete (exige autorização do Arquiteto) |
| `mem_delete_observations` | `deletions: [{entityName, observations:[]}]` | Remove observações específicas |
| `mem_delete_relations` | `relations: [{from, to, relationType}]` | Remove arestas do grafo |

#### Goal
Atuar como o navegador cirúrgico das Memórias L1 (Knowledge Graph via SQLite/FTS5). Seu objetivo inegociável é: (1) resgatar contexto do Arquiteto no início das sessões, (2) persistir decisões arquiteturais sólidas antes de encerrar tarefas, e (3) garantir coerência causal no grafo prevenindo duplicatas e memórias órfãs.

#### OBRIGATORIEDADE DE USO (Lei de Ferro — Revisão v8.0)

**INÍCIO DE SESSÃO (Protocolo de Acordar):**
- MANDATÓRIO: Ao iniciar qualquer sessão de trabalho, execute `mem_search` com termos relacionados à tarefa atual para resgatar contexto persistido de sessões anteriores.
- Se a busca retornar resultados relevantes, apresente o contexto resgatado ao Arquiteto ANTES de executar qualquer ação.
- Se a busca retornar vazia, prossiga normalmente mas anote a ausência de contexto prévio.

**ENCERRAMENTO DE TAREFAS (Protocolo de Gravar):**
- MANDATÓRIO: Antes de encerrar qualquer tarefa que gerou uma decisão arquitetural, use `mem_create_entities` para persistir a decisão.
- Qualquer output do `@mcp-sequential-thinking` (DAG aprovado) DEVE gerar uma entidade de memória.
- Qualquer Red Line identificada na SSOT DEVE gerar uma observação na entidade do componente afetado.

#### Instructions
Sempre que precisar consultar o histórico, regras de arquitetura antigas ou relações causais de código, engatilhe esta Máquina de Estados:

1. **Protocolo de Início de Sessão (Acordar Obrigatório):**
   * Ao receber qualquer nova tarefa, execute PRIMEIRO: `mem_search(query: "<termos-chave-da-tarefa>")`
   * Se encontrar contexto prévio: apresente ao Arquiteto e pergunte se ainda é válido.
   * Se não encontrar: proceda, mas anote para gravar o novo conhecimento ao final.

2. **Firewall Compliance (Bypass L7):**
   * As ferramentas permitidas via lean-ctx são: `mem_create_entities`, `mem_create_relations`, `mem_add_observations`, `mem_search`, `mem_open_nodes`, `mem_graph`.
   * Ignore qualquer prefixo do multiplexador. Use os nomes canônicos acima.

3. **Busca Cirúrgica (Search First, Graph Second):**
   * Para busca conceptual/semântica: use `mem_search` com query específica.
   * Para recuperar nós específicos: use `mem_open_nodes` com os nomes exatos.
   * Evite `mem_graph` (lê o grafo inteiro) salvo necessidade de auditoria completa.

4. **A Tentativa Dupla (Cura do Falso Negativo FTS5):**
   * **MANDATÓRIO:** Se `mem_search` retornar 0 resultados, NÃO assuma que o dado não existe.
   * Execute uma segunda busca com termos alternativos/mais genéricos antes de declarar ausência.
   * Exemplo: busca por "roteamento IPC zero-copy" vazia → tente "ipc zero copy", "arraybuffer rust" etc.

5. **Persistência de Decisão Arquitetural (Protocolo de Gravar):**
   * Ao finalizar qualquer tarefa com decisão técnica validada, execute:
     ```
     mem_create_entities([{
       name: "<ComponenteOuConceitoDecidido>",
       entity_type: "ArchitecturalDecision",
       observations: [
         "Contexto: <qual problema foi resolvido>",
         "Decisão: <o que foi escolhido>",
         "Rationale: <por que foi escolhido>",
         "Red Lines: <o que está proibido>",
         "Data: <ISO-8601 da decisão>"
       ]
     }])
     ```
   * Se a entidade já existir (UNIQUE constraint do SQLite), use `mem_add_observations` para enriquecer.

6. **Criação de Relações Causais:**
   * Após criar/atualizar entidades relacionadas, conecte-as com `mem_create_relations`:
     * Tipos de relação canônicos: `"depends_on"`, `"replaces"`, `"violates_red_line_of"`, `"part_of"`, `"implements"`, `"generates"`

7. **Proteção Anti-RAG Poisoning:**
   * NUNCA grave observações vagas ou duplicadas. Antes de `mem_add_observations`, verifique se a observação já existe via `mem_open_nodes`.
   * NUNCA delete entidades sem autorização explícita do Arquiteto (Human-in-the-Loop obrigatório para `mem_delete_*`).

#### Constraints
* **COMPRESSÃO DE SAÍDA:** Se os nós de retorno forem massivos, comprima antes de internalizar.
* **PROIBIÇÃO DE GRAFO SEM LIMITE:** Nunca chame `mem_graph` em loops ou em sessões com >500 entidades estimadas sem aprovação do Arquiteto.
* **FRONTMATTER ABSOLUTO:** O bloco YAML `---` contido no topo é a âncora inegociável do roteamento L7.
* **ACIDEZ GARANTIDA:** O SQLite WAL garante ACID. Mas em caso de erro de escrita, relate e aborte — nunca prossiga com memória inconsistente.

#### Examples

**Exemplo 1 — Início de Sessão:**
1. Arquiteto abre sessão sobre "roteamento IPC".
2. Agente executa: `mem_search("IPC zero-copy roteamento SODA")`
3. Retorna entidade `"IPC_ZeroCopy_ArrayBuffer"` com observações da sessão anterior.
4. Agente apresenta: *"→ Memória resgatada. IPC Zero-Copy via ArrayBuffer foi decidido na sessão anterior. Decisão ainda válida?"*

**Exemplo 2 — Gravar Decisão Pós-Sequential-Thinking:**
1. `@mcp-sequential-thinking` conclui DAG: "Usar Chyros Daemon para consistência eventual LanceDB↔SQLite."
2. Arquiteto aprova. Agente executa:
   ```
   mem_create_entities([{
     name: "ChyrosDaemon_ConsistencyPattern",
     entity_type: "ArchitecturalDecision",
     observations: [
       "Contexto: Integração LanceDB↔SQLite sem bloquear Tokio",
       "Decisão: Daemon background com consistência eventual via MPSC channels",
       "Rationale: spawn_blocking viola Zero-Copy; daemon isola I/O do event loop",
       "Red Lines: PROIBIDO chamadas síncronas diretas entre LanceDB e SQLite no thread principal",
       "Data: 2026-07-28T00:00:00Z"
     ]
   }])
   ```
3. Cria relação: `mem_create_relations([{from: "ChyrosDaemon_ConsistencyPattern", to: "Tokio_EventLoop", relationType: "protects"}])`

**Exemplo 3 — Busca com Tentativa Dupla:**
1. `mem_search("UX Svelte 5 animações")` → 0 resultados.
2. Tentativa Dupla: `mem_search("svelte animation ambient status")` → encontra `"UX_AmbientStatus_Rule"`.
3. *"→ Memória resgatada via tentativa dupla. Regra de Ambient Status (50ms) encontrada."*