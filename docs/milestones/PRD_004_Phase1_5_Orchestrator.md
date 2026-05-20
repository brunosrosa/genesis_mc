# PRD 004 — Phase 1.5 Orchestrator & DB Persister

## 1. Objetivo Atômico

Implementar o `Phase1_5Orchestrator` em Rust. O orquestrador receberá um `repo_id`, buscará os blobs brutos no `soda_heuristic_vault.db` (tabela `artefatos_brutos`), acionará o `ParetoBanditRouter` (N3) para cada blob e delegará a destilação para o `LocalDistiller` (N4) ou `CloudCascade` (N6/N7) conforme a Zona (Green, Yellow, Red). Ao final, deve salvar as `_essences_` de forma imutável no banco de dados.

Escopo mecânico deste PRD:

- Cobrir exclusivamente o nó `N5 (Phase1_5Orchestrator)`.
- Receber um `repo_id` e uma conexão SQLite.
- Buscar blobs brutos na tabela `artefatos_brutos` um a um.
- Para cada blob: classificar via `FinOpsRouter`, rotear para `LocalDistiller` ou `CloudCascade`.
- Salvar essências resultantes em nova tabela `artefatos_destilados`.
- Manter a imutabilidade do dado bruto original.

## 2. Contrato de I/O (Entrada e Saída)

### Entrada

- `repo_id: String` — identificador do repositório (ex: `aaif-goose/goose`).
- `db_pool: &rusqlite::Connection` — pool de conexão SQLite.

### Saída

- `Result<(), OrchestratorError>` — sinaliza que todas as essências foram geradas e salvas com sucesso.

### Tipos de Erro

```rust
#[derive(Error, Debug, Clone)]
pub enum OrchestratorError {
    #[error("Repositorio invalido ou vazio: {0}")]
    InvalidRepoId(String),
    #[error("Falha ao buscar blobs no banco: {0}")]
    DatabaseReadError(String),
    #[error("Falha na destilacao: {0}")]
    DistillationError(String),
    #[error("Falha ao persistir essencia: {0}")]
    PersistError(String),
    #[error("Nenhum blob encontrado para o repo_id: {0}")]
    NoBlobsFound(String),
}
```

### Tabela Destino: `artefatos_destilados`

```sql
CREATE TABLE IF NOT EXISTS artefatos_destilados (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    repo_id     TEXT NOT NULL,
    essence_name TEXT NOT NULL,
    lens_target TEXT NOT NULL,
    payload_essence TEXT NOT NULL,
    token_count INTEGER NOT NULL,
    routing_zone TEXT NOT NULL,
    created_at  TEXT NOT NULL DEFAULT (datetime('now')),
    UNIQUE(repo_id, essence_name)
);
```

### Nomenclatura de Essências

| Blob Original | Essence Destinada |
|--------------|------------------|
| `blob_01_promessa_readme` | `_essence_01_promessa_readme` |
| `blob_02_dependency_manifest` | `_essence_02_dependency_manifest` |
| `blob_04_repo_outline` | `_essence_04_repo_outline` |
| `blob_05_architecture_map` | `_essence_05_architecture_map` |
| `blob_06_unsafe_hotspots` | `_essence_06_unsafe_hotspots` |
| `blob_07_ops_blueprint` | `_essence_07_ops_blueprint` |
| `blob_08_health_report` | `_essence_08_health_report` |
| `blob_09_community_meta` | `_essence_09_community_meta` |
| `blob_10_soda_canon_context` | `_essence_10_soda_canon_context` |
| `blob_11_ux_contracts` | `_essence_11_ux_contracts` |

Nota: `blob_03_test_intent` é legado e não entra no pipeline de destilação.

## 3. Lógica de Persistência (O Guardião da Imutabilidade)

### Princípio da Imutabilidade

O orquestrador opera sob o paradigma **FASTSWITCH + GUILHOTINA DE PROFUNDIDADE**:

1. **Leitura Sequencial**: Buscar blobs um a um no SQLite (não carregar todos na RAM).
2. **Processamento Sequencial**: Cada blob é processado e sua essência salva antes do próximo.
3. **Tabela Destino Separada**: A tabela `artefatos_brutos` NUNCA é modificada. Apenas `artefatos_destilados` recebe novas inserções.

