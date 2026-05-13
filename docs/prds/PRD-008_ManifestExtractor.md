# PRD-008: ManifestExtractor

> **Nó DAG:** N8 (Depende de N5 — ExtractionRouter)
> **Módulo Rust:** `extract::ManifestExtractor`
> **Território:** PRODUTO (Daemon SODA — Rust/Tokio)
> **Classificação:** EXTRAÇÃO — Leitura Direta de Manifestos (Camada 2)
> **Target FinOps:** `local_slm` (custo ZERO)

---

## 1. Objetivo

Extrair as dependências declaradas nos arquivos de manifesto de um repositório
clonado no Ramdisk (`Cargo.toml`, `package.json`, `go.mod`, `pyproject.toml`,
`requirements.txt`, `pom.xml`, `build.gradle`, `*.csproj`) e consolidá-las
em uma struct Rust tipada (`ManifestPayload`).

### 1.1. Diferença Fundamental dos Sidecars (N6/N7)

O `ManifestExtractor` **NÃO** é um Sidecar Efêmero. Ele não aciona sandbox,
não spawna processos filhos, e não depende de binários externos.

Ele opera como um **leitor direto de arquivos pequenos** na RAM
(arquivos de manifesto residem no Ramdisk alocado pelo N1). A operação
é intrinsecamente segura:

| Propriedade | Sidecars (N6/N7) | ManifestExtractor (N8) |
|---|---|---|
| Processo externo | ✅ (jcodemunch/oxlint) | ❌ |
| Sandbox necessário | ✅ (LPAC/Landlock) | ❌ |
| Leitura de código-fonte | ❌ (delegada ao sidecar) | ❌ |
| Leitura de manifestos | ❌ | ✅ (arquivos < 1MB, lidos do Ramdisk) |
| I/O assíncrono | `tokio::process` | `tokio::fs` |
| Risco de RCE | Mitigado pelo sandbox | Inexistente (leitura de dados declarativos) |

### 1.2. Condição de Ativação

O `ManifestExtractor` é invocado quando o `ExtractionRouter` (N5)
emite a tarefa `ExtractionTask::ExtractManifests`. Isso ocorre para
**todas** as stacks, inclusive `Unknown` (como parte do fallback mínimo).

---

## 2. Contrato I/O (Régua Atômica)

### 2.1. Entrada

```rust
pub struct ManifestInput<'a> {
    pub repo_path: &'a RepoPath,
}
```

| Campo | Tipo | Origem | Semântica |
|---|---|---|---|
| `repo_path` | `&RepoPath` | N2 (BloblessCloner) | Caminho no Ramdisk onde o repo foi clonado |

> **Nota:** Diferente dos Sidecars, o `ManifestExtractor` precisa do
> `repo_path` diretamente porque ele próprio faz o I/O via `tokio::fs`.
> Não há `SandboxExecutor` intermediário.

### 2.2. Saída

```rust
pub async fn extract(
    input: ManifestInput<'_>,
) -> Result<ManifestPayload, ExtractionError>
```

