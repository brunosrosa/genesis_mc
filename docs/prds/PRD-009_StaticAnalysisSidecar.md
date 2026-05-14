# PRD-009: StaticAnalysisSidecar

> **Nó DAG:** N9 (Depende de N5 — ExtractionRouter, reutiliza N3 — SandboxOrchestrator)
> **Módulo Rust:** `sidecar::StaticAnalysisSidecar`
> **Território:** PRODUTO (Daemon SODA — Rust/Tokio)
> **Classificação:** EXTRAÇÃO — Sidecar Efêmero de Qualidade de Código (Camada 2)
> **Target FinOps:** `local_slm` (custo ZERO)

---

## 1. Objetivo

Executar um linter de análise estática (ex: `clippy-driver`, `oxlint`,
`ruff`, `golangci-lint`) dentro do `SandboxHandle` para extrair as
**violações de qualidade de código** — warnings, erros estilísticos,
code smells e potenciais bugs — de um repositório clonado no Ramdisk,
devolvendo os resultados como uma struct Rust tipada
(`StaticAnalysisPayload`) sem tocar o disco do host e sem bloquear o
Event Loop.

O `StaticAnalysisSidecar` é o **Cão de Guarda de Qualidade** do pipeline.
Ele não compila código, não executa testes, e não renderiza componentes.
Ele executa **análise estática pura** sobre os arquivos-fonte, extraindo
diagnósticos estruturados que serão consumidos pelo `BlobNormalizer` (N12)
para persistência no `soda_heuristic_vault.db`.

### 1.1. Diferença dos Outros Sidecars (N6/N7)

| Propriedade | JCodemunchSidecar (N6) | OxcSidecar (N7) | StaticAnalysisSidecar (N9) |
|---|---|---|---|
| Propósito | Extração de AST e Grafo de Deps | Extração de Contratos UX (Props/Events) | Extração de Violações de Qualidade |
| Escopo de Análise | Estrutural (esqueleto do código) | Interfaces de componentes UI | Diagnósticos e code smells |
| Binário alvo | `jcodemunch` | `oxlint` | Dinâmico por stack (vide §3.2) |
| Ativação | `RunJCodemunch` | `RunOxc` | `RunStaticAnalysis` |

### 1.2. Condição de Ativação

O `StaticAnalysisSidecar` é invocado quando o `ExtractionRouter` (N5)
emite a tarefa `ExtractionTask::RunStaticAnalysis`. Isso ocorre para
**todas** as stacks com linter disponível:

| StackProfile | RunStaticAnalysis emitido? | Linter Alvo |
|---|---|---|
| `Rust` | ✅ Sim | `clippy-driver` (JSON output) |
| `NodeJS` | ✅ Sim | `oxlint` (JSON output) |
| `Python` | ✅ Sim | `ruff` (JSON output) |
| `Go` | ✅ Sim | `golangci-lint` (JSON output) |
| `JVM` | ❌ Não (Fase 2) | — |
| `DotNet` | ❌ Não (Fase 2) | — |
| `Mixed(stacks)` | ✅ Sim (para cada stack com linter) | Variável |
| `Unknown` | ❌ Não | — |

### 1.3. Princípio do Sidecar Efêmero (Herdado do PRD-006)

O linter de análise estática é tratado como um processo descartável que:

1. Nasce dentro do sandbox (isolamento LPAC/Landlock).
2. Lê os arquivos-fonte diretamente no Ramdisk.
3. Despeja seu output em `stdout` como JSON estruturado.
4. Morre atomicamente após a execução (SIGKILL via `SandboxHandle::Drop`).

---

## 2. Contrato I/O (Régua Atômica)

### 2.1. Entrada

```rust
pub struct StaticAnalysisInput<'a, E: SandboxExecutor> {
    pub executor: &'a E,
    pub timeout_secs: u64,
}
```

| Campo | Tipo | Origem | Semântica |
|---|---|---|---|
| `executor` | `&E: SandboxExecutor` | N3 (via trait abstrata) | Gaiola de segurança com policy `ReadOnly` aplicada |
| `timeout_secs` | `u64` | Configuração do pipeline (padrão: `60`) | Limite de execução — SIGKILL após estourar |

