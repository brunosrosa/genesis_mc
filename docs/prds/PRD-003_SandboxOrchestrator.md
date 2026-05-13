# PRD-003: SandboxOrchestrator

> **Nó DAG:** N3 (Depende de N2 — BloblessCloner)
> **Módulo Rust:** `sandbox::SandboxOrchestrator`
> **Território:** PRODUTO (Daemon SODA — Rust/Tokio)
> **Classificação:** SEGURANÇA CRÍTICA — Última linha de defesa contra RCE
> **Target FinOps:** `local_slm` (custo ZERO)

---

## 1. Objetivo

Criar um envelope de isolamento nativo do sistema operacional
ao redor do código-fonte clonado (`RepoPath`), **antes** que qualquer
ferramenta de análise (linters, compiladores, extratores AST) toque
nos arquivos do repositório.

O `SandboxOrchestrator` é o **Portão de Segurança** do Harvester.
Ele garante que um `build.rs` malicioso, um `postinstall` script
envenenado, ou qualquer artefato hostil dentro do repositório clonado
**não consiga**:
- Ler ou escrever fora do diretório do clone
- Acessar rede (exceto para Git fetch sob demanda)
- Escalar privilégios no host
- Sobreviver ao ciclo de vida do Handle (RAII com SIGKILL)

### 1.1. Por que Sandboxing Nativo (e não Docker)?

| Abordagem | Boot Time | Overhead de Memória | Complexidade | Viabilidade |
|---|---|---|---|---|
| Docker/Podman | ~500ms-2s | ~50-200MB por container | Daemon externo, imagens, rede virtual | ❌ **BANIDO** pelo Canon V3 |
| Windows Sandbox (WSB) | ~1-3s | ~200MB (Hyper-V VM efêmera) | Requer Windows Pro/Enterprise + Hyper-V | ✅ Tier 1 (quando disponível) |
| AppContainer/LPAC (`rappct`) | <50ms | ~0 (processo enjaulado) | Crate Rust nativa, sem daemon externo | ✅ **Tier 2 / Fallback universal Windows** |
| Landlock LSM | <1ms | ~0 (filtro kernel) | API Rust via `landlock` crate, sem daemon | ✅ **Tier 1 Linux** |

O SODA opera em **Bare-Metal**. Soluções que exigem daemons externos,
imagens de container ou hypervisors pesados estão **terminantemente
proibidas** pelo Canon V3 §1.

---

## 2. Contrato I/O (Régua Atômica)

```
Entrada (I):  repo_path: &RepoPath, policy: SandboxPolicy
Saída   (O):  Result<SandboxHandle, SandboxError>
```

### 2.1. SandboxPolicy (Entrada — Configuração)

Enum que define o nível de isolamento solicitado pelo chamador:

```rust
// Pseudocódigo — não é código final
pub enum SandboxPolicy {
    /// O processo isolado pode APENAS ler os arquivos do RepoPath.
    /// Nenhuma escrita permitida. Nenhum acesso a rede.
    /// Uso: Linters estáticos, extratores AST (N6, N7, N9).
    ReadOnly,

    /// O processo pode ler e escrever dentro do RepoPath.
    /// Nenhum acesso a rede. Nenhum acesso fora do RepoPath.
    /// Uso: Compilação parcial para análise de dependências (N8).
    ReadWrite,
}
```

A `SandboxPolicy` é um **valor semântico** que é traduzido
pelo Orquestrador nas primitivas nativas de cada plataforma:

| Policy | Windows (AppContainer/LPAC) | Linux (Landlock) |
|---|---|---|
| `ReadOnly` | `FILE_GENERIC_READ` sobre RepoPath. Deny-all no resto. | `AccessFs::ReadFile \| ReadDir` sobre RepoPath |
| `ReadWrite` | `FILE_GENERIC_READ \| WRITE` sobre RepoPath | `AccessFs::ReadFile \| WriteFile \| ReadDir \| MakeDir` |

### 2.2. SandboxHandle (Saída de Sucesso)

`SandboxHandle` é um wrapper RAII sobre o ambiente isolado criado:

- Contém a referência ao **processo filho** (ou token AppContainer)
  que representa o sandbox ativo.
