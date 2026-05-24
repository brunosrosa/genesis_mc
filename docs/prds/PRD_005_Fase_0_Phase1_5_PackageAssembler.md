# PRD 005 — Phase 1.5 Package Assembler

## 1. Objetivo Atômico

Implementar o `PackageAssembler` em Rust. Ele lerá a tabela `artefatos_destilados` e a tabela `artefatos_brutos` para um dado `repo_id`, agrupando o texto em três Dossiês (Pacote A, B e C), retornando um struct `Phase2Payloads`.

Escopo mecânico deste PRD:

- Cobrir exclusivamente o nó `N9 (PackageAssembler)`.
- Receber um `repo_id` e uma conexão SQLite.
- Buscar essências na tabela `artefatos_destilados`.
- Buscar `blob_10_soda_canon_context` na tabela `artefatos_brutos` (NÃO na destilada).
- Montar 3 pacotes separados com composição fixa.
- Retornar `Phase2Payloads` com os 3 textos.

## 2. Contrato de I/O e Composição Inegociável

### Entrada

- `repo_id: String` — identificador do repositório (ex: `aaif-goose/goose`).
- `db_pool: &rusqlite::Connection` — pool de conexão SQLite.

### Saída

- `Result<Phase2Payloads, AssemblerError>` — struct contendo os 3 pacotes.

### Tipos de Erro

```rust
#[derive(Error, Debug, Clone)]
pub enum AssemblerError {
    #[error("Repositorio invalido: {0}")]
    InvalidRepoId(String),
    #[error("Falha ao buscar essencias: {0}")]
    DatabaseReadError(String),
    #[error("Essencia ausente no banco: {0}")]
    EssenceNotFound(String),
    #[error("Canon context ausente (blob_10): {0}")]
    CanonContextNotFound(String),
}
```

### Struct de Output

```rust
#[derive(Debug, Clone)]
pub struct Phase2Payloads {
    pub package_a: String,
    pub package_b: String,
    pub package_c: String,
}
```

### Composição Exata dos Pacotes

| Pacote | Lente | Essências Incluídas | Blob 10 Anexado |
|--------|-------|---------------------|-----------------|
| **Pacote A** | Produto/UX | `_essence_01` + `_essence_03` + `_essence_11` | ✅ SIM |
| **Pacote B** | Arquiteto | `_essence_04` + `_essence_05` | ✅ SIM |
| **Pacote C** | Ops/Auditor | `_essence_02` + `_essence_06` + `_essence_07` + `_essence_08` + `_essence_09` | ✅ SIM |

### Mapeamento de Artefatos para Pacotes

```
artefatos_destilados (busca por essence_name):
  _essence_01_promessa_readme      → Pacote A
  _essence_02_dependency_manifest  → Pacote C
  _essence_03_test_intent         → NÃO DESTILADO (legacy)
  _essence_04_repo_outline         → Pacote B
  _essence_05_architecture_map     → Pacote B
  _essence_06_unsafe_hotspots      → Pacote C
  _essence_07_ops_blueprint       → Pacote C
  _essence_08_health_report        → Pacote C
  _essence_09_community_meta       → Pacote C
  _essence_10_soda_canon_context   → NÃO EXISTE NA DESTILADA
  _essence_11_ux_contracts        → Pacote A

artefatos_brutos (busca por artifact_type):
  blob_10_soda_canon_context      → ANEXADO A TODOS OS 3 PACOTES
```

### Estrutura do Pacote Gerado

Cada pacote segue o formato:

```
=== PACOTE A (PRODUTO/UX) ===
[...] essência 01 [...]
[...] essência 03 [...]
[...] essência 11 [...]
=== BLOB_10_CANON_CONTEXT ===
[...] blob_10_soda_canon_context original [...]
=== FIM PACOTE A ===
```

## 3. Fluxo de Dados