> **Nota D2 (Consolidada dos PRDs 006/007):** O `repo_path` foi
> deliberadamente excluído da struct de input. O
> `SandboxExecutor::execute()` já aplica `current_dir(&self.repo_path)`
> internamente. Passar `repo_path` como campo seria um campo orphan
> (nunca lido pela função `extract`). Design DRY consolidado.

### 2.2. Saída

```rust
pub async fn extract<E: SandboxExecutor>(
    input: StaticAnalysisInput<'_, E>,
) -> Result<StaticAnalysisPayload, SidecarError>
```

### 2.3. StaticAnalysisPayload (Struct de Saída Tipada)

```rust
#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
pub struct StaticAnalysisPayload {
    /// Lista de violações/warnings/erros encontrados pelo linter
    pub violations: Vec<LintViolation>,

    /// Total de arquivos analisados pelo linter
    pub files_analyzed: u32,

    /// Nome do linter utilizado (ex: "clippy", "oxlint", "ruff")
    pub linter_name: String,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
pub struct LintViolation {
    /// Identificador da regra violada (ex: "clippy::unwrap_used",
    /// "no-unused-vars", "F401")
    pub rule_id: String,

    /// Severidade: "error", "warning", "info", "hint"
    pub severity: String,

    /// Mensagem legível do diagnóstico
    pub message: String,

    /// Arquivo de origem relativo ao repo_path
    pub file_path: String,

    /// Linha de início da violação (1-indexed)
    pub line: u32,

    /// Coluna de início da violação (1-indexed, opcional)
    pub column: Option<u32>,
}
```

> **Decisão Arquitetural:** O `StaticAnalysisPayload` é agnóstico ao
> linter. O campo `linter_name` identifica a ferramenta utilizada, mas
> a struct `LintViolation` é um formato unificado. A responsabilidade de
> mapear o output específico de cada linter (Clippy JSON, oxlint JSON,
> ruff JSON) para este formato cabe às funções de parsing internas do
> sidecar.

### 2.4. SidecarError (Reutilizado do PRD-006)

O `StaticAnalysisSidecar` reutiliza **integralmente** o enum `SidecarError`
já definido em `sidecar.rs`:

```rust
pub enum SidecarError {
    BinaryNotFound { binary: String },
    ExecutionFailed { reason: String },
    Timeout { timeout_secs: u64 },
    ParseError { reason: String },
}
```

Não há necessidade de criar variantes adicionais. A mesma tradução
`SandboxError → SidecarError` consolidada na função `execute_sidecar()`
se aplica aqui.

---

## 3. Fluxo de Execução (Sequência Mecânica)

```
StaticAnalysisSidecar::extract()
│
├─ 1. Determina o comando e argumentos do linter conforme a stack:
│     ├─ Rust:   ["clippy-driver", "--edition", "2021", "-W", "clippy::all",
│     │           "--error-format=json", "src/lib.rs"]
│     ├─ NodeJS: ["oxlint", "--format", "json", "--quiet",
│     │           "--ignore-pattern", "node_modules", "--ignore-pattern", "test*",
│     │           "--ignore-pattern", "*.test.*", "--ignore-pattern", "*.spec.*", "."]
│     ├─ Python: ["ruff", "check", "--output-format", "json",
│     │           "--exclude", "tests,test,*_test.py", "."]
│     └─ Go:     ["golangci-lint", "run", "--out-format", "json",
│                  "--exclude-dirs", "vendor,testdata", "./..."]
│
├─ 2. Invoca execute_sidecar(executor, binary, &args, timeout_secs)
│     ├─ SandboxHandle spawna tokio::process::Command (PT-3)
│     ├─ stdout capturado como Vec<u8> (PT-SIDECAR-2: Zero arquivos intermediários)
│     └─ Timeout gerenciado pelo SandboxHandle (SIGKILL automático)
│
├─ 3. Recebe Result<Vec<u8>, SidecarError>
│     ├─ Err(SidecarError::Timeout) → Propaga (Fail-Fast)
│     ├─ Err(SidecarError::BinaryNotFound) → Propaga
│     ├─ Err(SidecarError::*) → Propaga
│     └─ Ok(raw_bytes) → Prossegue para parsing
│
├─ 4. Mapeia raw_bytes para StaticAnalysisPayload:
│     ├─ Parse JSON bruto via serde_json::from_slice
│     ├─ Mapeia formato específico do linter → Vec<LintViolation>
│     └─ Err(serde_error) → SidecarError::ParseError
│
└─ 5. Retorna Result<StaticAnalysisPayload, SidecarError>
```

