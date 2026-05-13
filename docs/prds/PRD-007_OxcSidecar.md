# PRD-007: OxcSidecar

> **Nó DAG:** N7 (Depende de N5 — ExtractionRouter, reutiliza N3 — SandboxOrchestrator)
> **Módulo Rust:** `sidecar::OxcSidecar`
> **Território:** PRODUTO (Daemon SODA — Rust/Tokio)
> **Classificação:** EXTRAÇÃO — Sidecar Efêmero de Contratos UX (Camada 2)
> **Target FinOps:** `local_slm` (custo ZERO)

---

## 1. Objetivo

Executar o binário `oxc_linter` (ou `oxlint`) dentro do `SandboxHandle` para
extrair os **Contratos de Interface UX** de um repositório frontend —
especificamente: Props, Events, Slots/Children e exports de componentes
React/Vue/Svelte — devolvendo os resultados como uma struct Rust tipada
(`UxContractsPayload`) sem tocar o disco do host e sem bloquear o Event Loop.

O `OxcSidecar` é o **Extrator de Contratos UX** do pipeline. Ele não
renderiza componentes nem emula DOM. Ele realiza **Análise Estática Pura**
sobre a AST dos arquivos `.tsx`, `.jsx`, `.vue`, `.svelte` para extrair
declarações de interface (tipo TypeScript `interface Props {}`, `export default`,
`defineProps`, `$props`) que revelam o contrato público dos componentes.

### 1.1. Condição de Ativação

O `OxcSidecar` é invocado **exclusivamente** quando o `ExtractionRouter` (N5)
emite a tarefa `ExtractionTask::RunOxc`. Isso ocorre apenas para stacks
com frontend:

| StackProfile | RunOxc emitido? |
|---|---|
| `NodeJS` | ✅ Sim |
| `Mixed(stacks)` contendo `NodeJS` | ✅ Sim (via deduplicação) |
| `Rust`, `Go`, `Python`, `JVM`, `DotNet` | ❌ Não |
| `Unknown` | ❌ Não |

### 1.2. Princípio do Sidecar Efêmero (Herdado do PRD-006)

O `oxlint` é tratado como um processo descartável que:

1. Nasce dentro do sandbox (isolamento LPAC/Landlock).
2. Lê os arquivos de componentes frontend diretamente no Ramdisk.
3. Despeja seu output em `stdout` como JSON estruturado.
4. Morre atomicamente após a execução (SIGKILL via `SandboxHandle::Drop`).

---

## 2. Contrato I/O (Régua Atômica)

### 2.1. Entrada

```rust
pub struct OxcInput<'a, E: SandboxExecutor> {
    pub executor: &'a E,
    pub timeout_secs: u64,
}
```

| Campo | Tipo | Origem | Semântica |
|---|---|---|---|
| `executor` | `&E: SandboxExecutor` | N3 (via trait abstrata) | Gaiola de segurança com policy `ReadOnly` aplicada |
| `timeout_secs` | `u64` | Configuração do pipeline (padrão: `90`) | Limite de execução — SIGKILL após estourar |

> **Nota D2 (Lição do PRD-006):** O `repo_path` foi deliberadamente excluído
> da struct de input. O `SandboxExecutor::execute()` já aplica
> `current_dir(&self.repo_path)` internamente. Passar `repo_path` como campo
> seria um campo orphan (nunca lido pela função `extract`).

### 2.2. Saída

```rust
pub async fn extract<E: SandboxExecutor>(
    input: OxcInput<'_, E>,
) -> Result<UxContractsPayload, SidecarError>
```

### 2.3. UxContractsPayload (Struct de Saída Tipada)

