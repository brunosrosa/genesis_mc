# _ARCH_REFLECTION_REPORT_HARVESTER (Loop Profundo / Autópsia F0)
Data: 2026-06-30  
Escopo: Harvester Fase 0 (roteamento, orquestração, sandbox/sidecars e persistência).

## Jurisdição (Topologia)
Este relatório foi gravado em **docs/state/** para respeitar a Lei de Higiene de Workspace (fobia de raiz) e a Topologia SODA.

## Arquivos auditados (núcleo)
- **Z:\genesis_mc\src-tauri\src\harvester\router.rs**
- **Z:\genesis_mc\src-tauri\src\harvester\orchestrator.rs**
- **Z:\genesis_mc\src-tauri\src\harvester\persist.rs**
- **Z:\genesis_mc\src-tauri\src\harvester\sandbox.rs**
- **Z:\genesis_mc\src-tauri\src\harvester\sidecar.rs**
- **Z:\genesis_mc\src-tauri\src\bin\f0_harvester_cli.rs**

## Checklist do Arquiteto (itens exigidos)
### 1) `.unwrap() / .expect() / .clone()` “preguiçosos”
**Achados (evidência):**
- **persist.rs**: `.unwrap()` aparece em testes (in-memory DB) e não no caminho principal da persistência. Caminho principal usa `map_err` para traduzir falhas em `HarvesterError::StorageError` e executa `transaction()` + `commit()` (ver [persist.rs](file:///z:/genesis_mc/src-tauri/src/harvester/persist.rs)).
- **sidecar.rs**: há alto volume de `.clone()` no caminho de produção (ex.: blocos de progresso / targets / strings intermediárias). A busca mostra vários clones em áreas não-test (ex.: `guard.blocks = blocks.clone()` em [sidecar.rs:L1904-L1913](file:///z:/genesis_mc/src-tauri/src/harvester/sidecar.rs#L1904-L1913)).  
  Interpretação: parte disso é plausível (ex.: snapshot para telemetria/progresso), mas o volume em um arquivo de 7.5k linhas indica risco de “slop de alocação” (ver severidade abaixo).
- **f0_harvester_cli.rs**: há `unwrap_or_else` e `unwrap_or_default` em fluxos operacionais (ex.: variável de ambiente e leitura pós-run) (ver [f0_harvester_cli.rs:L820-L924](file:///z:/genesis_mc/src-tauri/src/bin/f0_harvester_cli.rs#L820-L924)). Não é fatal, mas sinaliza tolerância a falhas silenciosas (perde erro real e segue com default).
- **router.rs**: o caminho principal do parser `--only-blobs` é “hard fail” (retorna `Err(String)` em entrada inválida) e evita unwrap no runtime de produção (ver [router.rs:L78-L101](file:///z:/genesis_mc/src-tauri/src/harvester/router.rs#L78-L101)).

**Classificação:**
- SLOP Cosmético: unwraps em testes, clones pequenos de strings, utilitários.
- Risco Estrutural: clones grandes em `sidecar.rs` sem prova explícita de necessidade (potencial blow-up de RAM/tempo e “Flow-Debt” em bases grandes).

### 2) Variáveis/imports/funções órfãs (dead_code)
**Evidência mecânica disponível:** diagnósticos do editor retornaram vazio (sem warnings).  
Observação: isso não prova ausência total de dead_code (pode haver `#[allow(dead_code)]`/cfg gates), mas reduz a probabilidade de lixo óbvio ter passado.

### 3) Teardown (zumbis) sob Ctrl+C / abort
**Pontos fortes (o que está sólido):**
- Execução de subprocesso no sandbox usa `tokio::process::Command` com `.kill_on_drop(true)` (ver [sandbox.rs:L833-L841](file:///z:/genesis_mc/src-tauri/src/harvester/sandbox.rs#L833-L841)).
- No Windows, o sandbox anexa o processo a Job Object com `JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE` (busca aponta `KILL_ON_JOB_CLOSE` em `sandbox.rs`), reduzindo órfãos quando o processo pai morre.
- Em timeout, o sandbox chama `child.kill().await`, depois `kill_process_tree_by_pid(pid).await` e `reap_command_orphans(...)` (ver [sandbox.rs:L1024-L1079](file:///z:/genesis_mc/src-tauri/src/harvester/sandbox.rs#L1024-L1079)).
- `sidecar.rs` centraliza execução via `execute_sidecar(...)` → `executor.execute_in_dir(...)` (ver [sidecar.rs:L1191-L1244](file:///z:/genesis_mc/src-tauri/src/harvester/sidecar.rs#L1191-L1244)), mantendo o “caminho único” sob o sandbox.

**Furo real (slop / risco):**
- **Inconsistência crítica entre comentário e realidade no Drop do SandboxHandle.**  
  O comentário declara que o `join()` garante Drop síncrono, mas o código faz `std::thread::spawn(...)` e descarta o handle (não há `join`) (ver [sandbox.rs:L1115-L1137](file:///z:/genesis_mc/src-tauri/src/harvester/sandbox.rs#L1115-L1137)).  
  Implicação: em shutdown/ctrl+c, o Drop pode retornar antes de `taskkill` terminar. Em cenários de encerramento agressivo, isso aumenta a chance de processos ainda vivos por uma janela curta.

**Classificação:**
- Risco Estrutural (alto): divergência comentário vs comportamento real no teardown (pode mascarar “zumbi curto” e quebra o contrato mental de RAII).

### 4) UPSERT e transação SQLite (BusyLock)
**Pontos fortes:**
- Persistência usa `spawn_blocking` e `transaction()` + `commit()`; o commit está explícito (ver [persist.rs](file:///z:/genesis_mc/src-tauri/src/harvester/persist.rs)).
- O UPSERT é por `(repo_id, artifact_type)` e preserva blobs não reprocessados (há teste dedicado confirmando o requisito) (ver [persist.rs:L116-L155](file:///z:/genesis_mc/src-tauri/src/harvester/persist.rs#L116-L155)).

**Riscos residuais:**
- O `Arc<Mutex<Connection>>` é travado durante toda a transação dentro do `spawn_blocking`. Se existir outro escritor concorrente no mesmo `Connection` (ou se houver múltiplos tasks tentando persistir ao mesmo tempo com o mesmo `Arc`), o sistema vira “fila” (não deadlock, mas degradação/latência).  
  Isso pode ser aceitável (1 escritor), mas merece decisão explícita.
- Não vi evidência local (neste recorte) de `busy_timeout`/WAL tuning no `Connection`; se não existir em outro lugar do projeto, risco de `SQLITE_BUSY` em cenários de concorrência real.

**Classificação:**
- SLOP Cosmético: timestamp calculado dentro do loop (varia por blob; geralmente irrelevante).
- Risco Estrutural (médio): ausência explícita de política de busy/timeout e lock coarse no `Connection`.

### 5) `--only-blobs` quebrou o default?
**Evidência do comportamento atual:**
- `HarvesterOrchestrator::run(..., requested_blobs: Option<BlobSelection>)` faz `requested_blobs.unwrap_or_else(BlobSelection::all)` e passa `requested_blobs: Some(&requested_blobs)` ao router (ver [orchestrator.rs:L175-L182](file:///z:/genesis_mc/src-tauri/src/harvester/orchestrator.rs#L175-L182)).
- O router, ao receber `Some(selection)`, filtra tasks via `selection.allows_task(task)` (ver [router.rs:L293-L300](file:///z:/genesis_mc/src-tauri/src/harvester/router.rs#L293-L300)).

**Risco estrutural específico (regressão silenciosa):**
- Mesmo quando o usuário NÃO passa `--only-blobs`, o pipeline fica dependente de duas SSOTs ficarem sempre sincronizadas:
  - `PHASE0_BLOB_TYPES` (lista de artefatos) e
  - `BlobSelection::allows_task(...)` (mapeamento artefato → task).
  Se um blob novo entrar no pipeline e alguém esquecer de atualizar essas tabelas, o “default” pode deixar de ser completo sem erro explícito (filtra tasks sem o operador pedir filtro).

**Classificação:**
- Risco Estrutural (alto): default-path amarrado a filtro (em vez de ser “sem filtro” quando a flag é ausente).

## Sumário de “furos” (Severidade)
### Risco Estrutural (alto)
- Teardown: comentário promete `join()` no Drop do sandbox, mas o código não faz join (contrato mental quebrado) (ver [sandbox.rs:L1115-L1137](file:///z:/genesis_mc/src-tauri/src/harvester/sandbox.rs#L1115-L1137)).
- `--only-blobs`: default depende de filtro ativo (`Some(BlobSelection::all)`), sujeito a regressão silenciosa se `PHASE0_BLOB_TYPES`/`allows_task` ficarem desatualizados (ver [orchestrator.rs:L175-L182](file:///z:/genesis_mc/src-tauri/src/harvester/orchestrator.rs#L175-L182), [router.rs:L115-L131](file:///z:/genesis_mc/src-tauri/src/harvester/router.rs#L115-L131)).

### Risco Estrutural (médio)
- SQLite: lock coarse no `Arc<Mutex<Connection>>` + ausência de evidência explícita de `busy_timeout`/tuning (pode virar gargalo/`SQLITE_BUSY`) (ver [persist.rs](file:///z:/genesis_mc/src-tauri/src/harvester/persist.rs)).
- `sidecar.rs`: volume elevado de clones em caminho de produção em arquivo enorme (risco de alocação e piora em repos grandes).

### SLOP Cosmético
- Comentários “PT-* / D1 CORRIGIDO” embutidos em arquivos core (ex.: `persist.rs`, `sandbox.rs`, `router.rs`). Não quebra runtime, mas cria ruído e risco de divergência futura (o caso do Drop já materializou esse risco).
- `unwrap_or_default`/silenciamento de erro em leituras pós-run no CLI (tolerância a falhas sem sinalização forte) (ver [f0_harvester_cli.rs:L917-L924](file:///z:/genesis_mc/src-tauri/src/bin/f0_harvester_cli.rs#L917-L924)).

## Recomendações (sem aplicar correções agora)
- Teardown: alinhar comentário vs comportamento real no Drop; decidir explicitamente se o Drop deve ser bloqueante (join) ou “best-effort” (detached) e ajustar o contrato.
- `--only-blobs`: restaurar o default “sem filtro” quando a flag não existe (ou adicionar validação/assertiva para garantir que `BlobSelection::all()` cobre tudo o que o pipeline pode produzir).
- `sidecar.rs`: mapear os clones que carregam payloads grandes (blobs/targets) vs clones baratos; reduzir cópias de coleções grandes se aparecerem nos perfis de runtime.