### 3.1. Nota sobre Timeout

Herda a mesma arquitetura dos PRDs 006/007: o `StaticAnalysisSidecar`
**não gerencia timeout diretamente**. Delega integralmente ao
`SandboxHandle::execute()` via a função `execute_sidecar()`.

### 3.2. Resolução Dinâmica do Linter (Fase 1 — Conservadora)

Na Fase 1, o `StaticAnalysisSidecar` opera com um **único linter
pré-definido** por invocação. O `ExtractionRouter` (N5) é responsável
por selecionar o binário correto com base no `StackProfile` detectado
pelo N4 (`LanguageDetector`).

A resolução é passada ao sidecar via um campo adicional na struct de
input (Fase 2) ou via a assinatura dos argumentos (Fase 1 simplificada).

**Fase 1 — Implementação Mínima:**

O `StaticAnalysisSidecar` aceita o nome do linter e os argumentos como
parâmetros, usando a função `execute_sidecar()` centralizada. A escolha
de qual linter invocar é **externa** ao sidecar (decidida pelo Router).

### 3.3. Tradução de Erros (Camada Anti-Leak)

Idêntica aos PRDs 006/007. Reutiliza **integralmente** a função
`execute_sidecar()` já consolidada em `sidecar.rs`:

| SandboxError | → | SidecarError |
|---|---|---|
| `Timeout` | → | `Timeout { timeout_secs }` |
| `ProcessSpawnFailed` com "not found" | → | `BinaryNotFound { binary }` |
| `ProcessSpawnFailed` sem "not found" | → | `ExecutionFailed { reason }` |
| Qualquer outro | → | `ExecutionFailed { reason: e.to_string() }` |

---

## 4. Proibições Tóxicas

### PT-SIDECAR-1: PROIBIDO Ler Código-Fonte na Main Thread (Herdado)

**Lei Dura:** O SODA **NUNCA** abre arquivos de código-fonte (`.rs`,
`.ts`, `.py`, `.go`, `.java`, `.cs`) diretamente. A leitura é
responsabilidade **exclusiva** do binário do linter rodando dentro do
sandbox.

### PT-SIDECAR-2: IPC Zero-Garbage (Herdado)

**Lei Dura:** O output do linter trafega **exclusivamente** pelo pipe de
`stdout`. O buffer `Vec<u8>` é desserializado diretamente para
`StaticAnalysisPayload` via `serde_json::from_slice`. Zero arquivos
temporários. Zero conversão intermediária para `String`.

### PT-ANALYSIS-1: Strict Execution Bounds (Nova)

| Abordagem SLOP | Risco Letal |
|---|---|
| `oxlint .` sem filtros (escopo aberto) | Monorepo com 50k arquivos: o linter entra em loop por horas, a VRAM grita OOM e o pipeline congela |
| `ruff check .` sem `--exclude` | Varre `node_modules/`, `.venv/`, `dist/`, `build/` — milhões de linhas irrelevantes. Tempo de análise explode para 10min+ |
| `clippy` sem `--cap-lints` ou target restrito | Recompila todas as dependências transitivas para checar warnings. Timeout estourado pelo build puro |

**Lei Dura:** É TERMINANTEMENTE PROIBIDO rodar linters com escopo aberto.
Toda invocação do sidecar DEVE repassar parâmetros restritivos ao linter:

1. **Excluir diretórios de dependências:** `node_modules/`, `vendor/`,
   `.venv/`, `target/`, `dist/`, `build/`.
2. **Excluir diretórios de testes:** `tests/`, `test/`, `__tests__/`,
   `*_test.*`, `*.spec.*` (o escopo é código de produção, não cobertura).
3. **Limitar profundidade:** Se o linter suportar, restringir a análise
   ao diretório de código-fonte principal (`src/`, `lib/`, `pkg/`).
4. **Output JSON obrigatório:** Todo linter DEVE ser invocado com a flag
   de output JSON estruturado (`--format json`, `--output-format json`,
   `--error-format=json`). Output em texto livre está BANIDO.

### PT-3: PROIBIDO Bloquear o Event Loop do Tokio (Herdado)