### 2.3. ManifestPayload (Struct de Saída Tipada)

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ManifestPayload {
    /// Lista de manifestos encontrados na raiz do repositório
    pub manifests: Vec<ManifestInfo>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ManifestInfo {
    /// Nome do arquivo de manifesto (ex: "Cargo.toml", "package.json")
    pub file_name: String,

    /// Dependências de produção extraídas do manifesto
    pub dependencies: Vec<DependencyEntry>,

    /// Dependências de desenvolvimento
    pub dev_dependencies: Vec<DependencyEntry>,

    /// Tamanho em bytes do arquivo original (para telemetria)
    pub file_size_bytes: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DependencyEntry {
    /// Nome do pacote/crate/módulo
    pub name: String,

    /// Versão declarada (semver string ou range, ex: "^1.2.3", ">=0.5")
    pub version_spec: String,
}
```

> **Decisão Arquitetural:** As structs de payload do `ManifestExtractor` NÃO
> derivam `Deserialize` do serde. Diferente dos Sidecars (N6/N7) que
> desserializam o `stdout` de um processo externo, aqui o parsing é feito
> internamente pelo Rust usando crates especializadas (`toml` para TOML,
> `serde_json` para JSON, parsing textual para `requirements.txt`).
> Os dados são construídos campo a campo pelo código Rust.

### 2.4. ExtractionError (Erro Específico do Domínio)

```rust
#[derive(Error, Debug, Clone, PartialEq, Eq)]
pub enum ExtractionError {
    #[error("No manifest files found in repository root")]
    NotFound,

    #[error("Failed to parse manifest '{file}': {reason}")]
    ParseError { file: String, reason: String },

    #[error("Manifest file exceeds size limit ({size_bytes} bytes > {limit_bytes} bytes): {file}")]
    FileTooLarge {
        file: String,
        size_bytes: u64,
        limit_bytes: u64,
    },

    #[error("Filesystem error reading '{file}': {reason}")]
    IoError { file: String, reason: String },
}
```

> **Nota:** Um novo enum `ExtractionError` é criado — **não** se reutiliza
> `SidecarError`. O domínio semântico é diferente: não há binários para
> "not found", não há timeout de processo. Os erros aqui são de I/O de
> arquivo e de parsing de formatos estruturados.

---

## 3. Fluxo de Execução (Sequência Mecânica)

```
ManifestExtractor::extract()
│
├─ 1. Define TARGETS: lista fixa de nomes de manifesto na raiz
│     ["Cargo.toml", "package.json", "go.mod", "pyproject.toml",
│      "requirements.txt", "pom.xml", "build.gradle", "build.gradle.kts"]
│
├─ 2. Para cada TARGET:
│     ├─ 2a. Monta o caminho: repo_path.join(target)
│     ├─ 2b. Verifica existência: tokio::fs::metadata(&path)
│     │       ├─ Err(NotFound) → skip (não é erro)
│     │       └─ Ok(metadata) → prossegue
│     ├─ 2c. Verifica tamanho: metadata.len() > MAX_MANIFEST_SIZE (1MB)
│     │       ├─ true → ExtractionError::FileTooLarge (Fail-Fast)
│     │       └─ false → prossegue
│     ├─ 2d. Lê conteúdo: tokio::fs::read(&path) → Vec<u8>
│     ├─ 2e. Parseia conforme o formato:
│     │       ├─ "Cargo.toml" → parse_toml_deps()
│     │       ├─ "package.json" → parse_json_deps()
│     │       ├─ "go.mod" → parse_gomod_deps()
│     │       ├─ "pyproject.toml" → parse_toml_deps() (estrutura diferente)
│     │       ├─ "requirements.txt" → parse_requirements_txt()
│     │       ├─ "pom.xml" → parse_pom_xml_deps()
│     │       └─ "build.gradle*" → parse_gradle_deps()
│     └─ 2f. Adiciona ManifestInfo ao vetor de resultados
│
├─ 3. Se nenhum manifesto foi encontrado:
│     └─ return Err(ExtractionError::NotFound)
│
└─ 4. Retorna Ok(ManifestPayload { manifests })
```

### 3.1. Constante de Limite Termodinâmico

```rust
/// Tamanho máximo permitido para um arquivo de manifesto (1 MiB).
/// Arquivos acima deste limite são considerados corrompidos ou gerados
/// por ferramentas disfuncionais (ex: lockfiles gigantes confundidos
/// com manifestos). Aborta imediatamente para proteger a RAM.
const MAX_MANIFEST_SIZE: u64 = 1_048_576; // 1 MiB
```

### 3.2. Parsing Simplificado (Fase 1 — Extração Mínima)

Na Fase 1 do SODA, o parsing é **conservador e resiliente**:

- **TOML (Cargo.toml, pyproject.toml):** Usa a crate `toml` para
  desserializar e extrair chaves `[dependencies]` e `[dev-dependencies]`.
- **JSON (package.json):** Usa `serde_json` para extrair
  `dependencies` e `devDependencies`.
- **Texto puro (requirements.txt):** Parsing linha a linha com
  regex mínimo `nome==versão` ou `nome>=versão`.
- **Formatos complexos (pom.xml, build.gradle, go.mod):** Na Fase 1,
  estes formatos retornam listas **vazias** de deps com o campo
  `file_name` preenchido (sinaliza presença do manifesto). O parsing
  profundo de XML/Groovy/Go é adiado para a Fase 2 sem bloquear o pipeline.
- **Erro de parse NÃO é fatal:** Se um manifesto individual falhar o
  parsing, gera `ExtractionError::ParseError` para **aquele** arquivo.
  Os demais manifestos continuam sendo processados normalmente.
  O erro só propaga se **todos** os manifestos falharem.

---

## 4. Proibições Tóxicas

### PT-MANIFEST-1: PROIBIDO Recursão ou Varredura de Código (Nova)

| Abordagem SLOP | Risco Letal |
|---|---|
| `walkdir::WalkDir::new(repo_path)` varrendo todo o repo | Em monorepos com milhares de subdiretórios, o I/O sequencial mataria a performance. Nos repositórios Python, confundiria `requirements.txt` internos de subprojetos com o da raiz |
| Ler e parsear `.rs`/`.ts`/`.py` para inferir deps | Invasão do território do N6 (JCodemunchSidecar). Estouro de escopo e RAM |

**Lei Dura:** O `ManifestExtractor` opera ESTRITAMENTE sobre uma **lista
fixa de nomes de arquivo na raiz** do repositório. É TERMINANTEMENTE PROIBIDO:

1. Caminhar recursivamente em subdiretórios.
2. Usar `walkdir`, `glob`, ou `fs::read_dir` para varrer o diretório.
3. Ler ou parsear código-fonte (`.rs`, `.ts`, `.py`, `.java`).

A abordagem é `try_exists` + `read` direto no caminho pré-definido. `O(k)`
onde `k` = número fixo de alvos (≤ 8).

### PT-MANIFEST-2: Limite Termodinâmico de Tamanho (Nova)

| Abordagem SLOP | Risco Letal |
|---|---|
| `tokio::fs::read_to_string(path)` sem checar o tamanho | Um `package.json` de monorepo corrompido (ou um lockfile renomeado) de 50MB seria alocado integralmente na RAM, causando pico de memória e OOM |

**Lei Dura:** ANTES de ler o conteúdo de qualquer manifesto, o
`ManifestExtractor` DEVE verificar `metadata.len()`. Se o tamanho
exceder `MAX_MANIFEST_SIZE` (1 MiB), a extração para aquele arquivo
é abortada com `ExtractionError::FileTooLarge`. O conteúdo **nunca**
é alocado na RAM.

### PT-3: PROIBIDO Bloquear o Event Loop do Tokio (Herdado)

Toda operação de I/O usa `tokio::fs` (async). É PROIBIDO usar
`std::fs::read`, `std::fs::metadata` ou qualquer operação síncrona.

---

## 5. Cenário de Falha Isolado

### 5.1. Nenhum Manifesto Reconhecido + Arquivo TOML/JSON Corrompido

**Gatilho 1 — Nenhum manifesto encontrado:**

```
ManifestExtractor::extract() itera sobre a lista de TARGETS
  → Nenhum dos 8 arquivos existe na raiz do Ramdisk
    → Vetor de resultados permanece vazio
      → return Err(ExtractionError::NotFound)
```

**Gatilho 2 — TOML/JSON corrompido:**

```
tokio::fs::read("Cargo.toml") → Ok(bytes)
  → toml::from_str() → Err (sintaxe inválida)
    → ExtractionError::ParseError { file: "Cargo.toml", reason: "..." }
```

**Gatilho 3 — Arquivo excede 1MB:**

```
tokio::fs::metadata("package.json") → Ok(meta)
  → meta.len() = 52_428_800 (50MB!) > MAX_MANIFEST_SIZE
    → ExtractionError::FileTooLarge {
          file: "package.json",
          size_bytes: 52_428_800,
          limit_bytes: 1_048_576,
      }
```

**Garantias:**

1. **Zero alocação de arquivos gigantes:** A verificação de tamanho
   ocorre ANTES do `tokio::fs::read`.
2. **Fail-Closed:** Erros propagam-se ao chamador sem derrubar o Tokio.
3. **Zero processos órfãos:** Não há processos filhos — apenas I/O de arquivo.

---

## 6. Dependências (Cargo.toml)

| Crate | Uso | Já presente? |
|---|---|---|
| `toml` | Parsing de `Cargo.toml` e `pyproject.toml` | ❌ **Nova** |
| `serde_json` | Parsing de `package.json` | ✅ Sim |
| `serde` | Derive `Deserialize` para structs TOML intermediárias | ✅ Sim |
| `thiserror` | Derive para `ExtractionError` | ✅ Sim |
| `tokio` (feature `fs`) | `tokio::fs::metadata`, `tokio::fs::read` | ✅ Sim |

> **Nota:** A crate `toml` é a **única dependência nova** e é 100% Rust
> puro, sem ligações C. Peso negligível no binário.

---

## 7. Definition of Done (DoD)

A Fase C (TDD) DEVE comprovar mecanicamente os seguintes critérios:

| # | Critério | Teste Correspondente |
|---|---|---|
| 1 | Extração de `Cargo.toml`: deps e dev-deps parseados corretamente | `test_extract_cargo_toml` |
| 2 | Extração de `package.json`: dependencies e devDependencies | `test_extract_package_json` |
| 3 | Extração de `requirements.txt`: parsing linha a linha | `test_extract_requirements_txt` |
| 4 | Repositório sem manifestos → `ExtractionError::NotFound` | `test_no_manifests` |
| 5 | Manifesto corrompido (TOML inválido) → `ExtractionError::ParseError` | `test_corrupted_manifest` |
| 6 | Manifesto gigante (> 1MB) → `ExtractionError::FileTooLarge` | `test_file_too_large` |
| 7 | Múltiplos manifestos na raiz (ex: `Cargo.toml` + `package.json`) → ambos presentes no payload | `test_multiple_manifests` |
| 8 | Zero `read_dir` / Zero recursão / Zero `walkdir` no módulo | Inspeção estática na Fase D |
| 9 | Zero `unwrap()` na lógica de produção | Inspeção estática na Fase D |
| 10 | Toda operação de I/O é `tokio::fs` (zero `std::fs`) | Inspeção estática na Fase D |

### 7.1. Estratégia de Mock para TDD

Diferente dos Sidecars (N6/N7), o `ManifestExtractor` opera sobre
**arquivos físicos no disco** (Ramdisk). Os testes devem usar
`tempdir` (via `tempfile::TempDir`) para criar diretórios temporários
com manifestos de teste escritos programaticamente.

```rust
// Exemplo de setup de teste:
let dir = tempfile::TempDir::new().unwrap();
std::fs::write(
    dir.path().join("Cargo.toml"),
    r#"[dependencies]\nserde = "1.0"\n"#,
).unwrap();
let repo_path = RepoPath(dir.path().to_path_buf());
let result = ManifestExtractor::extract(ManifestInput { repo_path: &repo_path }).await;
```

---

## 8. Interface com o DAG

```
N5 (ExtractionRouter)              N8 (ManifestExtractor)
  ├─ route() → [ExtractManifests]     ├─ extract(ManifestInput)
  │   (TODAS as stacks)               │  → Result<ManifestPayload, ExtractionError>
  │                                    │
N2 (BloblessCloner)                   │    tokio::fs::metadata → size check
  ├─ clone() → RepoPath ──────────▶  │    tokio::fs::read → parse
  │   (no Ramdisk)                    │      ↓
                                       │    ManifestPayload { manifests: Vec<ManifestInfo> }
                                       └──▶ N12 (BlobNormalizer)
```

---

## 9. Decisões Arquiteturais

### 9.1. Módulo Separado `extract.rs` (Novo)

**Decisão:** O `ManifestExtractor` reside em um **novo arquivo**
`src/harvester/extract.rs`, não em `sidecar.rs`.

**Justificativa:**

1. **Domínio diferente:** Não é um Sidecar. Não usa `SandboxExecutor`.
   Não spawna processos. Misturá-lo com N6/N7 poluiria o namespace.
2. **Erro diferente:** Usa `ExtractionError`, não `SidecarError`.
3. **Escalabilidade:** O N11 (`OpsBlueprintExtractor`) também é um
   extrator direto (não-sidecar). Ambos compartilharão o módulo `extract`.

### 9.2. Parsing Conservador (Fase 1)

**Decisão:** Na Fase 1, `pom.xml`, `build.gradle` e `go.mod` retornam
listas vazias de deps, apenas sinalizando a presença do manifesto.

**Justificativa:**

1. **pom.xml:** Requer parser XML com herança de parent POM — complexo.
2. **build.gradle:** DSL Groovy/Kotlin — parsing estático não-trivial.
3. **go.mod:** Formato simples mas com `replace` directives.

O schema `ManifestInfo` acomoda estes formatos com `dependencies: vec![]`
e `dev_dependencies: vec![]`. O pipeline não perde dados — a **presença**
do manifesto é registrada, e o parsing profundo será feito na Fase 2.

### 9.3. Tratamento Parcial de Erros

**Decisão:** Se um manifesto individual falhar o parse, os demais
continuam sendo processados. O erro só é fatal se **nenhum**
manifesto for extraído com sucesso.

**Justificativa:** Em repositórios `Mixed`, é aceitável que o
`pyproject.toml` falhe o parse mas o `package.json` retorne dados
válidos. O pipeline deve ser resiliente a manifestos parcialmente
corrompidos sem abortar toda a extração.
