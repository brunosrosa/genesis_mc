# PRD 006 — Phase 2 Cognitive Swarm

## 1. Objetivo Atômico

Implementar o orquestrador do **Enxame Cognitivo** em Rust sob o paradigma
**Map-Reduce Socrático**. O módulo lerá os pacotes A, B e C do banco de dados,
despachará **3 requisições HTTP simultâneas** para os LLMs premium
(`Claude Opus 4.7`, `DeepSeek V4 Pro` e `GLM-5.1` ou `Qwen 3.6+`) e salvará os
três debates na tabela obrigatória `debates_enxame`.

Escopo mecânico deste PRD:

- Cobrir exclusivamente os nós `N1` a `N5` mapeados em `design_phase2.md`.
- Receber um `repo_id` e uma conexão SQLite.
- Buscar a `struct Phase2Payloads` gerada na Fase 1.5.
- Despachar as lentes `N2`, `N3` e `N4` em paralelo, sem bloqueio sequencial.
- Persistir os três mini-JSONs de debate em `debates_enxame`.
- Retornar `Result<(), Phase2Error>` quando os 3 debates forem salvos com sucesso.

## 2. Contrato de I/O e Composição

### Entrada

- `repo_id: String` — identificador do repositório (ex: `aaif-goose/goose`).
- `db_pool: &rusqlite::Connection` — conexão SQLite.
- O sistema buscará a `struct Phase2Payloads` gerada na Fase 1.5 para compor o
  contexto das Lentes.

### Saída

- `Result<(), Phase2Error>` — sucesso significa que os 3 debates foram salvos na
  tabela `debates_enxame`.

### Struct de Entrada Esperada

```rust
#[derive(Debug, Clone)]
pub struct Phase2Payloads {
    pub package_a: String,
    pub package_b: String,
    pub package_c: String,
}
```

### Tipos de Erro

```rust
#[derive(Error, Debug, Clone)]
pub enum Phase2Error {
    #[error("Repositorio invalido: {0}")]
    InvalidRepoId(String),
    #[error("Falha ao buscar payloads da Fase 1.5: {0}")]
    PayloadFetchError(String),
    #[error("Pacote ausente ou vazio: {0}")]
    EmptyPackage(String),
    #[error("Falha na lente {lens}: {message}")]
    LensExecutionError { lens: String, message: String },
    #[error("Falha ao persistir debates: {0}")]
    PersistError(String),
    #[error("Repositorio marcado como erro na Fase 2: {0}")]
    Phase2Aborted(String),
}
```

### Tabela Alvo Obrigatória: `debates_enxame`

```sql
CREATE TABLE IF NOT EXISTS debates_enxame (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    repo_id TEXT NOT NULL,
    lens_a_json TEXT NOT NULL,
    lens_b_json TEXT NOT NULL,
    lens_c_json TEXT NOT NULL,
    phase_status TEXT NOT NULL,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    UNIQUE(repo_id)
);
```

### Contrato de Composição dos Nós

| Nó | Responsabilidade | Entrada | Saída |
|---|---|---|---|
| `N1 (SwarmDispatcher)` | Buscar `Phase2Payloads` e preparar o fan-out | `repo_id`, SQLite | `package_a`, `package_b`, `package_c` |
| `N2 (LensA_ProductUX)` | Enviar o Pacote A para `Claude Opus 4.7` | `package_a` | Mini-JSON A |
| `N3 (LensB_Architecture)` | Enviar o Pacote B para `DeepSeek V4 Pro` | `package_b` | Mini-JSON B |
| `N4 (LensC_Operations)` | Enviar o Pacote C para `GLM-5.1` ou `Qwen 3.6+` | `package_c` | Mini-JSON C |
| `N5 (DebatePersister)` | Persistir os 3 mini-JSONs de forma atômica | `repo_id`, A, B, C | `Result<(), Phase2Error>` |

### Contrato do Payload de Saída das Lentes

Cada Lente deve responder em JSON estruturado, compacto e estável:

```json
{
  "lens_id": "LensA_ProductUX",
  "repo_id": "string",
  "bullets": [
    "3 a 5 bullets curtos e factuais"
  ],
  "risk_level": "low|medium|high",
  "recommendation": "keep|refine|reject"
}
```

Regras obrigatórias do contrato:

- Entre 3 e 5 bullets.
- Limite alvo de aproximadamente 250 tokens por Lente.
- Prosa livre fora do JSON é proibida.
- O payload deve ser persistível diretamente em `TEXT` no SQLite.

## 3. Proibições Tóxicas (Red Lines)

### PROIBIDO CÓDIGO SÍNCRONO

A execução das 3 Lentes **DEVE** ocorrer em paralelo. Qualquer fluxo que faça
`await` sequencial de N2, depois N3, depois N4, é falha crítica de arquitetura.
O programador deve usar `tokio::join!` ou `JoinSet`.

```rust
// CORRETO: paralelismo obrigatório
let (lens_a, lens_b, lens_c) = tokio::join!(
    run_lens_a(package_a),
    run_lens_b(package_b),
    run_lens_c(package_c),
);

// INCORRETO: bloqueio sequencial
let lens_a = run_lens_a(package_a).await?;
let lens_b = run_lens_b(package_b).await?;
let lens_c = run_lens_c(package_c).await?; // <- PROIBIDO
```