```
[repo_id]
    │
    ▼
┌─────────────────────────────────────────────┐
│ PackageAssembler                            │
│                                              │
│  1. Buscar _essence_01, _essence_03,        │
│     _essence_11 em artefatos_destilados      │
│     → Montar package_a                       │
│                                              │
│  2. Buscar _essence_04, _essence_05         │
│     em artefatos_destilados                 │
│     → Montar package_b                       │
│                                              │
│  3. Buscar _essence_02, _essence_06,        │
│     _essence_07, _essence_08, _essence_09   │
│     em artefatos_destilados                 │
│     → Montar package_c                       │
│                                              │
│  4. Buscar blob_10_soda_canon_context       │
│     em artefatos_brutos (BRUTO, não destilado)│
│     → Anexar ao final de cada pacote         │
└─────────────────────────────────────────────┘
    │
    ▼
[Phase2Payloads { package_a, package_b, package_c }]
```

## 4. Proibições Tóxicas (Red Lines)

### PROIBIDO DESTILAR O CANON (BLOB 10)

O `blob_10_soda_canon_context` NÃO existe na tabela `artefatos_destilados`. Ele deve ser buscado ESTRITAMENTE em sua forma original e inalterada na tabela `artefatos_brutos` e anexado ao final de cada pacote.

```rust
// CORRETO: Buscar blob_10 na tabela bruta
let canon = conn.query_blob_raw("artefatos_brutos", "blob_10_soda_canon_context")?;

// INCORRETO: Buscar na destilada
let canon = conn.query_essence("_essence_10")?; // ← NÃO EXISTE
```

### PROIBIDO VAZAMENTO DE CONTEXTO

Um pacote não pode conter essências de outro. A Lente B (Arquiteto) não pode receber dados de UX (Pacote A), para evitar "Lost in the Middle".

```rust
// CORRETO: Cada pacote contém apenas suas essências designadas
package_a = _essence_01 + _essence_03 + _essence_11;
package_b = _essence_04 + _essence_05;
package_c = _essence_02 + _essence_06 + _essence_07 + _essence_08 + _essence_09;

// INCORRETO: Vazamento de contexto
package_b = _essence_01 + _essence_04 + _essence_05; // ← _essence_01 é UX (Pacote A)
```

## 5. Definition of Done (DoD) & TDD

### Teste 1: Pacote A Contém Essências Corretas

- Mock do banco com essências simuladas.
- Verificar que `package_a` contém substrings de `_essence_01`, `_essence_03`, `_essence_11`.
- Verificar que NÃO contém `_essence_04` ou `_essence_05` (vazamento).

### Teste 2: Pacote B Contém Essências Corretas

- Verificar que `package_b` contém substrings de `_essence_04`, `_essence_05`.
- Verificar que NÃO contém `_essence_01` ou `_essence_11`.

### Teste 3: Pacote C Contém Essências Corretas

- Verificar que `package_c` contém substrings de `_essence_02`, `_essence_06`, `_essence_07`, `_essence_08`, `_essence_09`.
- Verificar que NÃO contém `_essence_01`, `_essence_03`, `_essence_11`.

### Teste 4: BLOB 10 Anexado aos 3 Pacotes

- Mock do banco com `blob_10_soda_canon_context` na tabela bruta.
- Verificar que `package_a` contém `blob_10_soda_canon_context`.
- Verificar que `package_b` contém `blob_10_soda_canon_context`.
- Verificar que `package_c` contém `blob_10_soda_canon_context`.

### Teste 5: Blob 10 NÃO Está na Tabela Destilada

- Tentar buscar `_essence_10` na tabela destilada.
- Confirmar que retorna erro `EssenceNotFound`.

### Critérios de Aceitação

- Módulo passa em `cargo clippy -- -D warnings`.
- Nenhum `unwrap()` ou `expect()` em código de produção.
- Prova de isolamento de pacotes via assertions negativas.
- BLOB 10证实来源为`artefatos_brutos`，不是`artefatos_destilados`.