### Fluxo de Dados

```
[repo_id]
    │
    ▼
┌─────────────────────────────────────────────┐
│ Phase1_5Orchestrator                        │
│                                              │
│  for each blob in artefatos_brutos:          │
│    │                                         │
│    ├──► FinOpsRouter.classify() ──► Zona     │
│    │                                         │
│    ├──► GREEN ──► PassThrough (sem distilação)│
│    │                                         │
│    ├──► YELLOW ──► LocalDistiller.distill() │
│    │                                         │
│    └──► RED ──► CloudCascade.cascade()      │
│    │                                         │
│    └──► Persistir em artefatos_destilados    │
└─────────────────────────────────────────────┘
    │
    ▼
[sucesso - todas essências salvas]
```

### FastSwitch Garantido

- Após cada blob: Drop explícito do `LocalDistiller` ou recursos da `CloudCascade`.
- Sem paralelismo de GPU (VRAM limpa entre blobs).
- Sem carregamento massivo de RAM (um blob por vez).

## 4. Proibições Tóxicas (Red Lines)

### PROIBIDO PARALELISMO CEGO NA GPU

A orquestração dos blobs de um repositório DEVE ser sequencial. O orquestrador não pode despachar dois blobs massivos simultaneamente para o `LocalDistiller`, pois isso violaria o teto de 6GB de VRAM e quebraria o paradigma `FastSwitch`.

```rust
// CORRETO: Processamento sequencial
for blob in blobs {
    let essence = process_blob(blob).await?;
    persist(essence).await?;
}

// INCORRETO: Paralelismo cego
let futures = blobs.map(|b| process_blob(b));
let results = futures::future::join_all(futures).await; // ← PROIBIDO
```

### PROIBIDO CARREGAR TODOS OS BLOBS NA RAM

A leitura do SQLite deve ocorrer blob a blob. É proibido carregar os 11 blobs gigantes de um repositório na memória ao mesmo tempo (Risco de OOM).

```rust
// CORRETO: Um blob por vez
let blobs = fetch_blob_one_by_one(repo_id, conn)?;

// INCORRETO: Carregar todos de uma vez
let blobs = conn.query_many_blobs(repo_id)?; // ← PROIBIDO (OOM risk)
```

## 5. Definition of Done (DoD) & TDD

### Teste 1: Nomenclatura de Essências Respeitada

- Mock do banco SQLite com blobs simulados.
- Processar blobs e verificar que a nomenclatura é convertida corretamente:
  - `blob_08_health_report` → `_essence_08_health_report`
  - `blob_10_soda_canon_context` → `_essence_10_soda_canon_context`

### Teste 2: Processamento Sequencial (Sem Paralelismo)

- Criar 3 blobs simulados com tokens > 16k (YELLOW).
- Verificar que `LocalDistiller` é chamado 3 vezes sequencialmente, nunca simultaneamente.
- Provar que não há race condition na VRAM.

### Teste 3: Imutabilidade da Tabela Bruta

- Após execução do orchestrator, verificar que `artefatos_brutos` permanece inalterado.
- Confirmar que apenas `artefatos_destilados` recebeu inserts.

### Teste 4: Zone Routing Correto

- Blob de 10k tokens → GREEN → PassThrough
- Blob de 30k tokens → YELLOW → LocalDistiller
- Blob de 70k tokens → RED → CloudCascade

### Teste 5: Erro Terminal

- Se um blob falhar na destilação, o processo deve retornar `OrchestratorError`.
- Não deve deixar blobs processados parcialmente sem erro.

### Critérios de Aceitação

- Módulo passa em `cargo clippy -- -D warnings`.
- Nenhum `unwrap()` ou `expect()` em código de produção (apenas testes).
- Prova de processamento sequencial via mock counters.
- Tabela `artefatos_brutos` intocada após execução.

### Dependências de Mock

- `mockall` para mockar `FinOpsRouter`, `LocalDistiller`, `CloudCascade`.
- `rusqlite` com transação simulada para testes.
