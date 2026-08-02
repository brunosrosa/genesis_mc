# PRD-031 & PRD-032: BLOCO I - CORE COGNITIVO (SOULS_GRAPH & SOULS_THINKING)
**ID de Rastreabilidade:** PRD_031_032_V4_CORE_COGNITIVE  
**Version:** 1.0  
**Status:** PROPOSED (Pronto para Injeção via CSDD/TDD)  
**Epic:** Cognição / Memória  
**Target:** `src-tauri/src/cognition/`  

---

## 🏛️ 1. PROPÓSITO DO BLOCO
O Bloco I existe para erradicar definitivamente duas patologias graves encontradas em subagentes agênticos de IA locais [103]:
1. **Amnésia Epistêmica Temporal (Context Rot):** Subagentes perdem rastro de descobertas e decisões tomadas em turnos passados, gerando repetição inútil de buscas e redigitação manual de código [103, 369].
2. **Paralisia de Análise (Overthinking):** Modelos compactos locais de linguagem tentam cuspir a resposta inteira em um único passo estocástico de decodificação, falhando na manutenção de restrições rígidas sintáticas e lógicas [193, 1011].

Unificamos a persistência relacional local baseada no `memory-mcp-rs` (`souls_graph`) [86] com a máquina socrática de raciocínio sequencial do `ultrafast-mcp-sequential-thinking` (`souls_thinking`) [87] dentro do core bare-metal de Rust do SODA, operando 100% offline, air-gapped e com latências sub-milissegundo [301, 356].

---

## ⚙️ 2. PRD-031: `souls_graph` (A Memória Relacional no SQLite)

### 2.1. O Esquema Físico do Banco de Dados (SQLite WAL)
Para evitar o fetiche por bancos vetoriais inflados que demandariam alocação severa de RAM/VRAM na máquina host [279, 362], a memória relacional e semântica de entidades será persistida inteiramente como tabelas dentro do banco **`souls_state.db`** (gerenciado de forma assíncrona por canais MPSC no Tokio) [357, 365]:

1. **`entities`**:
   * `name` TEXT PRIMARY KEY (Identificador exclusivo da entidade, ex: "file_locker.rs" ou "ADR-027").
   * `type` TEXT NOT NULL (Enum de classificação, ex: "File", "ADR", "Module", "Heuristic").

2. **`relations`**:
   * `from_entity` TEXT NOT NULL,
   * `to_entity` TEXT NOT NULL,
   * `relation_type` TEXT NOT NULL,
   * PRIMARY KEY (`from_entity`, `to_entity`, `relation_type`),
   * FOREIGN KEY (`from_entity`) REFERENCES `entities` (`name`) ON DELETE CASCADE,
   * FOREIGN KEY (`to_entity`) REFERENCES `entities` (`name`) ON DELETE CASCADE.

3. **`observations`**:
   * `id` INTEGER PRIMARY KEY AUTOINCREMENT,
   * `entity_name` TEXT NOT NULL,
   * `content` TEXT NOT NULL,
   * `created_at` INTEGER NOT NULL (Timestamp UNIX Epoch em milissegundos via CPU nativa, sem chrono) [428].
   * FOREIGN KEY (`entity_name`) REFERENCES `entities` (`name`) ON DELETE CASCADE.

4. **`entities_fts` (Virtual Table FTS5)**:
   * Tabela virtual SQLite FTS5 espelhando o conteúdo de `observations` e `entities` para busca textual por prefixos e similaridade léxica instantânea em tempo constante $\mathcal{O}(\log N)$ [124].

### 2.2. A Lei de Ferro do SQLite no SODA
* **Regra de Concorrência Segura:** A conexão de gravação de escrita física SQLite é sequencializada compulsoriamente através do `StateDbWorker` (canal `tokio::sync::mpsc` com buffer limitado em 100 unidades para evitar OOM) [357, 358].
* **Prevenção de Bloqueios:** Ativar sistematicamente o modo **WAL (Write-Ahead Logging)** e injetar `PRAGMA busy_timeout = 5000;` no bootstrap de cada nova conexão do `rusqlite` [358, 365].
* **Garantia de Integridade:** Toda inicialização de pool de banco de dados SODA deve executar explicitamente `PRAGMA foreign_keys = ON;`, blindando as exclusões em cascata contra nós órfãos [707].