```rust
#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
pub struct UxContractsPayload {
    /// Lista de componentes de UI com seus contratos (Props/Events/Slots)
    pub components: Vec<ComponentContract>,

    /// Total de arquivos frontend analisados pelo oxlint
    pub files_analyzed: u32,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
pub struct ComponentContract {
    /// Nome do componente (ex: "Button", "UserCard", "NavBar")
    pub name: String,

    /// Arquivo de origem relativo ao repo_path
    pub file_path: String,

    /// Framework detectado: "react", "vue", "svelte", "solid", "unknown"
    pub framework: String,

    /// Lista de Props declaradas no contrato público
    pub props: Vec<PropDeclaration>,

    /// Lista de Events emitidos pelo componente
    pub events: Vec<String>,

    /// Indica se o componente exporta como default export
    pub is_default_export: bool,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
pub struct PropDeclaration {
    /// Nome da prop (ex: "onClick", "variant", "disabled")
    pub name: String,

    /// Tipo TypeScript inferido (ex: "string", "boolean", "() => void")
    pub prop_type: String,

    /// Se a prop possui um valor default
    pub has_default: bool,

    /// Se a prop é obrigatória
    pub required: bool,
}
```

### 2.4. SidecarError (Reutilizado do PRD-006)

O `OxcSidecar` reutiliza **integralmente** o enum `SidecarError` já definido
em `sidecar.rs`:

```rust
pub enum SidecarError {
    BinaryNotFound { binary: String },
    ExecutionFailed { reason: String },
    Timeout { timeout_secs: u64 },
    ParseError { reason: String },
}
```

Não há necessidade de criar variantes adicionais. A mesma tradução
`SandboxError → SidecarError` do PRD-006 se aplica aqui.

---

## 3. Fluxo de Execução (Sequência Mecânica)

```
OxcSidecar::extract()
│
├─ 1. Monta o vetor de argumentos do CLI oxlint:
│     ["lint", "--format", "json", "--quiet", "."]
│
├─ 2. Invoca executor.execute("oxlint", &args)
│     ├─ SandboxHandle spawna tokio::process::Command (PT-3)
│     ├─ stdout capturado como Vec<u8> (PT-2: Zero arquivos intermediários)
│     └─ Timeout gerenciado pelo SandboxHandle (SIGKILL automático)
│
├─ 3. Recebe Result<Vec<u8>, SandboxError>
│     ├─ Err(SandboxError::Timeout) → SidecarError::Timeout { timeout_secs }
│     ├─ Err(ProcessSpawnFailed "not found") → SidecarError::BinaryNotFound { "oxlint" }
│     ├─ Err(SandboxError::*) → SidecarError::ExecutionFailed { reason }
│     └─ Ok(raw_bytes) → Prossegue para parsing
│
├─ 4. Desserializa raw_bytes via serde_json::from_slice::<UxContractsPayload>()
│     ├─ Err(serde_error) → SidecarError::ParseError
│     └─ Ok(payload) → Retorna UxContractsPayload
│
└─ 5. Retorna Result<UxContractsPayload, SidecarError>
```

### 3.1. Nota sobre Timeout

Herda a mesma arquitetura do PRD-006: o `OxcSidecar` **não gerencia timeout
diretamente**. Delega integralmente ao `SandboxHandle::execute()`.

### 3.2. Tradução de Erros (Camada Anti-Leak)

Idêntica ao PRD-006. O binário buscado muda de `"jcodemunch"` para `"oxlint"`:

| SandboxError | → | SidecarError |
|---|---|---|
| `Timeout` | → | `Timeout { timeout_secs }` |
| `ProcessSpawnFailed` com "not found" | → | `BinaryNotFound { binary: "oxlint" }` |
| `ProcessSpawnFailed` sem "not found" | → | `ExecutionFailed { reason }` |
| Qualquer outro | → | `ExecutionFailed { reason: e.to_string() }` |

---

## 4. Proibições Tóxicas

### PT-SIDECAR-1: PROIBIDO Ler Código-Fonte na Main Thread (Herdado)

**Lei Dura:** O SODA **NUNCA** abre arquivos `.tsx`, `.jsx`, `.vue`, `.svelte`
ou qualquer código-fonte. A leitura é responsabilidade **exclusiva** do
binário `oxlint` rodando dentro do sandbox.

