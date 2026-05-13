# PRD-006: JCodemunchSidecar

> **Nó DAG:** N6 (Depende de N3 — SandboxOrchestrator, N5 — ExtractionRouter)
> **Módulo Rust:** `sidecar::JCodemunchSidecar`
> **Território:** PRODUTO (Daemon SODA — Rust/Tokio)
> **Classificação:** EXTRAÇÃO — Primeiro Sidecar Efêmero da Camada 2
> **Target FinOps:** `local_slm` (custo ZERO)

---

## 1. Objetivo

Executar o binário `jcodemunch` dentro do `SandboxHandle` para extrair a
**Árvore de Sintaxe Abstrata (AST)** e o **Grafo de Dependências Topológico**
de um repositório clonado, devolvendo os resultados como uma struct Rust
tipada (`AstPayload`) sem tocar o disco do host e sem bloquear o Event Loop.

O `JCodemunchSidecar` é o **Bisturi de AST** do pipeline. Ele não interpreta
código; ele o **disseca mecanicamente**, extraindo o esqueleto estrutural
(funções, classes, imports, exports) que será consumido pelo `BlobNormalizer`
(N12) para persistência no `soda_heuristic_vault.db`.

### 1.1. Princípio do Sidecar Efêmero

O `jcodemunch` é tratado como um **processo descartável** que:

1. Nasce dentro do sandbox (isolamento LPAC/Landlock).
2. Lê os arquivos-fonte diretamente no Ramdisk (dentro do sandbox).
3. Despeja seu output em `stdout` como JSON estruturado.
4. Morre atomicamente após a execução (SIGKILL via `SandboxHandle::Drop`).

O SODA **nunca** lê o código-fonte diretamente. Toda a leitura pesada de
arquivos é delegada ao `jcodemunch` operando isolado dentro da gaiola.

---

## 2. Contrato I/O (Régua Atômica)

### 2.1. Entrada

```rust
pub struct JCodemunchInput<'a> {
    pub sandbox: &'a SandboxHandle,
    pub repo_path: &'a RepoPath,
    pub timeout_secs: u64,
}
```

| Campo | Tipo | Origem | Semântica |
|---|---|---|---|
| `sandbox` | `&SandboxHandle` | N3 (`SandboxOrchestrator::create`) | Gaiola de segurança com policy `ReadOnly` aplicada |
| `repo_path` | `&RepoPath` | N2 (`BloblessCloner::clone`) | Referência imutável ao repositório no Ramdisk |
| `timeout_secs` | `u64` | Configuração do pipeline (padrão: `120`) | Limite de execução do processo — SIGKILL após estourar |

### 2.2. Saída

```rust
pub async fn extract(input: JCodemunchInput<'_>) -> Result<AstPayload, SidecarError>
```

### 2.3. AstPayload (Struct de Saída Tipada)

```rust
#[derive(Debug, Clone, serde::Deserialize)]
pub struct AstPayload {
    /// Lista de símbolos extraídos (funções, classes, métodos, constantes)
    pub symbols: Vec<SymbolOutline>,

    /// Grafo de dependências (imports/exports/use) como lista de arestas
    pub dependency_edges: Vec<DependencyEdge>,

    /// Contagem total de arquivos processados pelo jcodemunch
    pub files_processed: u32,
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct SymbolOutline {
    /// Nome qualificado do símbolo (ex: "crate::harvester::git::BloblessCloner::clone")
    pub name: String,

    /// Tipo do símbolo: "function", "class", "method", "constant", "type", "import"
    pub kind: String,

    /// Arquivo de origem relativo ao repo_path
    pub file_path: String,

    /// Linha de início no arquivo
    pub start_line: u32,

    /// Linha de fim no arquivo
    pub end_line: u32,

    /// Assinatura completa (ex: "pub async fn clone(repo_url: &Url, ramdisk: &RamdiskHandle) -> Result<RepoPath, CloneError>")
    pub signature: Option<String>,
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct DependencyEdge {
    /// Arquivo que importa
    pub source_file: String,

    /// Módulo/símbolo importado
    pub target: String,

    /// Tipo de dependência: "use", "import", "require", "include"
    pub edge_type: String,
}
```

### 2.4. SidecarError (Enum de Falhas)