Toda invocação ocorre via `SandboxExecutor::execute()` (async) através
da função `execute_sidecar()`.

---

## 5. Cenário de Falha Isolado

### 5.1. Linter Entra em Loop Infinito — AST Complexa / Timeout

**Gatilho:** O linter de análise estática processa um repositório com
construções de linguagem patológicas (macros recursivas em Rust, templates
C++ aninhados, monorepo com milhares de arquivos Python com imports
circulares). O linter entra em loop de resolução de tipos ou esgota a
memória.

**Comportamento:**

```
linter (PID: 67890) excede timeout de 60s ou limite de memória
  → SandboxHandle::execute() detecta Timeout
    → child.kill().await (SIGKILL incondicional)
    → PID removido da guilhotina (D3 do PRD-003)
    → Retorna SandboxError::Timeout
      → execute_sidecar() traduz para SidecarError::Timeout { timeout_secs: 60 }
        → StaticAnalysisSidecar propaga Err para o chamador
```

**Garantias:**

1. **Zero processos órfãos:** SIGKILL incondicional. `Drop` mata PIDs
   ativos.
2. **Zero corrupção de estado:** O `StaticAnalysisSidecar` não mantém
   estado interno. Cada invocação é atômica e independente.
3. **Fail-Closed:** Erro propaga-se ao chamador. O Harvester pode decidir
   pular o repositório ou registrar a falha na telemetria sem derrubar o
   pipeline.
4. **Proteção de RAM do Host:** O linter roda isolado no sandbox. Se
   exceder a memória do cgroup, é eliminado sem afetar o processo
   principal do SODA.

### 5.2. JSON Corrompido / Output Parcial (Herdado do padrão PRD-006)

```
stdout: Vec<u8> = [bytes corrompidos ou truncados]
  → serde_json::from_slice::<RawLinterOutput>() → Err
    → SidecarError::ParseError { reason: "..." }
```

### 5.3. Linter Não Instalado

```
execute_sidecar(executor, "ruff", &args, timeout_secs)
  → executor.execute("ruff", &args).await → Err(ProcessSpawnFailed)
    → reason.contains("not found")
      → SidecarError::BinaryNotFound { binary: "ruff" }
```

---

## 6. Dependências (Cargo.toml)

**Nenhuma dependência nova.** O `StaticAnalysisSidecar` reutiliza:

| Crate | Uso | Já presente? |
|---|---|---|
| `serde` | Derive `Deserialize` na `StaticAnalysisPayload` | ✅ Sim |
| `serde_json` | `from_slice` para parsing do stdout | ✅ Sim |
| `thiserror` | Reutiliza `SidecarError` existente | ✅ Sim |

---

## 7. Definition of Done (DoD)

A Fase C (TDD) DEVE comprovar mecanicamente os seguintes critérios:

| # | Critério | Teste Correspondente |
|---|---|---|
| 1 | Extração bem-sucedida: executor retorna JSON válido com violações → `StaticAnalysisPayload` desserializado | `test_static_analysis_success` |
| 2 | Binary não encontrado: executor retorna erro de spawn → `SidecarError::BinaryNotFound { binary: "ruff" }` | `test_linter_not_found` |
| 3 | Execução falha (exit code != 0 sem output útil): → `SidecarError::ExecutionFailed` | `test_execution_failed` |
| 4 | Timeout: executor retorna `SandboxError::Timeout` → `SidecarError::Timeout` | `test_timeout_propagation` |
| 5 | JSON corrompido: stdout contém bytes inválidos → `SidecarError::ParseError` | `test_invalid_json` |
| 6 | Repositório limpo (zero violações): JSON válido com listas vazias → `StaticAnalysisPayload` com vetores vazios (não é erro) | `test_clean_repo_valid_json` |
| 7 | A função `extract` é `async` (usa `SandboxExecutor::execute` assíncrono) | Compilação (assinatura `async fn`) |
| 8 | Os argumentos do linter incluem flags restritivas (PT-ANALYSIS-1) | Inspeção estática na Fase D |
| 9 | Zero imports de `std::fs` ou `std::process` no módulo | Inspeção estática na Fase D |
| 10 | Zero `unwrap()` na lógica de produção | Inspeção estática na Fase D |

### 7.1. Estratégia de Mock para TDD

