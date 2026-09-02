# Especificação Técnica e Caderno de TDD: MARCO 5.16.0 — O Orquestrador de Cascata Documental SDD (sdd.rs)

## 🏛️ 1. O Racional de Design (A Cura da Desconexão de Especificação)

No ecossistema **SODA / Souls MC**, a conformidade entre a especificação declarativa de alto nível (foco humano) e a implementação de baixo nível (execução em silício) é uma barreira de proteção absoluta contra a entropia e o código inflado (*code slop*).

Atualmente, o ciclo de vida do **Spec-Driven Development (SDD)**:
$$\text{REQUIREMENTS.md} \rightarrow \text{DESIGN.md} \rightarrow \text{TASKS.md} \rightarrow \text{TEST\_SPECS.md}$$
opera de forma estocástica e semi-manual, confiando na disciplina subjetiva do agente de IA na IDE. Se o operador humano alterar os requisitos de escopo no primeiro documento, a IA pode ignorar a mudança e continuar escrevendo códigos de produção com base no design defasado, gerando desperdício financeiro de tokens e desvio de metas (*goal drift*).

O **Marco 5.16.0** introduz o motor determinístico **`sdd.rs`** no núcleo em Rust da biblioteca estável `souls_mc_lib`. Ele gerencia, valida e impõe a integridade física do ciclo em tempo real através de travas lógicas e criptográficas de hashes SHA-256 no banco de dados **souls_state.db** (elevado de forma idempotente para a versão **V6**).

Se um hash de especificação superior for alterado à revelia do protocolo, as fases subsequentes são invalidadas recursivamente, bloqueando qualquer injeção física de código de produção até que ocorra o Code Review e a aprovação humana explicita (**Human-in-the-Loop - HITL**).

---

## 💾 2. Modelagem Relacional (SOULS State v6 DDL)

Promovemos o `PRAGMA user_version` para `6` por meio de uma migração idempotente. Criamos a tabela de rastreabilidade de integridade documental do SDD:

```sql
-- Tabela de Estado e Integridade de Documentos SDD (v6)
CREATE TABLE IF NOT EXISTS sdd_document_states (
    document_path TEXT PRIMARY KEY STRICT, -- Path canônico normalizado do arquivo MD
    sha256_hash TEXT NOT NULL,             -- Hash SHA-256 do conteúdo do arquivo
    last_validated_at INTEGER NOT NULL,    -- UNIX epoch timestamp em segundos
    is_approved INTEGER NOT NULL DEFAULT 0 -- Boolean (0 = Bloqueado/Pendente, 1 = Aprovado HITL)
);
```

---

## ⚙️ 3. As Três Leis da Cascata de Invalidação

O módulo `src-tauri/src/core/sdd.rs` implementará as seguintes regras de estado:

### Lei I: A Assinatura de Requisitos (HITL Signature Verification)
O validador abre o arquivo **`REQUIREMENTS.md`**. O documento só é considerado elegível para aprovação se contiver a assinatura textual explícita do operador humano no formato:
`[APPROVED_BY_HUMAN: YYYY-MM-DD]`
Se a tag estiver ausente ou malformada, o status cai para `is_approved = 0` no banco de dados, interrompendo imediatamente o fluxo de compilação da funcionalidade.

### Lei II: A Invalidação em Cascata (SHA-256 Cascade)
Sempre que o orquestrador realiza a varredura das especificações, ele recalcula os hashes de cada um dos quatro arquivos no disco.
Se o hash armazenado de **`REQUIREMENTS.md`** divergir do hash atual (indicando que o escopo mudou):
1. O status de **`REQUIREMENTS.md`** é rebaixado para `is_approved = 0` no SQLite.
2. O sistema executa a **invalidação em cascata recursiva**, resetando o status de **`DESIGN.md`**, **`TASKS.md`** e **`TEST_SPECS.md`** para `is_approved = 0` de forma atômica no banco de dados.
3. O orquestrador aborta a transação e emite o erro `CognitiveError::SddCascadeViolation` na telemetria de erro.

### Lei III: O Teste de Cobertura TDD Estrito (Cross-Match Verification)
Antes de autorizar a transição para a Fase C (escrita de código de produção em Rust), o parser do `sdd.rs` executa uma análise estática semântica simples (Zero-Allocation) sobre os arquivos markdown:
1. Varre **`TASKS.md`** extraindo os IDs de tarefas pendentes ou ativas (ex: `Task 140`).
2. Varre **`TEST_SPECS.md`** buscando as assinaturas e decorações de testes associadas a esses IDs (ex: `test_sdd_cascade_invalidation`).
3. Se houver qualquer tarefa ativa declarada no plano sem a respectiva correspondência de especificação de teste unitário redigida, o portão de segurança trava a esteira com erro `CognitiveError::UntrustedExecutionBlocked`.

---

## 🔬 4. Suíte de Testes TDD (Fast Pass < 0.2s)

Os 3 contratos de testes de estresse serão integrados diretamente no arquivo modular de testes `src/bin/souls_mcp_server/tests.rs`:

1. **`test_sdd_requirements_approved_gate`**:
   - Cria um arquivo `REQUIREMENTS.md` fictício sem a assinatura `APPROVED_BY_HUMAN`.
   - Assevera que o motor de validação recusa a homologação, retornando status `is_approved = 0`.
   - Insere a assinatura válida e assevera a transição bem-sucedida para `is_approved = 1`.

2. **`test_sdd_cascade_hash_invalidation`**:
   - Popula estados aprovados (`is_approved = 1`) para os 4 arquivos na tabela `sdd_document_states`.
   - Altera sinteticamente o conteúdo de `REQUIREMENTS.md` (provocando desvio de SHA-256).
   - Executa a rotina do orquestrador e assevera via queries SQL que as aprovações de `DESIGN.md`, `TASKS.md` e `TEST_SPECS.md` decaíram atomicamente para `0`.

3. **`test_sdd_tdd_coverage_check`**:
   - Fornece um arquivo `TASKS.md` contendo a tarefa `Task 999: Teste` e um `TEST_SPECS.md` sem a assinatura correspondente.
   - Prova por meio de asserção que o validador lança o erro de quebra de cobertura estrutural.
   - Corrige adicionando a assinatura de teste no markdown e assevera a liberação do sinal verde.