```rust
#[derive(Error, Debug, Clone, PartialEq, Eq)]
pub enum SidecarError {
    /// O processo jcodemunch não foi encontrado no PATH do sandbox
    #[error("Sidecar binary not found: {binary}")]
    BinaryNotFound { binary: String },

    /// A execução falhou (exit code != 0, OOM kill, crash)
    #[error("Execution failed: {reason}")]
    ExecutionFailed { reason: String },

    /// O processo excedeu o timeout e foi guilhotinado
    #[error("Execution timed out after {timeout_secs}s")]
    Timeout { timeout_secs: u64 },

    /// O stdout do processo não continha JSON válido
    #[error("Failed to parse sidecar output: {reason}")]
    ParseError { reason: String },
}
```

---

## 3. Fluxo de Execução (Sequência Mecânica)

```
JCodemunchSidecar::extract()
│
├─ 1. Monta o vetor de argumentos do CLI jcodemunch:
│     ["index", "--format", "json", "--stdout", "<repo_path>"]
│
├─ 2. Invoca sandbox.execute("jcodemunch", &args)
│     ├─ SandboxHandle spawna tokio::process::Command (PT-3)
│     ├─ stdout capturado como Vec<u8> (PT-2: Zero arquivos intermediários)
│     └─ Timeout gerenciado pelo SandboxHandle (SIGKILL automático)
│
├─ 3. Recebe Result<Vec<u8>, SandboxError>
│     ├─ Err(SandboxError::Timeout) → Traduz para SidecarError::Timeout
│     ├─ Err(SandboxError::*) → Traduz para SidecarError::ExecutionFailed
│     └─ Ok(raw_bytes) → Prossegue para parsing
│
├─ 4. Desserializa raw_bytes via serde_json::from_slice::<AstPayload>()
│     ├─ Err(serde_error) → SidecarError::ParseError
│     └─ Ok(payload) → Retorna AstPayload
│
└─ 5. Retorna Result<AstPayload, SidecarError>
```

### 3.1. Nota sobre o Timeout

O `JCodemunchSidecar` **não gerencia timeout diretamente**. Ele delega
integralmente ao `SandboxHandle::execute()`, que já implementa:

- `tokio::time::timeout` com SIGKILL automático.
- Remoção do PID da guilhotina após o kill (D3 do PRD-003).
- Leitura concorrente de stdout/stderr via `tokio::join!` (D2 do PRD-003).

O campo `timeout_secs` na `JCodemunchInput` é passado ao `SandboxHandle`
para configurar o limite. Se o `SandboxHandle` atual usar um timeout fixo
(30s), o PRD-006 deve **propagar** o timeout configurado, não duplicar a lógica.

### 3.2. Tradução de Erros (Camada Anti-Leak)

O `JCodemunchSidecar` **nunca** expõe `SandboxError` ao chamador. Ele
traduz todos os erros do sandbox para variantes de `SidecarError`,
mantendo a separação de camadas:

| SandboxError | → | SidecarError |
|---|---|---|
| `Timeout` | → | `Timeout { timeout_secs }` |
| `ProcessSpawnFailed` | → | `BinaryNotFound` (se "not found" no reason) ou `ExecutionFailed` |
| `PrivilegeError` | → | `ExecutionFailed` |
| `UnsupportedPlatform` | → | `ExecutionFailed` |

---

## 4. Proibições Tóxicas

### PT-SIDECAR-1: PROIBIDO Ler Código-Fonte na Main Thread

| Abordagem SLOP | Risco Letal |
|---|---|
| `std::fs::read_to_string("src/main.rs")` no SODA para "preparar" o input | Megabytes de código carregados na RAM da thread principal, asfixiando o Tokio |

**Lei Dura:** O SODA **NUNCA** abre arquivos `.rs`, `.ts`, `.py`, `.go` ou qualquer
código-fonte. A leitura de arquivos é responsabilidade **exclusiva** do binário
`jcodemunch` rodando dentro do sandbox. O SODA apenas recebe o resultado
estruturado via `stdout` do subprocesso.

### PT-SIDECAR-2: IPC Zero-Garbage (Proibição de Arquivos Intermediários)

