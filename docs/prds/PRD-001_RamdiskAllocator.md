# PRD-001: RamdiskAllocator

> **Nó DAG:** N1 (Raiz — Dependência Zero)
> **Módulo Rust:** `ramdisk::RamdiskAllocator`
> **Território:** PRODUTO (Daemon SODA — Rust/Tokio)
> **Target FinOps:** `local_slm` (custo ZERO)

---

## 1. Objetivo

Prover a alocação e desmontagem determinística de um disco RAM efêmero
que sirva como zona de clonagem temporária para repositórios Git.

O Ramdisk existe para **blindar o SSD NVMe** do usuário contra
milhões de operações de escrita aleatória geradas pelo `git clone`
de 370+ repositórios. Toda escrita temporária ocorre na RAM.
Toda desmontagem é atômica via RAII (`Drop` trait).

---

## 2. Contrato I/O (Régua Atômica)

```
Entrada (I):  tamanho_mb: u32 (default: 2048)
Saída   (O):  Result<RamdiskHandle, RamdiskError>
```

### 2.1. RamdiskHandle (Saída de Sucesso)

O `RamdiskHandle` é uma struct que:

- Expõe um campo `path: PathBuf` → o ponto de montagem do Ramdisk
  (ex: `R:\` no Windows, `/mnt/soda_ramdisk` no Linux).
- Implementa `Drop` → ao sair de escopo (ou em `panic!`), executa a
  desmontagem atômica do Ramdisk. **Não existe cenário onde o Ramdisk
  sobrevive ao fim do escopo do Handle.**
- Implementa `AsRef<Path>` → para passagem ergonômica como argumento
  para o `BloblessCloner` (N2) e demais nós downstream.

### 2.2. RamdiskError (Saída de Falha)

Enum com variantes estritas:

| Variante | Causa | Ação |
|---|---|---|
| `InsufficientMemory` | RAM disponível < `tamanho_mb` + margem de 2GB | Fail-Fast (aborta job) |
| `AllocationFailed` | Comando `imdisk`/`mount` retornou exit code ≠ 0 | Fail-Fast (aborta job) |
| `UnsupportedPlatform` | OS não é Windows nem Linux | Fail-Fast (aborta lote) |

---

## 3. Cenário de Falha Isolado

> **Régua Atômica:** Uma entrada, uma saída, **UM** cenário principal de falha.

### Cenário: RAM Insuficiente (Fail-Fast)

**Pré-condição:** O sistema possui 32GB de RAM total, mas 30GB estão
em uso por outros processos e pelo KV Cache da GPU.

**Fluxo:**

1. O `RamdiskAllocator` recebe `tamanho_mb = 2048`.
2. Antes de qualquer alocação, consulta a RAM **disponível** (não total)
   via `sysinfo::System::available_memory()`.
3. Verifica: `ram_disponivel >= tamanho_mb + MARGEM_SEGURANCA_MB` (2GB).
4. `2048 MB disponíveis < 2048 + 2048 = 4096 MB exigidos` → **FALHA**.
5. Retorna `Err(RamdiskError::InsufficientMemory)`.
6. O Orquestrador MPSC recebe o erro, registra no SQLite
   (`status = ERRO_INFRA`) e avança para o próximo job.
7. **Nenhum Ramdisk é alocado. Nenhum byte toca o SSD.**

**Pós-condição:** O job é ejetado do circuito com log explícito.
O Event Loop do Tokio permanece livre.

---

## 4. Proibições Tóxicas Injetadas

### PT-1: PROIBIDO CLONAR NO SSD NVMe ✅ (Aplicação Direta)

Este PRD **é** a materialização da PT-1. O `RamdiskAllocator` existe
exclusivamente para impedir que `git clone` toque o NVMe. Se este
módulo falhar, o job DEVE ser abortado — nunca cair em fallback
para o disco físico.

**Anti-Padrão Proibido:**
```
// ❌ PROIBIDO: Fallback silencioso para o SSD
if ramdisk.is_err() {
    clone_to("./temp/");  // VIOLAÇÃO LETAL DA PT-1
}
```

**Padrão Obrigatório:**
```
// ✅ CORRETO: Fail-Fast explícito
let handle = RamdiskAllocator::new(2048)?;  // Propaga erro
// Se falhar, o '?' ejeta o job inteiro do circuito
```

### PT-3: PROIBIDO BLOQUEAR O EVENT LOOP DO TOKIO ✅

A alocação do Ramdisk envolve invocar processos do sistema operacional
(`imdisk` / `mount`). Estes comandos DEVEM ser executados via
`tokio::process::Command` (assíncrono) ou dentro de
`tokio::task::spawn_blocking`.

**Anti-Padrão Proibido:**
```
// ❌ PROIBIDO: Chamada bloqueante no Tokio
std::process::Command::new("imdisk").arg("-a").output();
```

**Padrão Obrigatório:**
```
// ✅ CORRETO: Assíncrono via Tokio
tokio::process::Command::new("imdisk").arg("-a").output().await;
```

---

## 5. Agnosticismo de Plataforma

| Plataforma | Backend de Alocação | Comando de Montagem | Comando de Desmontagem |
|---|---|---|---|
| **Windows** | `imdisk` (ImDisk Virtual Disk Driver) | `imdisk -a -s <size>M -m <letter>: -p "/fs:ntfs /q /y"` | `imdisk -D -m <letter>:` |
| **Linux** | `tmpfs` (kernel nativo) | `mount -t tmpfs -o size=<size>M soda_ramdisk <path>` | `umount <path>` |

### 5.1. Detecção de Plataforma

A detecção ocorre em **tempo de compilação** via `cfg(target_os)`:

- `#[cfg(target_os = "windows")]` → módulo `ramdisk::windows`
- `#[cfg(target_os = "linux")]` → módulo `ramdisk::linux`
- Qualquer outro OS → `RamdiskError::UnsupportedPlatform`

### 5.2. Dependência Externa (Windows)

O `imdisk` precisa estar instalado no sistema. O `RamdiskAllocator`
DEVE verificar a existência do binário `imdisk.exe` no `PATH`
antes de tentar a alocação. Se ausente:

```
Err(RamdiskError::AllocationFailed {
    reason: "imdisk.exe não encontrado no PATH. Instale o ImDisk Virtual Disk Driver."
})
```

---

## 6. Invariantes de Segurança (RAII)

1. **Drop Incondicional:** O `Drop` do `RamdiskHandle` executa a
   desmontagem **mesmo** em caso de `panic!` na thread. Isto é
   garantido pelo runtime do Rust (stack unwinding).

2. **Sem Clone/Copy:** `RamdiskHandle` NÃO implementa `Clone` nem `Copy`.
   Existe exatamente **uma** referência ao Ramdisk a qualquer momento.
   Ownership é transferida para o `PurgeGuard` (N13) no fim da pipeline.

3. **Sem Fallback para Disco:** Se a alocação falhar, o sistema
   NÃO tenta escrever no SSD. A única resposta válida é `Err`.

4. **Timeout de Segurança:** Se `imdisk`/`mount` não responder em
   10 segundos, o processo filho é terminado via `kill()` e
   `AllocationFailed` é retornado.

---

## 7. Definition of Done (DoD) para Fase C

- [ ] Teste `test_alloc_success` — aloca 64MB de Ramdisk, verifica que `path` existe e é gravável
- [ ] Teste `test_alloc_insufficient_memory` — mock de `sysinfo` retornando 1GB livre → espera `InsufficientMemory`
- [ ] Teste `test_drop_unmounts` — verifica que ao dropar o Handle, o ponto de montagem deixa de existir
- [ ] Teste `test_no_ssd_fallback` — verifica que `AllocationFailed` **não** tenta gravar no disco físico
- [ ] `cargo clippy` sem warnings
- [ ] `cargo test` com exit code 0

---

## 8. Dependências de Crates (Propostas)

| Crate | Propósito | Justificativa |
|---|---|---|
| `sysinfo` | Consultar RAM disponível | Cross-platform, zero unsafe, amplamente auditada |
| `tokio` | `tokio::process::Command` para invocar `imdisk`/`mount` | Core runtime do SODA |
| `thiserror` | Derivar `RamdiskError` com mensagens claras | Padrão idiomático Rust para error types |
| `tracing` | Logs estruturados (alocação/desmontagem) | Padrão do ecossistema Tokio para observabilidade |