### PROIBIDO COMPARTILHAMENTO DE CONTEXTO

As Lentes `N2`, `N3` e `N4` são autônomas. É proibido passar a resposta da
Lente A para a Lente B, ou usar a saída de qualquer Lente como prompt de outra.

```rust
// CORRETO: isolamento absoluto
run_lens_a(package_a_only).await?;
run_lens_b(package_b_only).await?;
run_lens_c(package_c_only).await?;

// INCORRETO: contaminação cruzada
let a = run_lens_a(package_a_only).await?;
let b = run_lens_b(format!("{package_b_only}\n{a}")).await?; // <- PROIBIDO
```

### MECÂNICA FAIL-FAST OBRIGATÓRIA

A chamada HTTP das APIs premium deve ter no máximo **2 retries**. Se a 3a
tentativa falhar, o Nó morre, a persistência atômica do SQLite falha e o
repositório é marcado com `ERRO_FASE_2`.

```rust
// REGRA: tentativa inicial + 2 retries = teto absoluto
for attempt in 1..=3 {
    match execute_http_call().await {
        Ok(response) => return Ok(response),
        Err(err) if attempt < 3 => continue,
        Err(err) => return Err(Phase2Error::LensExecutionError {
            lens: "LensA_ProductUX".into(),
            message: err.to_string(),
        }),
    }
}
```

### PROIBIDO PERSISTÊNCIA PARCIAL

Os três debates devem ser salvos juntos. Se uma Lente falhar, `debates_enxame`
não pode receber escrita parcial para aquele `repo_id`.

## 4. Fluxo de Execução e Persistência

```text
[repo_id]
   |
   v
N1: SwarmDispatcher
   |-- busca Phase2Payloads no SQLite
   |
   +--> N2: LensA_ProductUX ------+
   +--> N3: LensB_Architecture ---+--> N5: DebatePersister --> debates_enxame
   +--> N4: LensC_Operations -----+
```

### Sequência Mecânica

1. `N1` valida `repo_id` e busca `Phase2Payloads` no banco.
2. `N1` verifica que `package_a`, `package_b` e `package_c` não estão vazios.
3. `N1` dispara `N2`, `N3` e `N4` em paralelo.
4. Cada Lente executa no máximo 3 tentativas HTTP.
5. Se as três Lentes retornarem sucesso, `N5` abre a persistência atômica.
6. `N5` grava os três mini-JSONs em `debates_enxame` e marca `FASE_2_OK`.
7. Se qualquer Lente falhar terminalmente, a transação falha e o repositório é
   marcado com `ERRO_FASE_2`.

### Regras de Persistência

- Tabela obrigatória: `debates_enxame`.
- Sucesso: gravar `lens_a_json`, `lens_b_json`, `lens_c_json` e `phase_status = 'FASE_2_OK'`.
- Falha terminal: não gravar debates parciais; marcar o repositório com `ERRO_FASE_2`.
- A Fase 2 não escreve em Google Sheets.

## 5. Definition of Done (DoD) & TDD

O programador deverá criar testes unitários com **mocks de API** provando o
comportamento do enxame antes da lógica final de produção.

### Teste 1: Paralelismo funcional

- Criar 3 mocks de API com atraso controlado.
- Disparar as três Lentes no orchestrator.
- Provar que as 3 requisições ocorrem sem bloqueio mútuo.
- Evidência esperada: o tempo total do teste é compatível com execução paralela,
  e não com soma sequencial das latências.

### Teste 2: Isolamento de pacotes

- Mockar `Phase2Payloads` com conteúdos rastreáveis.
- Verificar que `LensA_ProductUX` recebe **estritamente** `package_a` com seu
  `blob_10` embutido, sem conteúdo de `package_b` ou `package_c`.
- Repetir a prova para `LensB_Architecture` e `LensC_Operations`.

### Teste 3: Fail-Fast

- Mockar uma Lente retornando `HTTP 429` infinitamente.
- Confirmar que o sistema tenta no máximo 3 vezes.
- Confirmar que o orchestrator aborta e retorna `Phase2Error`.
- Confirmar que `debates_enxame` não recebe persistência parcial.

### Critérios de Aceitação

- O módulo retorna `Result<(), Phase2Error>`.
- As três Lentes são executadas em paralelo via runtime assíncrono do Tokio.
- Nenhuma Lente recebe contexto de outra.
- O teto de retry é respeitado sem loops infinitos.
- `debates_enxame` recebe escrita apenas em caso de sucesso total.
- O repositório é marcado com `ERRO_FASE_2` quando houver falha terminal.
- Nenhum `unwrap()` ou `expect()` em código de produção.
- O módulo deve passar em `cargo clippy -- -D warnings`.

### Dependências de Mock Sugeridas

- `mockall` para mockar clientes HTTP e persistência.
- `tokio::time` para controlar latência artificial nos testes assíncronos.
- SQLite em memória para provar atomicidade e ausência de persistência parcial.