- Implementa `Drop` para executar **SIGKILL atômico** (`TerminateProcess`
  no Windows, `kill(pid, SIGKILL)` no Linux) sobre qualquer processo
  filho sobrevivente ao sair de escopo.
- **NÃO** implementa `Clone` nem `Copy` — ownership linear no pipeline.
  O Handle é emprestado ao N9 (`StaticAnalysisSidecar`) e transferido
  ao N13 (`PurgeGuard`) para destruição final.
- Expõe o método `execute(&self, command: &str, args: &[&str]) -> Result<Vec<u8>, SandboxError>`
  para o chamador executar ferramentas **dentro** do sandbox,
  capturando stdout como `Vec<u8>` via IPC Zero-Copy (PT-2).

### 2.3. SandboxError (Saída de Falha)

Enum com variantes estritas:

| Variante | Causa | Ação |
|---|---|---|
| `PrivilegeError { reason: String }` | O SO recusou a criação do sandbox (ex: AppContainer requer permissão, Landlock não suportado pelo kernel) | Fail-Fast (aborta job, registra no SQLite) |
| `PolicyViolation { detail: String }` | O processo tentou acessar recurso fora da policy (rede, disco externo) | Kill processo + registra violação |
| `ProcessSpawnFailed { reason: String }` | Falha ao criar o processo filho dentro do sandbox | Fail-Fast (aborta job) |
| `Timeout` | Processo dentro do sandbox excedeu o tempo limite configurado | Kill processo |
| `UnsupportedPlatform` | Plataforma sem mecanismo de sandboxing disponível (nem WSB, nem LPAC, nem Landlock) | Fail-Fast (aborta lote) |

---

## 3. Cenário de Falha Isolado

> **Régua Atômica:** Uma entrada, uma saída, **UM** cenário principal de falha.

### Cenário: Falha de Alocação de Privilégios Nativos (Fail-Fast)

**Pré-condição:** O `RepoPath` é válido (N2 OK). O código está no Ramdisk.
O sistema está rodando em Windows Home **sem** suporte a Hyper-V (WSB indisponível).
A crate `rappct` tenta criar um AppContainer/LPAC.

**Fluxo:**

1. O `SandboxOrchestrator::create()` recebe `repo_path` e `policy = ReadOnly`.
2. Detecta a edição do Windows via registro (`HKLM\SOFTWARE\Microsoft\Windows NT\CurrentVersion`):
   - Verifica se `EditionID` contém "Home", "Pro" ou "Enterprise".
3. Seleciona a estratégia de sandboxing:
   - Windows Pro/Enterprise → Tenta WSB primeiro, fallback para LPAC.
   - Windows Home → LPAC diretamente (WSB indisponível).
   - Linux → Landlock.
4. Invoca `rappct` para criar o AppContainer com as capabilities mapeadas pela `SandboxPolicy`.
5. O SO **recusa** a criação (ex: política de grupo corporativa bloqueou AppContainers,
   ou a versão do Windows é anterior ao build 17763 que introduziu LPAC).
6. A crate `rappct` retorna erro nativo.
7. O Orquestrador **NÃO** tenta fallback degradado (sem sandbox).
   Retorna `Err(SandboxError::PrivilegeError { reason: "..." })`.
8. O pipeline aborta o job para este repositório.
9. Registra no SQLite: `status = SANDBOX_FAILED`.

**Pós-condição:** O job é ejetado do circuito com log explícito.
O `RepoPath` permanece intacto no Ramdisk (o `PurgeGuard` N13
destruirá tudo no final). Nenhum processo zumbi sobrevive.
**PROIBIDO** executar análise sem sandbox — Zero-Trust absoluto.

---

## 4. Proibições Tóxicas Injetadas

### PT-SANDBOX-1: PROIBIDO DOCKER, PODMAN OU QUALQUER DAEMON DE CONTAINER ✅

O Canon V3 §1 determina: *"Micro-VMs pesadas estão banidas."*
O `SandboxOrchestrator` utiliza **exclusivamente** primitivas nativas
do kernel do sistema operacional:

- **Windows:** AppContainer/LPAC via `rappct` (processo enjaulado, <50ms de boot)
- **Linux:** Landlock LSM via `landlock` crate (filtro kernel, <1ms)
- **Fallback Windows Pro:** Windows Sandbox (WSB) via Hyper-V
  (VM efêmera, aceito como exceção por ser nativo do Windows)

**Anti-Padrão Proibido:**
```rust
// ❌ PROIBIDO: Docker como sandbox
std::process::Command::new("docker").arg("run").arg("--rm")...
```

### PT-SANDBOX-2: PROIBIDO EXECUTAR ANÁLISE SEM SANDBOX (ZERO-TRUST) ✅

Se o `SandboxOrchestrator` falhar em criar o sandbox, o pipeline
**DEVE** abortar o job. É **terminantemente proibido** executar
linters ou compiladores sobre código de terceiros **fora** do sandbox
como "fallback degradado".

Um repositório não-sandboxado é um vetor de RCE.

**Anti-Padrão Proibido:**
```rust
// ❌ PROIBIDO: Fallback sem sandbox
match SandboxOrchestrator::create(&repo, policy).await {
    Ok(handle) => run_analysis(handle),
    Err(_) => run_analysis_without_sandbox(&repo), // NUNCA
}
```

### PT-3: PROIBIDO BLOQUEAR O EVENT LOOP DO TOKIO ✅

Toda criação de sandbox, execução de processos isolados e destruição
de handles DEVE usar `tokio::process::Command` para processos e
`tokio::task::spawn_blocking` para chamadas de sistema síncronas
(como a API do registro do Windows ou chamadas nativas do `rappct`).

---

## 5. RAII e o Padrão da Guilhotina Atômica

O `SandboxHandle` implementa o `Drop` trait com a seguinte garantia:

### 5.1. Invariante de Destruição

> *"Não existe cenário onde um processo de sandbox sobrevive
> ao fim do escopo do Handle."*

Quando o `SandboxHandle` é dropado:

1. **Windows (AppContainer):** Enumera todos os processos filhos
   via `EnumProcesses` filtrando pelo token AppContainer.
   Executa `TerminateProcess` com exit code 1 em cada um.
   Destrói o perfil AppContainer via `rappct`.

2. **Linux (Landlock):** Executa `kill(child_pid, SIGKILL)` seguido
   de `waitpid` para colher o zumbi. O filtro Landlock é
   automaticamente destruído com o processo.

3. **Cleanup:** O `Drop` executa em `std::thread::spawn` + `join()`
   (mesmo padrão validado na Auditoria D1 do PRD-001) para não
   bloquear o Event Loop do Tokio mas garantir que a destruição
   complete antes de retornar.

### 5.2. Process Pool Guard (Guilhotina de Memória)

Cada processo filho criado dentro do sandbox possui um **limite
de memória** configurado via:
- Windows: Job Objects (`SetInformationJobObject` com `JOB_OBJECT_LIMIT_PROCESS_MEMORY`)
- Linux: cgroups v2 (`memory.max`)

Se o processo exceder o limite (ex: linter malicioso tentando
alocar GBs), o SO o mata automaticamente. O `SandboxHandle`
detecta o exit code anômalo e reporta `SandboxError::PolicyViolation`.

---

## 6. Detecção de Plataforma e Estratégia de Seleção

```
Boot do Daemon SODA:
│
├── Windows?
│   ├── Pro/Enterprise?
│   │   ├── Hyper-V habilitado? → WSB (Tier 1)
│   │   └── Hyper-V desabilitado → LPAC via rappct (Tier 2)
│   └── Home?
│       └── LPAC via rappct (Tier 2)
│
├── Linux?
│   └── Kernel >= 5.13? → Landlock (Tier 1)
│       └── Kernel < 5.13 → UnsupportedPlatform (Fail-Fast)
│
└── Outro SO? → UnsupportedPlatform (Fail-Fast)
```

A detecção ocorre **uma única vez** no boot do Daemon, e o resultado
é armazenado em uma `OnceLock<SandboxStrategy>` estática para
reutilização sem overhead em cada job.

---

## 7. Interação com o DAG

