# PRD-002: BloblessCloner

> **Nó DAG:** N2 (Depende de N1 — RamdiskAllocator)
> **Módulo Rust:** `git::BloblessCloner`
> **Território:** PRODUTO (Daemon SODA — Rust/Tokio)
> **Target FinOps:** `local_slm` (custo ZERO)

---

## 1. Objetivo

Executar o `git clone --filter=blob:none` de um repositório remoto
**exclusivamente** dentro do Ramdisk alocado pelo PRD-001,
produzindo um clone parcial (blobless) que contém toda a árvore
de diretórios, todo o histórico de commits e todos os metadados
Git — mas **nenhum** conteúdo de arquivo (blobs) até que sejam
explicitamente requisitados.

### 1.1. Racionalidade do `--filter=blob:none`

| Estratégia | Dados Baixados | Uso de I/O no Ramdisk | Viabilidade |
|---|---|---|---|
| `git clone` (completo) | 100% dos blobs + histórico | Dezenas de GB para repos grandes | ❌ Estoura o Ramdisk de 2GB |
| `git clone --depth=1` | Último snapshot apenas | Leve, mas perde histórico e metadados | ❌ Insuficiente para análise AST |
| `git clone --filter=blob:none` | Histórico + árvore, sem blobs | ~5-50MB típico para repos médios | ✅ **Padrão SODA** |

O clone blobless preserva o grafo de commits completo (necessário para
o `CommunityMetaFetcher` N10 e análise de frequência de contribuição)
sem materializar os arquivos pesados na RAM. Os blobs individuais
serão buscados sob demanda pelo `JCodemunchSidecar` (N6) apenas para
os arquivos que a AST precisa analisar.

---

## 2. Contrato I/O (Régua Atômica)

```
Entrada (I):  repo_url: Url, ramdisk: &RamdiskHandle
Saída   (O):  Result<RepoPath, CloneError>
```

### 2.1. RepoPath (Saída de Sucesso)

`RepoPath` é um newtype wrapper sobre `PathBuf` que:

- Aponta para o diretório raiz do repositório clonado dentro do Ramdisk
  (ex: `R:\soda_clone_<hash>\` no Windows, `/mnt/soda_ramdisk_<id>/soda_clone_<hash>/` no Linux).
- Implementa `AsRef<Path>` para passagem ergonômica para os nós downstream
  (N3 `SandboxOrchestrator`, N4 `LanguageDetector`, N6 `JCodemunchSidecar`).
- Implementa `Deref<Target = Path>` para acesso transparente aos métodos de `Path`.
- **NÃO** implementa `Clone` nem `Copy` — ownership é transferida
  sequencialmente no pipeline (N2 → N4 → N5 → Extratores → N13).

### 2.2. Geração do Nome do Diretório

O nome do diretório de clone é derivado deterministicamente do URL:

```
soda_clone_<truncated_sha256_of_url>
```

Exemplo: Para `https://github.com/nickel-org/nickel.rs`:
```
R:\soda_clone_a7f3b2c1\
```

Isto evita colisões de nomes e caracteres inválidos no filesystem,
sem depender do nome do repositório (que pode conter caracteres
problemáticos ou colidir entre orgs diferentes).

### 2.3. CloneError (Saída de Falha)

Enum com variantes estritas:

| Variante | Causa | Ação |
|---|---|---|
| `NetworkError { reason: String }` | Timeout de rede, DNS falhou, SSH key inválida | Fail-Fast (aborta job, registra no SQLite) |
| `RepositoryNotFound { url: String }` | URL inválida ou repo deletado/privado (exit code 128) | Fail-Fast (aborta job, marca como `REPO_MORTO` no SQLite) |
| `GitNotInstalled` | Binário `git` não encontrado no `PATH` | Fail-Fast (aborta lote inteiro) |
| `RamdiskFull { path: String }` | Sem espaço no Ramdisk durante o clone | Fail-Fast (aborta job) |
| `Timeout` | Clone excedeu o limite de 120 segundos | Fail-Fast (mata processo filho, aborta job) |

---

## 3. Cenário de Falha Isolado

> **Régua Atômica:** Uma entrada, uma saída, **UM** cenário principal de falha.

### Cenário: Falha de Rede / Repositório Não Encontrado (Fail-Fast)

**Pré-condição:** O Ramdisk está alocado e saudável (N1 OK).
O URL passado aponta para um repositório que foi deletado do GitHub.

**Fluxo:**

1. O `BloblessCloner::clone()` recebe `repo_url = "https://github.com/org/deleted-repo"`
   e uma referência válida `&RamdiskHandle`.
2. Verifica que o binário `git` existe no PATH via `spawn_blocking` (mesmo padrão do PRD-001).
3. Calcula o diretório de destino: `ramdisk.path().join("soda_clone_<hash>")`.
4. Invoca `tokio::process::Command::new("git")` com os argumentos:
   ```
   git clone --filter=blob:none --single-branch <url> <destino>
   ```
5. Aguarda com **timeout de 120 segundos** via `tokio::time::timeout()`.
6. O processo `git` retorna **exit code 128** e stderr contém
   `"fatal: repository 'https://...' not found"`.
7. O módulo parseia o exit code:
   - Exit code 128 + stderr contendo "not found" → `CloneError::RepositoryNotFound`.
8. Retorna `Err(CloneError::RepositoryNotFound { url: "https://..." })`.
9. O Orquestrador MPSC recebe o erro, registra no SQLite
   (`status = REPO_MORTO`) e avança para o próximo job na fila.
10. **Nenhum diretório lixo permanece no Ramdisk** — o diretório parcial
    (se criado pelo git antes de falhar) é removido atomicamente.

**Pós-condição:** O job é ejetado do circuito com log explícito.
O Ramdisk permanece limpo e reutilizável para o próximo repositório.
O Event Loop do Tokio permanece livre.

---

## 4. Proibições Tóxicas Injetadas

### PT-1: PROIBIDO CLONAR NO SSD NVMe ✅ (Aplicação por Construção)

O `BloblessCloner` recebe um `&RamdiskHandle` **por referência imutável**.
O diretório de destino é **sempre** `ramdisk.path().join(...)`.
Não existe parâmetro, configuração ou fallback que permita
direcionar o clone para o SSD.

**Garantia Tipológica:** A assinatura da função torna fisicamente
impossível clonar fora do Ramdisk — o compilador Rust impede
qualquer chamada que não forneça um `&RamdiskHandle` válido.

**Anti-Padrão Proibido:**
```rust
// ❌ PROIBIDO: Path arbitrário como destino
pub async fn clone(url: &Url, dest: &Path) -> Result<RepoPath, CloneError>
```

**Padrão Obrigatório:**
```rust
// ✅ CORRETO: Ramdisk como única opção tipológica
pub async fn clone(url: &Url, ramdisk: &RamdiskHandle) -> Result<RepoPath, CloneError>
```

### PT-3: PROIBIDO BLOQUEAR O EVENT LOOP DO TOKIO ✅

Toda invocação do `git` DEVE ser feita via `tokio::process::Command`
com `.output().await`. O processo filho é gerenciado assincronamente.

Adicionalmente, a verificação de existência do `git` no PATH
DEVE usar `tokio::task::spawn_blocking` (mesmo padrão corrigido
pela Auditoria D1 do PRD-001).

**Anti-Padrão Proibido:**
```rust
// ❌ PROIBIDO: git síncrono
std::process::Command::new("git").arg("clone").output();
```

**Padrão Obrigatório:**
```rust
// ✅ CORRETO: git assíncrono com timeout
let output = tokio::time::timeout(
    Duration::from_secs(120),
    tokio::process::Command::new("git")
        .args(["clone", "--filter=blob:none", "--single-branch", url, dest])
        .output()
).await;
```

---

## 5. Timeout e Kill do Processo Filho

O `git clone` de repositórios grandes ou em redes lentas pode
travar indefinidamente. O `BloblessCloner` DEVE impor um teto
temporal de **120 segundos** usando `tokio::time::timeout()`.

Se o timeout expirar:

1. O processo filho `git` é terminado via `.kill().await` no `Child`.
2. O diretório parcial no Ramdisk é removido (`std::fs::remove_dir_all`).
3. Retorna `Err(CloneError::Timeout)`.

**Nota:** Para implementar kill-on-timeout corretamente, o clone
deve usar `.spawn()` + `child.wait_with_output()` ao invés de
`.output()` direto, permitindo acesso ao `Child` handle para kill.

---

## 6. Limpeza de Artefatos Parciais

Se o `git clone` falhar por **qualquer** motivo (rede, timeout, espaço),
o diretório de destino parcial DEVE ser removido antes de retornar o erro.

```rust
// Pseudocódigo de cleanup
let dest = ramdisk.path().join(dir_name);
match do_clone(&dest).await {
    Ok(()) => Ok(RepoPath(dest)),
    Err(e) => {
        // Limpa diretório parcial, se existir
        if dest.exists() {
            let _ = std::fs::remove_dir_all(&dest);
        }
        Err(e)
    }
}
```

---

## 7. Flags do Git

| Flag | Propósito |
|---|---|
| `--filter=blob:none` | Clone parcial: baixa árvore + commits, sem blobs |
| `--single-branch` | Apenas a branch default (reduz dados transferidos) |
| `--no-tags` | Não baixa tags (economia adicional de banda) |
| `--quiet` | Suprime output verboso do git (reduz dados em stdout/stderr) |

Comando final montado:
```
git clone --filter=blob:none --single-branch --no-tags --quiet <url> <dest>
```

---

## 8. Invariantes de Segurança

1. **Sem Fallback para Disco:** Se o `&RamdiskHandle` for válido, o clone
   vai para a RAM. Se o Ramdisk estiver cheio, retorna `Err(RamdiskFull)`.
   NUNCA tenta escrever no SSD como alternativa.

2. **Sem Clone/Copy em RepoPath:** `RepoPath` NÃO implementa `Clone` nem `Copy`.
   Ownership é transferida linearmente pelo pipeline. Apenas uma referência
   ao diretório clonado existe em qualquer instante.

3. **Cleanup Determinístico:** O diretório de clone vive dentro do Ramdisk.
   Quando o `RamdiskHandle` for dropado (pelo `PurgeGuard` N13), todo o
   conteúdo — incluindo clones — é destruído atomicamente pelo `Drop` do N1.
   O `BloblessCloner` apenas cria subdiretórios; não gerencia ciclo de vida.

4. **Isolamento de Credenciais:** O `git clone` usa HTTPS público.
   Repositórios privados não são suportados na Fase 1 do Harvester.
   Nenhuma chave SSH, token ou cookie é manipulada.

---

## 9. Definition of Done (DoD) para Fase C

- [ ] Teste `test_clone_success` — clona um micro-repo público, verifica que `RepoPath` existe e contém `.git/`
- [ ] Teste `test_clone_repo_not_found` — URL de repo inexistente → espera `CloneError::RepositoryNotFound`
- [ ] Teste `test_clone_stays_in_ramdisk` — verifica que `RepoPath` é prefixo de `ramdisk.path()`
- [ ] Teste `test_cleanup_on_failure` — após falha, o diretório parcial não existe no Ramdisk
- [ ] Teste `test_git_not_installed` — mock do PATH sem `git` → espera `CloneError::GitNotInstalled`
- [ ] `cargo clippy` sem warnings
- [ ] `cargo test` com exit code 0

---

## 10. Dependências de Crates (Propostas)

| Crate | Propósito | Justificativa |
|---|---|---|
| `tokio` | `tokio::process::Command` + `tokio::time::timeout` | Core runtime do SODA (já presente via PRD-001) |
| `url` | Tipo `Url` para parsing e validação de URLs | Garante URL bem-formada antes de invocar git |
| `sha2` | Hash SHA-256 do URL para gerar nome de diretório | Determinístico, sem colisões, sem chars inválidos |
| `thiserror` | Derivar `CloneError` com mensagens claras | Padrão idiomático Rust (já presente via PRD-001) |
| `tracing` | Logs estruturados (início/fim do clone, erros) | Padrão do ecossistema Tokio (já presente via PRD-001) |