---

## 🧠 3. PRD-032: `souls_thinking` (O Scratchpad Socrático)

### 3.1. A Máquina de Estados de Pensamento Sequencial
Toda inferência complexa ou mutação de arquivo despachada para os motores locais (como o `LlamaVanguardEngine` ou `LlamaCppEngine`) é interceptada e forçada a utilizar o scratchpad socrático antes de emitir a escrita no disco [1011, 1029]. A estrutura do pensamento obedece à especificação do `ThoughtData` [87]:

```rust
pub struct ThoughtData {
    pub thought_number: i32,
    pub is_revision: bool,
    pub revises_thought: Option<i32>,
    pub branch_from_thought: Option<i32>,
    pub branch_id: Option<String>,
    pub next_thought_needed: bool,
    pub thought_content: String,
}
```

As transições de estados cognitivos são controladas localmente em RAM via `HashMap<BranchId, Vec<ThoughtId>>` alocada temporariamente no contexto do laço da tarefa, limpando-se o heap imediatamente no teardown do subagente [1012, 1043].

### 3.2. As Válvulas de Escape e Limites (Anti-Overthinking)
* **Disjuntor de Iterações (Hard Limit):** Fica decretado que nenhum subagente local pode ultrapassar o limite físico de **5 pensamentos sequenciais** por ciclo [711]. Se o modelo entrar em loop socrático redundante, o SODA aborta a tarefa de inferência, executa o *Fail-Closed* e notifica o Arquiteto Humano [1188, 1196]. 
* **Teto Elástico HITL:** Apenas sob aprovação explícita em tempo real do Arquiteto (Human-in-the-Loop), o limite pode ser flexibilizado para o teto absoluto de **7 pensamentos** [711].

---

## 🧪 4. SUÍTE DE TESTES EXIGIDA (TDD)
O desenvolvedor na IDE (Trae) deve implementar e vergar a suíte de testes unitários na pasta de testes do core, garantindo que passem com Exit Code 0 sob as flags de compilação da v4:

1. **`test_graph_cascade_delete`**: Provar que, ao remover uma entidade da tabela `entities`, todas as suas relações correspondentes em `relations` e observações em `observations` sofrem expurgo físico imediato do disco por ação do banco de dados (WAL), com `foreign_keys` ativo.
2. **`test_thinking_disjuntor_loop`**: Instanciar uma máquina de estados de `souls_thinking` e simular a injeção contínua de pensamentos. Provar que, ao tentar registrar o 6º pensamento (violando o limite de 5), o módulo de controle em Rust retorna compulsoriamente um erro do tipo `CognitiveError::OverthinkingThresholdBreached` e aborta a execução do Tokio.
3. **`test_fts5_observational_grounding`**: Inserir observações técnicas no banco de dados e provar que a consulta de FTS5 localiza a "agulha no palheiro" das entidades em menos de 1 milissegundo de CPU.

---

## 🚦 DEFINITION OF DONE (DoD) MATEMÁTICO
Para esta entrega ser considerada homologada pela "Alfândega" do SODA [253, 513]:
* [ ] Nenhuma dependência externa à árvore estrita de crates do SODA V4 (como sincronizadores HTTP do `memory-mcp`) foi introduzida no `Cargo.toml`.
* [ ] O código do `file_locker.rs` e do `state.db` opera com zero warnings no clippy e 100% de sucesso no `cargo test`.
* [ ] O modo WAL e `foreign_keys = ON` estão comprovados em tempo de execução nos testes de integração.
* [ ] A documentação do `SODA_CURRENT_STATE.md` foi atualizada com o registro do Marco 3.5 ativo.