Reutiliza **integralmente** o `MockExecutor` já criado nos testes do
PRD-006 (`sidecar.rs`). Os testes injetam respostas pré-definidas
simulando o stdout do linter no formato JSON unificado.

---

## 8. Interface com o DAG

```
N5 (ExtractionRouter)               N9 (StaticAnalysisSidecar)
  ├─ route() → [RunStaticAnalysis]     ├─ extract(StaticAnalysisInput)
  │   (stacks com linter disponível)   │  → Result<StaticAnalysisPayload, SidecarError>
  │                                    │
N3 (SandboxOrchestrator)              │    [stdout: Vec<u8>]
  ├─ create() → SandboxHandle         │      ↓
  │   (via trait SandboxExecutor) ─▶  │    serde_json::from_slice
  │                                    │      ↓
                                       │    StaticAnalysisPayload { violations, files_analyzed, linter_name }
                                       └──▶ N12 (BlobNormalizer)
```

---

## 9. Decisões Arquiteturais

### 9.1. Co-localização com JCodemunchSidecar e OxcSidecar

**Decisão:** O `StaticAnalysisSidecar` reside no **mesmo arquivo**
`sidecar.rs`, abaixo do `OxcSidecar`.

**Justificativa:**

1. **Reutilização da infraestrutura:** Reutiliza `SandboxExecutor`,
   `SidecarError`, `execute_sidecar()` e `MockExecutor`.
2. **Coesão temática:** Todos são Sidecars Efêmeros da Camada 2.
3. **Zero duplicação:** O `MockExecutor` e o fluxo de tradução de erros
   são compartilhados entre os três sidecars.

Se o arquivo `sidecar.rs` ultrapassar ~600 linhas após esta adição,
fracionar em `sidecar/jcodemunch.rs`, `sidecar/oxc.rs`,
`sidecar/static_analysis.rs` com um `sidecar/mod.rs` central que
re-exporta a trait, o enum de erro e a função `execute_sidecar()`.

### 9.2. Formato Unificado vs. Formato Nativo do Linter

**Decisão:** O `StaticAnalysisSidecar` parseia o JSON nativo de cada
linter e o **normaliza** para o formato `LintViolation` unificado.

**Alternativa rejeitada:** Retornar o JSON cru e delegar normalização
ao N12. Rejeição: viola fail-fast. Se o JSON do linter mudar de schema
entre versões, o erro deve ser detectado no N9, não no N12.

**Implementação (Fase 1 — Parsing Conservador):**

Na Fase 1, o `StaticAnalysisSidecar` implementa parsing **apenas** para
o formato JSON de um linter (ex: `ruff` ou `oxlint`). Os demais linters
retornam `StaticAnalysisPayload` com `violations: vec![]` e
`linter_name` preenchido, sinalizando presença do linter sem parsing
profundo. O parsing dos formatos adicionais é adiado para a Fase 2.

### 9.3. `severity` como String vs. Enum

**Decisão:** Manter `severity` como `String` (não enum).

**Justificativa:** Cada linter usa terminologias ligeiramente diferentes
("error" vs "E", "warning" vs "W", "convention" vs "C"). Uma `String`
com valores normalizados (`"error"`, `"warning"`, `"info"`, `"hint"`)
oferece flexibilidade sem breaking changes no schema de desserialização
ao adicionar suporte a novos linters.

### 9.4. Exit Code de Linters ≠ Falha

**Decisão:** Linters frequentemente retornam exit code != 0 quando
**encontram** violações (ex: `ruff` retorna 1 se houver warnings).
Isso **não** é um erro de execução.

O `StaticAnalysisSidecar` deve tratar exit codes de linters com
inteligência:

- **Exit code 0:** Execução limpa (zero violações ou todas silenciadas).
- **Exit code 1:** Violações encontradas — **parsear stdout normalmente**
  (não é `ExecutionFailed`).
- **Exit code 2+:** Erro real de execução (config inválida, crash) →
  `SidecarError::ExecutionFailed`.

> **Nota para a Fase C:** A função `execute_sidecar()` atualmente traduz
> qualquer erro do sandbox para `SidecarError`. Se o `SandboxHandle`
> tratar exit code != 0 como `SandboxError`, pode ser necessário ajustar
> a lógica para distinguir "violações encontradas" (exit 1) de "crash
> real" (exit 2+). Avaliar na implementação.