| Nó Consumidor | Como usa o SandboxHandle |
|---|---|
| **N9 (StaticAnalysisSidecar)** | Recebe `&SandboxHandle` por empréstimo imutável. Executa `clippy` e `semgrep` **dentro** do sandbox via `handle.execute()`. |
| **N13 (PurgeGuard)** | Recebe ownership do `SandboxHandle` (move). Executa `Drop` para destruição final. |

O `SandboxHandle` é criado em N3, emprestado a N9, e finalmente
consumido (movido) para N13 que o destrói junto com o `RamdiskHandle`.

---

## 8. Invariantes de Segurança

1. **Zero-Trust Absoluto:** Nenhum código de terceiros executa fora do sandbox.
   Falha em criar sandbox = aborta job. Sem exceções.

2. **Sem Clone/Copy em SandboxHandle:** Ownership linear.
   Apenas uma entidade controla o sandbox em qualquer instante.

3. **SIGKILL Incondicional no Drop:** O `Drop` do `SandboxHandle`
   mata todos os processos filhos com sinal irrecusável (SIGKILL/TerminateProcess).
   Processos zumbis estão banidos.

4. **Limite de Memória por Processo:** Cada processo sandboxado
   tem ceiling de RAM via Job Objects (Windows) ou cgroups v2 (Linux).
   Estouro = kill automático pelo SO.

5. **Sem Acesso a Rede:** A `SandboxPolicy::ReadOnly` e `ReadWrite`
   não concedem acesso a rede. Processos isolados não podem exfiltrar dados.

6. **Sem Escrita Fora do RepoPath:** O filesystem visível ao processo
   sandboxado é restrito ao `RepoPath` (e somente se a policy permitir escrita).

---

## 9. Definition of Done (DoD) para Fase C

- [ ] Teste `test_create_sandbox_success` — cria sandbox com policy `ReadOnly` sobre um mock RepoPath, verifica que o `SandboxHandle` é retornado
- [ ] Teste `test_execute_in_sandbox` — executa um comando trivial (`echo "SODA"`) dentro do sandbox e captura stdout como `Vec<u8>`
- [ ] Teste `test_drop_kills_processes` — verifica que processos filhos são terminados ao dropar o `SandboxHandle`
- [ ] Teste `test_no_fallback_without_sandbox` — se a criação falhar, retorna `SandboxError::PrivilegeError` (sem fallback degradado)
- [ ] Teste `test_filesystem_isolation` — processo dentro do sandbox não pode ler/escrever fora do `RepoPath`
- [ ] `cargo clippy` sem warnings
- [ ] `cargo test` com exit code 0

---

## 10. Dependências de Crates (Propostas)

| Crate | Propósito | Plataforma | Justificativa |
|---|---|---|---|
| `rappct` | AppContainer/LPAC nativo | Windows | Aprovada pelo Arquiteto na Fase A (LPAC como fallback) |
| `landlock` | Landlock LSM | Linux | Crate oficial do ecossistema Landlock |
| `winreg` | Leitura do registro do Windows | Windows | Detecção de edição (Home/Pro/Enterprise) |
| `tokio` | Processos assíncronos + timeout | Todas | Core runtime do SODA (já presente) |
| `thiserror` | Derivar `SandboxError` | Todas | Padrão idiomático Rust (já presente) |
| `tracing` | Logs estruturados | Todas | Padrão do ecossistema Tokio (já presente) |

---

## 11. Nota sobre Fase C (TDD)

A implementação real do `SandboxOrchestrator` possui uma **complexidade
significativamente maior** que os PRDs anteriores, pois depende de
APIs nativas do SO que variam entre plataformas.

**Estratégia de Mock para testes:**

Os testes unitários na Fase C utilizarão o mesmo padrão de mock
estabelecido no PRD-001 (`is_mock` flag). O sandbox mock:
- Cria um processo filho normal (sem isolamento real)
- Valida o contrato I/O (entradas, saídas, erros)
- Testa o RAII do `Drop` (kill do processo filho)

Os testes de integração com isolamento real (`SODA_REAL_SANDBOX=1`)
são executados apenas em CI/CD ou manualmente pelo Arquiteto.
