# PRD-004: LanguageDetector

> **Nó DAG:** N4 (Depende de N2 — BloblessCloner)
> **Módulo Rust:** `detect::LanguageDetector`
> **Território:** PRODUTO (Daemon SODA — Rust/Tokio)
> **Classificação:** ROTEAMENTO — Primeira bifurcação do pipeline poliglota
> **Target FinOps:** `local_slm` (custo ZERO)

---

## 1. Objetivo

Determinar a **stack tecnológica primária** de um repositório clonado,
de forma **100% determinística**, analisando exclusivamente a presença
de **arquivos de manifesto** na raiz (ou subdiretórios canônicos de
primeiro nível) do `RepoPath`.

O `LanguageDetector` é o **Sensor de Triagem** do Harvester.
Ele classifica o repositório em um `StackProfile` que será consumido
pelo nó N5 (`ExtractionRouter`) para despachar as ferramentas de
extração corretas (N6–N11).

### 1.1. Regra do "Zero Parser" na Detecção

O `LanguageDetector` **NÃO** lê o conteúdo dos arquivos-fonte.
Ele opera exclusivamente sobre **metadados do filesystem**:

- Existência de arquivos de manifesto raiz (`Cargo.toml`, `package.json`, etc.)
- Extensões de arquivos em um scan superficial do primeiro nível

A leitura do conteúdo de qualquer arquivo (parsing de AST, regex sobre
código-fonte, leitura de `import` statements) é **terminantemente proibida**
nesta fase. Essa responsabilidade pertence aos nós N6–N11 da Camada 2.

### 1.2. Complexidade Algorítmica

A detecção opera em **O(1)** sobre o número de arquivos do repositório.
O scan verifica a existência de, no máximo, ~10 manifestos conhecidos
via chamadas `tokio::fs::try_exists()`. Não há caminhamento recursivo
da árvore de diretórios.

---

## 2. Contrato I/O (Régua Atômica)

```
Entrada (I):  repo_path: &RepoPath
Saída   (O):  Result<StackProfile, DetectionError>
```

### 2.1. Entrada

Referência imutável ao `RepoPath` produzido pelo `BloblessCloner` (N2).
O `RepoPath` aponta para o diretório raiz do repositório clonado
dentro do Ramdisk. A existência e validade do diretório são garantidas
pelo tipo `RepoPath` (ownership linear do N2).

### 2.2. StackProfile (Saída de Sucesso)

Enum que classifica a stack tecnológica dominante do repositório:

```rust
// Pseudocódigo — não é código final
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StackProfile {
    /// Cargo.toml presente na raiz.
    Rust,

    /// package.json presente na raiz (Node.js / Frontend JS/TS).
    NodeJS,

    /// go.mod presente na raiz.
    Go,

    /// requirements.txt, setup.py, pyproject.toml ou Pipfile na raiz.
    Python,

    /// pom.xml ou build.gradle(.kts) na raiz.
    JVM,

    /// *.sln ou *.csproj na raiz.
    DotNet,

    /// Múltiplos manifestos de linguagens diferentes detectados
    /// simultaneamente (ex: Cargo.toml + package.json = monorepo híbrido).
    /// Contém o vetor das linguagens individuais detectadas.
    Mixed(Vec<SingleStack>),

    /// Nenhum manifesto reconhecido. Repositório genérico.
    /// O ExtractionRouter (N5) decidirá se aplica heurística
    /// de extensão ou aborta a análise.
    Unknown,
}

/// Enum auxiliar para compor o `Mixed` sem recursão.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SingleStack {
    Rust,
    NodeJS,
    Go,
    Python,
    JVM,
    DotNet,
}
```

**Regras de classificação:**

| Prioridade | Manifesto(s) Detectado(s) | StackProfile Resultante |
|---|---|---|
| 1 | Apenas `Cargo.toml` | `Rust` |
| 2 | Apenas `package.json` | `NodeJS` |
| 3 | Apenas `go.mod` | `Go` |
| 4 | Apenas `requirements.txt` OU `pyproject.toml` OU `setup.py` OU `Pipfile` | `Python` |
| 5 | Apenas `pom.xml` OU `build.gradle` OU `build.gradle.kts` | `JVM` |
| 6 | Apenas `*.sln` OU `*.csproj` (qualquer arquivo com essas extensões na raiz) | `DotNet` |
| 7 | Dois ou mais manifestos de linguagens **diferentes** | `Mixed(vec![...])` |
| 8 | Nenhum dos acima | `Unknown` |

**Decisão de design — sem precedência entre linguagens:**
Quando múltiplos manifestos de linguagens diferentes coexistem,
o detector **não assume** qual é a primária. Ele retorna `Mixed`
com todas as linguagens detectadas. O `ExtractionRouter` (N5)
resolve a ambiguidade aplicando regras de prioridade baseadas
em linhas de código ou estrutura de diretórios.

### 2.3. DetectionError (Saída de Falha)

Enum com variantes estritas:

| Variante | Causa | Ação |
|---|---|---|
| `EmptyRepository { path: PathBuf }` | O `RepoPath` existe mas está vazio (0 entradas no diretório raiz). Possível clone blobless sem checkout. | Fail-Fast: aborta job, registra no SQLite. |
| `FilesystemError { reason: String }` | Erro de I/O ao acessar o Ramdisk (ex: ramdisk desmontado prematuramente, permissão negada). | Fail-Fast: aborta job. Possível corrupção do Ramdisk — o PurgeGuard (N13) será acionado. |

**Nota:** Não existe variante `UnsupportedLanguage`.
Repositórios sem manifestos reconhecidos retornam `Ok(StackProfile::Unknown)`,
não `Err(...)`. A ausência de manifesto é um **resultado válido**,
não um erro. O `ExtractionRouter` (N5) decide o que fazer com `Unknown`.

---

## 3. Cenário de Falha Isolado

> **Régua Atômica:** Uma entrada, uma saída, **UM** cenário principal de falha.

### Cenário: Repositório Vazio ou Ramdisk Corrompido (Fail-Fast)

**Pré-condição:** O `RepoPath` é válido por tipo (produzido pelo N2).
O clone blobless foi executado com `--filter=blob:none`, o que significa
que apenas a árvore de diretórios e o histórico mínimo estão presentes.
Em alguns repositórios, se o `--single-branch` ou um filtro agressivo
foi aplicado, o checkout pode não materializar nenhum arquivo.

**Fluxo:**

1. O `LanguageDetector::detect()` recebe `repo_path`.
2. Executa `tokio::fs::read_dir(repo_path).await` para listar
   as entradas do diretório raiz.
3. O `read_dir` retorna um stream vazio (0 entradas).
4. O detector identifica a condição `EmptyRepository`.
5. Retorna `Err(DetectionError::EmptyRepository { path: repo_path.to_path_buf() })`.
6. O pipeline aborta o job para este repositório.
7. Registra no SQLite: `status = EMPTY_REPO`.

**Cenário alternativo (Ramdisk corrompido):**

1. O `read_dir` retorna `Err(std::io::Error)` — permissão negada,
   dispositivo não encontrado, etc.
2. O detector encapsula o erro em `DetectionError::FilesystemError`.
3. Retorna `Err(DetectionError::FilesystemError { reason: "..." })`.
4. O pipeline interpreta como corrupção do Ramdisk e aciona o
   `PurgeGuard` (N13) para limpeza de emergência.

**Pós-condição:** O job é ejetado do pipeline com log explícito.
O `RepoPath` permanece intacto no Ramdisk (o PurgeGuard limpa tudo
no final). Nenhuma ferramenta de extração é invocada sobre um
repositório vazio ou corrompido.

---

## 4. Proibições Tóxicas Injetadas

### PT-3: PROIBIDO BLOQUEAR O EVENT LOOP DO TOKIO ✅

Toda operação de I/O sobre o filesystem do Ramdisk **DEVE** usar
a API assíncrona `tokio::fs`:

- `tokio::fs::try_exists()` para verificar manifestos
- `tokio::fs::read_dir()` para listar entradas do diretório raiz

**Anti-Padrão Proibido:**
```rust
// ❌ PROIBIDO: I/O síncrono na thread async
use std::fs;
if fs::metadata(repo_path.join("Cargo.toml")).is_ok() { ... }
```

**Padrão Obrigatório:**
```rust
// ✅ CORRETO: I/O assíncrono via tokio::fs
if tokio::fs::try_exists(repo_path.join("Cargo.toml")).await? { ... }
```

### PT-DETECT-1: PROIBIDO PARSER DE CONTEÚDO (ZERO-READ) ✅

O detector verifica **exclusivamente** a existência de arquivos
de manifesto. Ele **NÃO** abre, lê ou faz parse do conteúdo
de nenhum arquivo do repositório.

A leitura do conteúdo é responsabilidade dos extratores da Camada 2
(N6–N11), que operam dentro do Sandbox (N3).

**Motivação:**

1. **Segurança:** Ler conteúdo de arquivos de um repositório não-auditado
   **fora** do Sandbox é um vetor de ataque (ex: `Cargo.toml` com
   payload malicioso em campo `build`).
2. **Performance:** A detecção deve ser instantânea. Ler conteúdo
   de arquivos adiciona latência desnecessária.
3. **Separação de responsabilidades:** O detector classifica;
   os extratores (Camada 2) analisam.

**Anti-Padrão Proibido:**
```rust
// ❌ PROIBIDO: Ler conteúdo de arquivo no detector
let content = tokio::fs::read_to_string(repo_path.join("Cargo.toml")).await?;
if content.contains("[dependencies]") { ... }
```

### PT-DETECT-2: PROIBIDO CAMINHAMENTO RECURSIVO DA ÁRVORE ✅

O detector opera **exclusivamente** sobre o diretório raiz do
`RepoPath`. Não há `walkdir`, `glob` recursivo, ou iteração
sobre subdiretórios.

**Exceção controlada para DotNet:** A detecção de `*.sln` e `*.csproj`
pode listar entradas do diretório raiz e verificar extensões,
mas **NÃO** desce em subdiretórios.

