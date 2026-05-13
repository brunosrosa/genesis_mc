# PRD-005: ExtractionRouter

> **Nó DAG:** N5 (Depende de N4 — LanguageDetector)
> **Módulo Rust:** `router::ExtractionRouter`
> **Território:** PRODUTO (Daemon SODA — Rust/Tokio)
> **Classificação:** ROTEAMENTO — Máquina de Estados determinística
> **Target FinOps:** `local_slm` (custo ZERO)

---

## 1. Objetivo

Traduzir o `StackProfile` retornado pelo `LanguageDetector` (N4) em uma **lista
ordenada de intenções de extração** (`Vec<ExtractionTask>`), determinando
**quais ferramentas** devem ser disparadas para dissecar aquele repositório
específico.

O `ExtractionRouter` é a **Torre de Controle** do pipeline. Ele **NÃO executa**
nenhuma ferramenta. Ele produz um plano de voo (o vetor de tarefas) que os
nós N6–N11 da Camada 2 consumirão para executar a extração real.

### 1.1. Princípio da Separação Comando/Execução

O N5 aplica a doutrina **CQRS (Command-Query Responsibility Segregation)**
ao pipeline:

- **N5 (Comando):** Decide O QUE extrair. Resultado: `Vec<ExtractionTask>`.
- **N6–N11 (Execução):** Executam COMO extrair. Consomem as tasks.

Esta separação impede que a lógica de decisão se emaranhe com a lógica de
I/O, tornando o roteamento 100% testável por Pattern Matching puro.

---

## 2. Contrato I/O (Régua Atômica)

### 2.1. Entrada

```rust
pub struct ExtractionInput<'a> {
    pub profile: StackProfile,
    pub repo_path: &'a RepoPath,
}
```

| Campo | Tipo | Origem | Semântica |
|---|---|---|---|
| `profile` | `StackProfile` | N4 (`LanguageDetector::detect`) | Enum determinístico da stack tecnológica detectada |
| `repo_path` | `&RepoPath` | N2 (`BloblessCloner::clone`) | Referência imutável ao diretório clonado no Ramdisk |

### 2.2. Saída

```rust
pub fn route(input: ExtractionInput<'_>) -> Vec<ExtractionTask>
```

Retorna um `Vec<ExtractionTask>` **não-vazio** e **ordenado** por prioridade
de execução. A função é **pura** (sem efeitos colaterais, sem I/O, sem async).