| Abordagem SLOP | Risco Letal |
|---|---|
| `jcodemunch > /tmp/ast_output.json && cat /tmp/ast_output.json` | Arquivo intermediário no disco: vetor de SDC, lixo residual, I/O desnecessário |

**Lei Dura:** O output do `jcodemunch` trafega **exclusivamente** pelo pipe de
`stdout`, capturado via `tokio::process::Command` (já implementado no
`SandboxHandle::execute()`). O buffer `Vec<u8>` é desserializado diretamente
para `AstPayload` via `serde_json::from_slice`. Zero arquivos temporários.

### PT-3: PROIBIDO Bloquear o Event Loop do Tokio (Herdado)

| Abordagem SLOP | Risco Letal |
|---|---|
| `std::process::Command::output()` síncrono | Congela a thread do Tokio durante a extração AST (potencialmente dezenas de segundos) |

**Lei Dura:** Toda invocação do `jcodemunch` ocorre via `SandboxHandle::execute()`,
que já usa `tokio::process::Command` com captura assíncrona. O `JCodemunchSidecar`
**herda** esta garantia sem reimplementá-la.

---

## 5. Cenário de Falha Isolado

### 5.1. OOM / Timeout — Guilhotina Atômica

**Gatilho:** O `jcodemunch` processa um monorepo gigante (>10k arquivos) e
excede o limite de memória do sandbox (cgroups v2) ou o timeout configurado.

**Comportamento:**

```
jcodemunch (PID: 12345) excede limite
  → SandboxHandle::execute() detecta Timeout
    → child.kill().await (SIGKILL incondicional)
    → PID removido da guilhotina (D3 PRD-003)
    → Retorna SandboxError::Timeout
      → JCodemunchSidecar traduz para SidecarError::Timeout { timeout_secs: 120 }
```

**Garantias:**

1. **Zero processos órfãos:** O SIGKILL é incondicional. Se o `SandboxHandle`
   for dropado antes do timeout, o `Drop` mata todos os PIDs ativos.
2. **Zero corrupção de estado:** O `JCodemunchSidecar` não mantém estado interno.
   Cada invocação é atômica e independente.
3. **Fail-Closed:** O erro `SidecarError::Timeout` propaga-se ao chamador. O
   pipeline pode decidir pular este repositório ou re-tentar com timeout maior.

### 5.2. JSON Corrompido — Parse Defensivo

**Gatilho:** O `jcodemunch` retorna exit code 0 mas produz JSON malformado
(bug no sidecar, truncamento de stdout por buffer overflow parcial).

**Comportamento:**

```
stdout: Vec<u8> = [bytes corrompidos]
  → serde_json::from_slice::<AstPayload>() → Err(serde_json::Error)
    → SidecarError::ParseError { reason: "invalid JSON at line 42, column 7" }
```

**Garantias:** O SODA **nunca** interpreta bytes crus como dados válidos.
A barreira do `serde_json::from_slice` garante que apenas payloads
100% conformes ao schema da `AstPayload` passam.

---

## 6. Dependências (Cargo.toml)

| Crate | Uso | Tipo |
|---|---|---|
| `serde` | Derive `Deserialize` na `AstPayload` | **dependencies** (features: `["derive"]`) |
| `serde_json` | `from_slice` para parsing do stdout | **dependencies** |
| `thiserror` | Derive `Error` no `SidecarError` | Já presente |

> **Nota:** `serde` e `serde_json` provavelmente já estarão no `Cargo.toml`
> quando o projeto integrar o Tauri v2. Se não estiverem, devem ser adicionados
> como dependências de produção (não dev-dependencies), pois o parsing de
> JSON do sidecar ocorre em runtime.

---

## 7. Definition of Done (DoD)

A Fase C (TDD) DEVE comprovar mecanicamente os seguintes critérios:

| # | Critério | Teste Correspondente |
|---|---|---|
| 1 | Extração bem-sucedida: sandbox retorna JSON válido → `AstPayload` desserializado | `test_extract_success` |
| 2 | Sidecar não encontrado: sandbox retorna erro de spawn → `SidecarError::BinaryNotFound` | `test_binary_not_found` |
| 3 | Execução falha (exit code != 0): → `SidecarError::ExecutionFailed` com stderr no reason | `test_execution_failed` |
| 4 | Timeout: sandbox retorna `SandboxError::Timeout` → `SidecarError::Timeout` | `test_timeout_propagation` |
| 5 | JSON corrompido: stdout contém bytes inválidos → `SidecarError::ParseError` | `test_invalid_json` |
| 6 | JSON vazio (`[]` ou `{}`): stdout contém JSON válido mas semanticamente vazio → `AstPayload` com vetores vazios (não é erro) | `test_empty_repo_valid_json` |
| 7 | A função `extract` é `async` (usa `SandboxHandle::execute` assíncrono) | Compilação (assinatura `async fn`) |
| 8 | Zero imports de `std::fs` ou `std::process` no módulo | Inspeção estática na Fase D |
| 9 | Zero `unwrap()` na lógica de produção | Inspeção estática na Fase D |

### 7.1. Estratégia de Mock para TDD

Os testes **NÃO** devem invocar o binário `jcodemunch` real. A estratégia:

1. **Mock do SandboxHandle:** Os testes injetam respostas pré-definidas
   (JSON válido, JSON corrompido, erro de timeout) simulando o retorno de
   `sandbox.execute()`.
2. **Alternativa:** Criar uma trait `SandboxExecutor` que o `SandboxHandle`
   implementa, e usar um `MockExecutor` nos testes que retorna `Vec<u8>`
   ou `SandboxError` conforme o cenário.
3. **JSON de teste:** Fixtures mínimos com 1-2 símbolos e 1 aresta de
   dependência, embutidos como `const &str` nos testes (sem arquivos de fixture).

---

## 8. Interface com o DAG

```
N3 (SandboxOrchestrator)          N6 (JCodemunchSidecar)
  ├─ create() → SandboxHandle ──▶   ├─ extract(JCodemunchInput)
  │                                   │  → Result<AstPayload, SidecarError>
  │                                   │
N5 (ExtractionRouter)                │    [stdout: Vec<u8>]
  ├─ route() → [RunJCodemunch] ──▶   │      ↓
  │                                   │    serde_json::from_slice
  │                                   │      ↓
N2 (BloblessCloner)                  │    AstPayload { symbols, edges, count }
  ├─ clone() → RepoPath ───────▶    │      ↓
                                      └──▶ N12 (BlobNormalizer)
```

---

## 9. Decisões Arquiteturais

### 9.1. Trait `SandboxExecutor` vs. Invocação Direta

**Decisão:** Introduzir uma trait `SandboxExecutor` para desacoplar o
sidecar do `SandboxHandle` concreto:

```rust
#[async_trait]
pub trait SandboxExecutor {
    async fn execute(&self, command: &str, args: &[&str]) -> Result<Vec<u8>, SandboxError>;
    fn repo_path(&self) -> &Path;
}
```

**Justificativa:**

1. **Testabilidade:** Permite injetar `MockExecutor` nos testes sem
   precisar de um sandbox real.
2. **Extensibilidade:** Futuramente, diferentes estratégias de sandbox
   (WSB vs LPAC vs Landlock) podem implementar a mesma trait.
3. **Inversão de Dependência:** O sidecar depende da abstração, não do concreto.

> **Nota de Cuidado:** A trait `SandboxExecutor` requer a crate `async-trait`
> (ou Rust 1.75+ com `async fn in traits` estabilizado). Verificar a versão
> do Rust toolchain antes da Fase C. Se o `async fn in traits` estiver
> estável (Rust ≥ 1.75), **PROIBIDO** usar `async-trait` — usar a feature nativa.

### 9.2. `AstPayload` como Struct Tipada vs. `Vec<u8>` Cru

**Decisão:** O N6 desserializa o JSON para `AstPayload` tipada antes de
devolver ao chamador.

**Alternativa rejeitada:** Retornar `Vec<u8>` cru e delegar o parsing
para o N12. Razão da rejeição: propagar bytes crus viola o princípio de
fail-fast. Se o JSON estiver corrompido, o erro deve ser detectado no N6
(perto da fonte), não no N12 (tarde demais).

### 9.3. Timeout Configurável vs. Fixo

O `timeout_secs` é passado como campo da `JCodemunchInput`, não hardcoded.
Isso permite que o orquestrador ajuste o timeout por repositório (repos
pequenos: 30s, monorepos: 300s) sem recompilar.