**Anti-Padrão Proibido:**
```rust
// ❌ PROIBIDO: Caminhamento recursivo
for entry in walkdir::WalkDir::new(repo_path) { ... }
```

---

## 5. Algoritmo de Detecção (Pseudocódigo)

```
fn detect(repo_path: &RepoPath) -> Result<StackProfile, DetectionError>:
    // 1. Validação: diretório não-vazio
    entries = tokio::fs::read_dir(repo_path).await?
    if entries.is_empty():
        return Err(EmptyRepository)

    // 2. Probe de manifestos (O(1) — máximo ~10 chamadas try_exists)
    detected = []

    if exists("Cargo.toml"):    detected.push(Rust)
    if exists("package.json"):  detected.push(NodeJS)
    if exists("go.mod"):        detected.push(Go)
    if exists("requirements.txt") OR exists("pyproject.toml")
       OR exists("setup.py") OR exists("Pipfile"):
                                detected.push(Python)
    if exists("pom.xml") OR exists("build.gradle")
       OR exists("build.gradle.kts"):
                                detected.push(JVM)
    if any_entry_matches("*.sln") OR any_entry_matches("*.csproj"):
                                detected.push(DotNet)

    // 3. Classificação
    match detected.len():
        0 => Ok(Unknown)
        1 => Ok(detected[0].into_stack_profile())
        _ => Ok(Mixed(detected))
```

**Nota sobre `any_entry_matches` para DotNet:**
Como `.sln` e `.csproj` não têm nomes fixos, o detector
reutiliza o stream de `read_dir` já obtido no passo 1
para verificar extensões. Isso não viola PT-DETECT-2
porque opera apenas no diretório raiz, sem recursão.

---

## 6. Interação com o DAG

```
N2 (BloblessCloner)
  │
  ├──► N3 (SandboxOrchestrator)   [paralelo]
  │
  └──► N4 (LanguageDetector) ────► N5 (ExtractionRouter)
           ▲                              │
           │                              ├──► N6 (JCodemunch)
           &RepoPath                      ├──► N7 (Oxc)
                                          ├──► N8 (Manifest)
                                          ├──► N9 (StaticAnalysis)
                                          └──► N11 (OpsBlueprint)
```

O `LanguageDetector` (N4) é **paralelo** ao `SandboxOrchestrator` (N3).
Ambos consomem `&RepoPath` de forma imutável e podem executar
simultaneamente no pipeline `tokio::join!`.

O resultado do N4 (`StackProfile`) alimenta o N5 (`ExtractionRouter`),
que decide quais extratores da Camada 2 serão acionados.

---

## 7. Invariantes

1. **O(1) sobre o número de arquivos:** O detector executa, no máximo,
   ~10 chamadas `try_exists` + 1 `read_dir` (para validação de vazio
   e detecção DotNet). Nenhum caminhamento recursivo.

2. **Zero leitura de conteúdo:** O detector jamais abre ou lê
   o conteúdo de nenhum arquivo. Opera exclusivamente sobre metadados
   do filesystem (existência e extensão).

3. **Sem Clone/Copy em RepoPath:** O detector recebe `&RepoPath`
   por empréstimo imutável. Não toma ownership.

4. **Determinismo puro:** Para o mesmo conjunto de arquivos no diretório
   raiz, o detector sempre retorna o mesmo `StackProfile`.
   Sem randomização, sem heurísticas probabilísticas, sem IA.

5. **`Unknown` é resultado, não erro:** Repositórios sem manifestos
   conhecidos são classificados como `Unknown`, permitindo que o
   `ExtractionRouter` (N5) decida o tratamento downstream.

---

## 8. Definition of Done (DoD) para Fase C

- [ ] Teste `test_detect_rust` — `Cargo.toml` presente → `StackProfile::Rust`
- [ ] Teste `test_detect_nodejs` — `package.json` presente → `StackProfile::NodeJS`
- [ ] Teste `test_detect_python` — `pyproject.toml` presente → `StackProfile::Python`
- [ ] Teste `test_detect_mixed` — `Cargo.toml` + `package.json` → `StackProfile::Mixed(vec![Rust, NodeJS])`
- [ ] Teste `test_detect_unknown` — diretório sem manifestos → `StackProfile::Unknown`
- [ ] Teste `test_detect_empty_repo` — diretório vazio → `Err(DetectionError::EmptyRepository)`
- [ ] Teste `test_no_recursive_walk` — garantir que subdiretórios não são inspecionados (diretório raiz com `subdir/Cargo.toml` retorna `Unknown`, não `Rust`)
- [ ] `cargo clippy` sem warnings
- [ ] `cargo test` com exit code 0

---

## 9. Dependências de Crates (Propostas)

| Crate | Propósito | Justificativa |
|---|---|---|
| `tokio` | `tokio::fs::try_exists`, `tokio::fs::read_dir` | Core runtime do SODA (já presente) |
| `thiserror` | Derivar `DetectionError` | Padrão idiomático Rust (já presente) |

**Nota:** Este módulo **não adiciona** nenhuma dependência nova ao `Cargo.toml`.
Opera exclusivamente com o `tokio` e `thiserror` já presentes.