### 2.3. ExtractionTask (Enum de Intenções)

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExtractionTask {
    /// N6: Extrair outline AST + grafo de dependências via jcodemunch
    RunJCodemunch,

    /// N7: Extrair contratos UX (Props/Events de componentes frontend)
    RunOxc,

    /// N8: Extrair manifesto de dependências (Cargo.toml, package.json, etc.)
    ExtractManifests,

    /// N9: Rodar análise estática em sandbox (clippy, semgrep, etc.)
    RunStaticAnalysis,

    /// N10: Buscar metadados de comunidade via GitHub API
    FetchCommunityMeta,

    /// N11: Extrair blueprint operacional (CI/CD, Dockerfiles, unsafe blocks)
    ExtractOpsBlueprint,
}
```

---

## 3. Tabela de Roteamento (Máquina de Estados)

A lógica do roteamento é resolvida por um **bloco `match` exaustivo**
sobre o `StackProfile`. Cada variante do enum produz um vetor fixo
e determinístico de `ExtractionTask`.

### 3.1. Trilhas por Stack

| StackProfile | Tarefas Despachadas (em ordem) | Justificativa |
|---|---|---|
| `Rust` | `RunJCodemunch`, `ExtractManifests`, `RunStaticAnalysis`, `FetchCommunityMeta`, `ExtractOpsBlueprint` | AST nativo + clippy no sandbox |
| `NodeJS` | `RunJCodemunch`, `RunOxc`, `ExtractManifests`, `RunStaticAnalysis`, `FetchCommunityMeta`, `ExtractOpsBlueprint` | Inclui extração de contratos UX (componentes React/Vue/Svelte) |
| `Go` | `RunJCodemunch`, `ExtractManifests`, `RunStaticAnalysis`, `FetchCommunityMeta`, `ExtractOpsBlueprint` | go.mod + análise estática |
| `Python` | `RunJCodemunch`, `ExtractManifests`, `RunStaticAnalysis`, `FetchCommunityMeta`, `ExtractOpsBlueprint` | pyproject.toml + semgrep |
| `JVM` | `RunJCodemunch`, `ExtractManifests`, `RunStaticAnalysis`, `FetchCommunityMeta`, `ExtractOpsBlueprint` | pom.xml/build.gradle + análise estática |
| `DotNet` | `RunJCodemunch`, `ExtractManifests`, `RunStaticAnalysis`, `FetchCommunityMeta`, `ExtractOpsBlueprint` | .sln/.csproj + análise estática |
| `Mixed(stacks)` | **União determinística** das trilhas de cada stack individual, sem duplicatas | Monorepos poliglotas recebem a superposição de todas as ferramentas necessárias |
| `Unknown` | `ExtractManifests`, `FetchCommunityMeta`, `ExtractOpsBlueprint` | **Fallback degradado**: sem AST, sem linting. Apenas metadados genéricos. |

### 3.2. Regras de Deduplicação para `Mixed`

Quando o `StackProfile` é `Mixed(stacks)`, o router:

1. Itera sobre cada `SingleStack` contido no vetor.
2. Coleta as tarefas de cada stack individual em um `IndexSet` (ordem de inserção preservada, sem duplicatas).
3. Retorna o `IndexSet` convertido em `Vec<ExtractionTask>`.

> **Nota:** O `IndexSet` é usado conceitualmente. Na implementação, um `Vec`
> com verificação `.contains()` antes da inserção é suficiente (o vetor
> máximo tem 6 elementos — custo O(1) na prática).

### 3.3. Invariante: Vetor Nunca Vazio

O retorno de `route()` é **sempre não-vazio**. Mesmo para `Unknown`,
pelo menos 3 tarefas genéricas são despachadas. Isso garante que o
pipeline nunca "engole" um repositório silenciosamente.

---

## 4. Proibições Tóxicas

### PT-ROUTE-1: PROIBIDO Invocar IA/LLM no Roteamento

| Abordagem SLOP | Risco Letal |
|---|---|
| "Perguntar ao LLM qual ferramenta usar" | Indeterminismo, latência, custo FinOps, alucinação de ferramentas inexistentes |

**Lei Dura:** O roteamento é resolvido **exclusivamente** por Pattern Matching
(bloco `match` em Rust). A decisão é mecânica, determinística e O(1).
Não existem prompts, embeddings, classificadores ou qualquer forma de
inferência probabilística neste nó.

### PT-ROUTE-2: PROIBIDO Executar Extratores Neste Nó

| Abordagem SLOP | Risco Letal |
|---|---|
| "Já que temos o RepoPath, vamos rodar o jcodemunch aqui mesmo" | Violação de CQRS, acoplamento fatal, impossibilidade de testar o roteamento isoladamente |

**Lei Dura:** O `ExtractionRouter` **APENAS** retorna `Vec<ExtractionTask>`.
Ele não spawna processos, não faz I/O de arquivos, não abre sockets.
A função `route()` é **pura e síncrona** — não precisa sequer ser `async`.

### PT-ROUTE-3: PROIBIDO Produzir Vetor Vazio

| Abordagem SLOP | Risco Letal |
|---|---|
| Retornar `vec![]` para stacks desconhecidas | Repositório é silenciosamente ignorado pelo pipeline, dados perdidos sem rastro |

**Lei Dura:** Todo `StackProfile`, incluindo `Unknown`, DEVE produzir
pelo menos um `ExtractionTask`. O fallback mínimo é
`[ExtractManifests, FetchCommunityMeta, ExtractOpsBlueprint]`.

---

## 5. Cenário de Falha Isolado

### 5.1. StackProfile::Unknown — Degradação Controlada

**Gatilho:** O `LanguageDetector` (N4) não encontrou nenhum arquivo de manifesto
reconhecido na raiz do repositório.

**Comportamento:**

```
StackProfile::Unknown → [ExtractManifests, FetchCommunityMeta, ExtractOpsBlueprint]
```

O router **NÃO aborta**. Ele despacha um conjunto mínimo de tarefas genéricas
que não dependem de conhecimento da linguagem:

- `ExtractManifests`: Tenta extrair qualquer manifesto que o N8 consiga parsear.
- `FetchCommunityMeta`: Busca metadados de comunidade via GitHub API (independente de linguagem).
- `ExtractOpsBlueprint`: Grep por CI/CD, Dockerfiles e padrões operacionais.

**Justificativa:** `Unknown` é um **resultado legítimo**, não um erro.
Repositórios sem linguagem dominante (ex: repos de documentação, datasets,
configurações) ainda possuem valor de metadados para o SODA.

### 5.2. Nota sobre Infabilidade

A função `route()` é **infálivel** — ela não retorna `Result`.
Não existe cenário de erro possível:

- A entrada é um `StackProfile` (enum finito, exaustivamente mapeado).
- A saída é um `Vec` (alocação na stack/heap, sem I/O falível).
- O `match` é exaustivo sobre todas as variantes.

Se o compilador Rust compilar o código, a correção total é **garantida
matematicamente** pelo type system.

---

## 6. Dependências (Cargo.toml)

**Zero dependências novas.** O `ExtractionRouter` opera com tipos
primitivos (`Vec`, `enum`) e Pattern Matching puro. Não requer
crates adicionais.

---

## 7. Definition of Done (DoD)

A Fase C (TDD) DEVE comprovar mecanicamente os seguintes critérios:

| # | Critério | Teste Correspondente |
|---|---|---|
| 1 | `Rust` despacha 5 tarefas na ordem correta | `test_route_rust` |
| 2 | `NodeJS` despacha 6 tarefas (inclui `RunOxc`) | `test_route_nodejs` |
| 3 | `Go` despacha 5 tarefas | `test_route_go` |
| 4 | `Python` despacha 5 tarefas | `test_route_python` |
| 5 | `JVM` despacha 5 tarefas | `test_route_jvm` |
| 6 | `DotNet` despacha 5 tarefas | `test_route_dotnet` |
| 7 | `Unknown` despacha exatamente 3 tarefas (fallback) | `test_route_unknown_fallback` |
| 8 | `Mixed([Rust, NodeJS])` despacha a **união sem duplicatas** (6 tarefas, com `RunOxc`) | `test_route_mixed_dedup` |
| 9 | `Mixed([Go, Python])` despacha a **união sem duplicatas** (5 tarefas) | `test_route_mixed_no_frontend` |
| 10 | Vetor de retorno é **sempre não-vazio** para qualquer input | `test_route_never_empty` |
| 11 | A função `route()` é **síncrona** (não é `async`) | Compilação (tipo de retorno sem `impl Future`) |
| 12 | A função `route()` é **pura** (sem I/O, sem efeitos colaterais) | Inspeção estática na Fase D |

---

## 8. Interface com o DAG

```
N4 (LanguageDetector)                  N5 (ExtractionRouter)
  ├─ detect(&RepoPath)  ──────────▶   ├─ route(ExtractionInput)
  │  → StackProfile                   │  → Vec<ExtractionTask>
  │                                    │
  │                                    ├──▶ N6: RunJCodemunch
  │                                    ├──▶ N7: RunOxc
  │                                    ├──▶ N8: ExtractManifests
  │                                    ├──▶ N9: RunStaticAnalysis
  │                                    ├──▶ N10: FetchCommunityMeta
  │                                    └──▶ N11: ExtractOpsBlueprint
```

---

## 9. Decisões Arquiteturais

### 9.1. Função Pura vs. Async

O `route()` é deliberadamente **síncrono e puro**. Razões:

1. Não faz I/O (não precisa de `tokio`).
2. Testabilidade máxima (sem runtime async nos testes).
3. Custo de invocação: O(1) — um `match` com no máximo 8 branches.

### 9.2. `ExtractionTask` como Enum, Não Trait Object

As tarefas são representadas como um **enum plano**, não como `Box<dyn Task>`.
Razões:

1. Zero alocação heap para o roteamento.
2. Exaustividade garantida pelo compilador em futuros `match`.
3. Serialização trivial para telemetria/logging.

### 9.3. Ownership do `repo_path` na ExtractionInput

O `repo_path` é passado como `&RepoPath` (empréstimo). O router **não** consome
nem clona o path. Os nós N6–N11 receberão o `RepoPath` por referência diretamente
do orquestrador, não via o output do router.

O `repo_path` é incluído na `ExtractionInput` para permitir futuras heurísticas
de roteamento baseadas na estrutura do diretório (ex: presença de `src/frontend/`
para decidir `RunOxc`), sem alterar a assinatura da API.