### PT-SIDECAR-2: IPC Zero-Garbage (Herdado)

**Lei Dura:** O output do `oxlint` trafega **exclusivamente** pelo pipe de
`stdout`. O buffer `Vec<u8>` é desserializado diretamente para
`UxContractsPayload` via `serde_json::from_slice`. Zero arquivos temporários.
Zero conversão intermediária para `String`.

### PT-OXC-1: PROIBIDO Emulação DOM (Nova)

| Abordagem SLOP | Risco Letal |
|---|---|
| `puppeteer.launch()` ou `JSDOM.parse()` para extrair props | Boot de navegador headless: 200MB+ de RAM, 2-5s de startup, dependência de Node.js residente |
| `svelte.compile()` para renderizar SSR | Execução de JavaScript arbitrário do repositório → RCE direto |

**Lei Dura:** O `OxcSidecar` extrai contratos de UI **puramente via Análise
Estática da AST**. É TERMINANTEMENTE PROIBIDO:

1. Renderizar componentes (SSR, hydration, virtual DOM).
2. Executar JavaScript/TypeScript do repositório analisado.
3. Instanciar um runtime Node.js ou navegador headless.
4. Importar ou executar `build.rs`, `postinstall` ou qualquer script do repo.

A extração opera em milissegundos sobre a AST estática, não em segundos
sobre o DOM renderizado.

### PT-3: PROIBIDO Bloquear o Event Loop do Tokio (Herdado)

Toda invocação ocorre via `SandboxExecutor::execute()` (async).

---

## 5. Cenário de Falha Isolado

### 5.1. Parse Error em JSX/Svelte Malformado + OOM/Timeout

**Gatilho:** O `oxlint` tenta analisar um arquivo `.tsx` com JSX sintaticamente
inválido (ex: tags não-fechadas, TypeScript experimental não-suportado) ou
um monorepo frontend com milhares de componentes excede o limite de memória.

**Comportamento (Parse Error):**

```
oxlint analisa arquivo com JSX quebrado
  → oxlint emite exit code 0 com output parcial (comportamento de linter)
    → stdout contém JSON com componentes parciais (os que parsaram OK)
    → serde_json::from_slice parse normalmente
    → UxContractsPayload com componentes parciais (não é erro)
```

**Comportamento (OOM/Timeout):**

```
oxlint (PID: 54321) excede limite de memória ou timeout
  → SandboxHandle::execute() detecta Timeout
    → child.kill().await (SIGKILL incondicional)
    → PID removido da guilhotina (D3 do PRD-003)
    → Retorna SandboxError::Timeout
      → OxcSidecar traduz para SidecarError::Timeout { timeout_secs: 90 }
```

**Garantias:**

1. **Zero processos órfãos:** SIGKILL incondicional. Drop mata PIDs ativos.
2. **Zero corrupção:** O `OxcSidecar` não mantém estado interno. Atômica.
3. **Fail-Closed:** Erro propaga-se ao chamador.

### 5.2. JSON Corrompido (Herdado do padrão PRD-006)

```
stdout: Vec<u8> = [bytes corrompidos]
  → serde_json::from_slice::<UxContractsPayload>() → Err
    → SidecarError::ParseError { reason: "..." }
```

---

## 6. Dependências (Cargo.toml)

**Nenhuma dependência nova.** O `OxcSidecar` reutiliza:

| Crate | Uso | Já presente? |
|---|---|---|
| `serde` | Derive `Deserialize` na `UxContractsPayload` | ✅ Sim |
| `serde_json` | `from_slice` para parsing do stdout | ✅ Sim |
| `thiserror` | Reutiliza `SidecarError` existente | ✅ Sim |

---

## 7. Definition of Done (DoD)

A Fase C (TDD) DEVE comprovar mecanicamente os seguintes critérios:

| # | Critério | Teste Correspondente |
|---|---|---|
| 1 | Extração bem-sucedida: executor retorna JSON válido com componentes → `UxContractsPayload` desserializado com props/events | `test_extract_success` |
| 2 | Binary não encontrado: executor retorna erro de spawn → `SidecarError::BinaryNotFound { binary: "oxlint" }` | `test_binary_not_found` |
| 3 | Execução falha (exit code != 0): → `SidecarError::ExecutionFailed` | `test_execution_failed` |
| 4 | Timeout: executor retorna `SandboxError::Timeout` → `SidecarError::Timeout` | `test_timeout_propagation` |
| 5 | JSON corrompido: stdout contém bytes inválidos → `SidecarError::ParseError` | `test_invalid_json` |
| 6 | Repo sem componentes: JSON válido com listas vazias → `UxContractsPayload` com vetores vazios (não é erro) | `test_no_components_valid_json` |
| 7 | A função `extract` é `async` (usa `SandboxExecutor::execute` assíncrono) | Compilação (assinatura `async fn`) |
| 8 | Zero imports de `std::fs` ou `std::process` no módulo | Inspeção estática na Fase D |
| 9 | Zero `unwrap()` na lógica de produção | Inspeção estática na Fase D |

### 7.1. Estratégia de Mock para TDD

Reutiliza **integralmente** o `MockExecutor` já criado nos testes do PRD-006.
Os testes injetam respostas pré-definidas simulando o stdout do `oxlint`.

---

## 8. Interface com o DAG

```
N5 (ExtractionRouter)               N7 (OxcSidecar)
  ├─ route() → [RunOxc] ──────▶      ├─ extract(OxcInput)
  │   (apenas NodeJS/Mixed)           │  → Result<UxContractsPayload, SidecarError>
  │                                    │
N3 (SandboxOrchestrator)              │    [stdout: Vec<u8>]
  ├─ create() → SandboxHandle         │      ↓
  │   (via trait SandboxExecutor) ─▶  │    serde_json::from_slice
  │                                    │      ↓
                                       │    UxContractsPayload { components, files_analyzed }
                                       └──▶ N12 (BlobNormalizer)
```

---

## 9. Decisões Arquiteturais

### 9.1. Co-localização com JCodemunchSidecar

**Decisão:** O `OxcSidecar` reside no **mesmo arquivo** `sidecar.rs`, abaixo
do `JCodemunchSidecar`.

**Justificativa:**

1. **Reutilização da trait:** Ambos usam `SandboxExecutor` e `SidecarError`.
2. **Coesão temática:** Ambos são Sidecars Efêmeros da Camada 2.
3. **Zero duplicação:** `MockExecutor`, `SidecarError`, structs de input
   compartilham o mesmo namespace.

Se o arquivo crescer acima de ~500 linhas, o módulo deve ser fracionado em
`sidecar/jcodemunch.rs` e `sidecar/oxc.rs` com um `sidecar/mod.rs` central.

### 9.2. Extração de Props vs. Linting

O `oxlint` é primariamente um **linter**, não um **extrator de props**.
Na implementação real, o comando CLI pode precisar ser substituído por
uma ferramenta mais especializada (ex: `oxc_parser` direto, ou um wrapper
customizado que extrai `interface Props` da AST).

Para esta Fase 1, o contrato I/O (`UxContractsPayload`) é agnóstico ao
binário utilizado. O PRD define **o que** deve ser extraído; o `command`
passado ao executor pode ser ajustado sem alterar a interface.

### 9.3. `framework` como String vs. Enum

**Decisão:** Manter `framework` como `String` (não enum).

**Justificativa:** Novos frameworks surgem frequentemente (Solid, Qwik, Astro).
Um enum fechado exigiria recompilação a cada adição. Uma `String` com valores
convencionais (`"react"`, `"vue"`, `"svelte"`, `"unknown"`) oferece
extensibilidade sem breaking changes no schema de desserialização.
