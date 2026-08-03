---
id: "ADR-039"
title: "ADR-039: Auditoria de Cargo FinOps & Pipeline de Build Determinístico"
version: 1.0
status: Ativo_Inegociavel
epic: "Infraestrutura"
description: "Consolida as decisões de otimização do pipeline `cargo build` no SOULS MC (RTX 2060m 6GB / Z: ReFS): sccache persistente, rust-lld, perfis granulares, exclusões Defender, migração NTFS→ReFS e patch GGML_CCACHE=OFF com idempotência. Speedup medido: 120x (sem CUDA) e ~1000x (com CUDA) entre cold e warm."
---

# ADR-039: Auditoria de Cargo FinOps & Pipeline de Build Determinístico

## Status

Aceito (Ativo e Inegociável) — Branch `fix/cargo-finops-v1` consolidada

## Contexto Técnico e Ameaça ao Determinismo

O SOULS MC é uma esteira **bare-metal Rust + Tauri v2** que compila 7 binários supervisionados (`souls_mc`, `souls_mcp_server`, `agentgateway_tcp_proxy`, `mcp_stdio_guard`, `scan_local_models_cli`, `souls_ephemeral_infer_cli`, e variantes CUDA) com dependências pesadas: `wasmtime`, `gix`, `reqwest`, `llama-cpp-sys-2` (vendor pinado em 0.1.152 com 343 arquivos CUDA), `mistralrs`, `tauri-build` (codegen re-iterativo). Ambiente: Windows 11 + Rust 1.94.1 + RTX 2060m (6GB VRAM) com disco Z: (ReFS Dev Drive 80GB) e `target/debug/` chegando a **27+ GB** após builds CUDA.

A auditoria realizada nas sessões 2026-07-31 → 2026-08-01 (documentada em [`.souls_scratchpad/_CARGO_AUDIT_2026-07-31.md`](file:///z:/souls_mc/.souls_scratchpad/_CARGO_AUDIT_2026-07-31.md)) identificou **três ameaças críticas** ao ciclo de desenvolvimento iterativo:

1. **Saturação de tempo de build (cold ~42 min com CUDA)**: o `link.exe` do MSVC é single-thread e gargalo nº1 do link. Sem cache persistente, `cargo clean` invalida **100% do trabalho de rustc** a cada vez. Defender em modo default escaneia cada `.rlib`/`.rmeta` em tempo real, multiplicando I/O por 3-5x.
2. **Bug upstream bloqueante (`option(GGML_CCACHE ... ON)` no llama.cpp)**: o vendor pinado cria um CACHE BOOL no CMake que **ignora `GGML_CCACHE=OFF` no ambiente**. Quando `sccache` está no PATH, CMake wrappa `sccache nvcc`, que falha com `fatbinary: Could not open input file '*.ptx'`. Bloqueio total de builds com `llama_backend` feature (CUDA).
3. **Premissas operacionais incorretas herdadas** (`_CARGO_VERIFICATION.txt`): 5/15 premissas estavam erradas ou não-implementadas (rust-lld ausente, sccache não instalado, ReFS assumido mas Z: era NTFS, perfis granulares inexistentes, `[profile.release]` ausente).

## Decisões Inegociáveis

### 1. Migração do Volume de Build: NTFS → ReFS Dev Drive

- O disco Z: foi inadvertidamente criado como **NTFS** (default do Windows ao formatar). Recriado como **ReFS Dev Drive** via `Format-Volume -FileSystem ReFS -DevDrive`, preservando 85.83 GB com hard links nativos e block cloning.
- ReFS entrega 20-30% mais I/O em workloads de milhares de arquivos pequenos (padrão de `target/debug/deps/` com 22+ GB em 65k arquivos).
- `Get-MpPreference | Select-Object DevDriveProtectionMode` deve retornar `RealTimeAsync` (proteção assíncrona, zero overhead de scan). Limitação aceita: em build 26H2 Insider Preview, este parâmetro não existe; as exclusões de path/processo suprem parcialmente.

### 2. sccache Persistente com Wrapper em `RUSTC_WRAPPER`

- Instalação: `cargo install sccache --locked --jobs 8` (16m 34s para v0.17.0).
- Cache vive em `Z:\.sccache` (NÃO em `target/`), sobrevive a `cargo clean` e a trocas de branch. Budget: `SCCACHE_CACHE_SIZE=8G`.
- Wrapper declarado em [`.cargo/config.toml`](file:///z:/souls_mc/.cargo/config.toml) via `[build] rustc-wrapper = "sccache"`, com env `RUSTC_WRAPPER="sccache"` injetado pelo [`boot.ps1`](file:///z:/souls_mc/boot.ps1).
- Defesa em profundidade: `CARGO_BUILD_JOBS=8` (paralelismo de rustc) declarado no `.cargo/config.toml` e reforçado no `boot.ps1`.
- Flag `--locked` obrigatória no `cargo build` do `boot.ps1` (evita regenerar `Cargo.lock` e invalidar chaves do cache).

### 3. rust-lld como Linker (Paralelização do Link)

- `linker = "rust-lld.exe"` em `[target.x86_64-pc-windows-msvc]`. O `link.exe` do MSVC é single-thread e gargalo principal; rust-lld paraleliza por padrão.
- Rustflags adicionais: `/OPT:REF` (remove seções não-referenciadas), `/OPT:ICF` (folding de funções idênticas), `/Brepro` (reprodutibilidade bit-a-bit do binário).
- **NÃO usamos** `/GUARD:CF` (mitigação Spectre) no dev — adiciona latência de link sem ganho mensurável em prototipagem.

### 4. Perfis Granulares (dev / test / release) no Cargo

- **[`.cargo/config.toml`](file:///z:/souls_mc/.cargo/config.toml) `[profile.dev]`**:
  - `incremental = false` (LEI DE FERRO — Tauri-codegen + multi-vendor corrompe cache incremental em silêncio; confiamos 100% no sccache).
  - `opt-level = 1` em código nosso (ganho de runtime sem esticar compile).
  - `debug = 1` (line-tables only — stack traces claros, sem PDBs gigantes de ~30 MB por crate).
  - `codegen-units = 256` em código nosso (máxima paralelização de LLVM).
- **`[profile.dev.package."*"]`**: `opt-level=3`, `codegen-units=1` para deps (estabilidade de RAM; evita RSS >12 GB durante build paralela de 5 binários).
- **Memory-killers explícitos** (`opt-level=0`): `wasmtime`, `wasmtime-cranelift`, `wasmtime-environ`, `gix`, `reqwest`, `llama-cpp-sys-2`, `mistralrs`. Mantém RSS do build agregado < 6 GB (alinhado ao orçamento VRAM do host).
- **[`src-tauri/Cargo.toml`](file:///z:/souls_mc/src-tauri/Cargo.toml) `[profile.release]`**: `lto = "thin"`, `panic = "abort"`, `codegen-units = 1`, `strip = "symbols"`, `opt-level = 3`, `incremental = false`.
- **`[profile.test]`** herda de dev, mas `opt-level = 0` (sem ganho real em otimizar para test; economiza ~10%).

### 5. Patch Local `GGML_CCACHE=OFF` no Vendor + Idempotência

- **Arquivo patcheado**: [`src-tauri/vendor/llama-cpp-sys-2/llama.cpp/ggml/CMakeLists.txt:125`](file:///z:/souls_mc/src-tauri/vendor/llama-cpp-sys-2/llama.cpp/ggml/CMakeLists.txt#L125-L125)
  - Antes: `option(GGML_CCACHE "ggml: use ccache if available" ON)`
  - Depois: `option(GGML_CCACHE "ggml: use ccache if available" OFF)`
- **Causa raiz**: `option(... ON)` cria CACHE BOOL no CMake com precedência sobre env var. `GGML_CCACHE=OFF` no ambiente é **ignorado**. CMake sempre wrappa `sccache nvcc` quando `sccache` está no PATH, e nvcc falha com `fatbinary: Could not open input file '*.ptx'`.
- **Idempotência obrigatória no [`boot.ps1`](file:///z:/souls_mc/boot.ps1#L29-L43)** (linhas 29-43): bloco PowerShell que re-aplica o patch em todo boot, tolerante a `cargo update` que reverte o vendor. Regex match → `-replace` → `Set-Content`. Logado como `[PATCH] vendor/llama-cpp-sys-2 GGML_CCACHE -> OFF (auto-fix CUDA+sccache)`.
- **Workarounds testados e rejeitados** (todos falharam): `GGML_CCACHE=OFF` no env (ignorado), `SCCACHE_DISABLE=1` (RULE_LAUNCH_COMPILE do build.ninja sobrescreve), `find_program` filter (ninja cacheado ignora PATH), renomear `sccache.exe` (ninja invoca por path absoluto), `cargo clean -p llama-cpp-sys-2` (fingerprint recriado).
- **Issue draft** preparado em [`.souls_scratchpad/_ISSUE_DRAFT_GGML_CCACHE.md`](file:///z:/souls_mc/.souls_scratchpad/_ISSUE_DRAFT_GGML_CCACHE.md) para postar em `ggml-org/llama.cpp` (template `010-bug-compilation.yml`).

### 6. 8 Exclusões Windows Defender (Paths + Processes)

- Script: [`scripts/add_defender_exclusions.ps1`](file:///z:/souls_mc/scripts/add_defender_exclusions.ps1) — auto-eleva privilégio, idempotente.
- **Bug corrigido na linha 74**: `@{ $Kind = $Value }` produzia `Path`/`Process` em vez de `ExclusionPath`/`ExclusionProcess` que `Add-MpPreference` espera. Patch aplicado: `if ($Kind -eq 'Path') { Add-MpPreference -ExclusionPath $Value } else { Add-MpPreference -ExclusionProcess $Value }`.
- **Paths (3)**: `Z:\souls_mc`, `Z:\souls_mc\src-tauri\target`, `C:\Users\rosas\.cargo`, `C:\Users\rosas\.rustup`.
- **Processes (5)**: `rustc.exe`, `cargo.exe`, `sccache.exe`, `rust-lld.exe`, `link.exe`.
- Aplicadas em janela admin via `Invoke-TrackedProcess` ou execução direta. Validação: `Get-MpPreference | Select-Object ExclusionPath -ExpandProperty ExclusionPath` lista 7+ paths.

### 7. Baseline Empírico Capturado (Medições, Não Estimativas)

| Build | Cold real | Warm | Speedup |
|---|---|---|---|
| **Sem `llama_backend`** (CUDA off) | **16m 35s** (708 rustc) | 1-8s | **~120x** |
| **Com `llama_backend`** (CUDA on) | **42m 9s** (343 CUDA + 708 rustc) | 2.4s | **~1000x** |

- **Cold real corrigido pós-`cargo clean`**: o baseline inicial de 2m era **warm disfarçado de cold** (artefatos em `target/debug/` do build anterior). O cold real é 16-17 min para o grafo completo sem CUDA. Builds < 5 min "cold" sem `cargo clean` prévio são na verdade warm.
- `cargo clean` liberou **27.66 GB em 30s** de `target/debug/` pós-builds CUDA. Padrão correto: `cargo clean -p <crate>` para limpar 1 pacote; `cargo clean` total é operação rara.

## Consequências e Trade-offs

### Impactos Positivos

- **Speedup massivo entre iterações**: warm builds < 10s (sem CUDA) e < 3s (com CUDA). O ganho está 100% no ciclo dev iterativo, não no cold (que ainda é demorado por causa do CUDA device code).
- **CUDA destravado**: builds com `llama_backend` (feature `["dep:llama-cpp-2", "llama-cpp-2/cuda", "dep:llguidance"]`) agora completam em 42m cold / 2.4s warm. Antes: EXIT 101 imediato.
- **Idempotência absoluta**: o patch GGML_CCACHE sobrevive a `cargo update` via auto-reaplicação no `boot.ps1`. O Defender exclusions é idempotente. O sccache cache é persistente em Z: (ReFS).
- **I/O alinhado com VRAM**: perfis granulares mantêm RSS do build agregado < 6 GB, alinhado ao teto da RTX 2060m. Sem OOM durante build paralela de 5 binários.
- **Documentação cross-session**: [`project_memory.md`](file:///c:/Users/rosas/.trae/memory/projects/-z-souls-mc/project_memory.md) carrega baseline, hard constraints, lessons learned — engenheiro SOULS retoma sem repetir auditoria.

### Impactos Negativos

- **sccache NÃO wrappa compiladores C/C++ do llama.cpp**: o `option(GGML_CCACHE ... OFF)` desabilita o cache de C/C++ no llama.cpp. Ganho perdido: ~0.5-1s no cold. Aceitável (era bloqueante antes).
- **Patch vendor é invasivo**: embora idempotente, depende de patch local em arquivo do vendor. Se o upstream mudar a linha (ex: reformatação), o regex do `boot.ps1` pode não casar. Mitigação: regex tolerante a whitespace variável (`\s+ON\)`).
- **Mover `CARGO_HOME` para Z: foi rejeitado**: ganho marginal (10-20% I/O no cold) com risco não-zero de quebrar symlinks/hooks do rustup. Trade-off FinOps desfavorável.
- **Mover `~/.rustup` foi rejeitado**: mesma análise — risco médio, ganho marginal (5-10% cold).
- **Unificar `base64` 0.21.7/0.22.1 via `[patch.crates-io]` rejeitado**: ganho <1% no cold, risco alto de quebrar grafo de features. `cargo tree --workspace --duplicates` documenta, mas unificação não compensa.

## Restrições Bare-Metal

- **Latência de warm build**: deve permanecer **abaixo de 10s** (sem CUDA) e **abaixo de 5s** (com CUDA). Acima disso, regressão indica invalidação do cache sccache ou mudança estrutural que requer nova auditoria.
- **Carga de RAM durante build**: RSS agregado do `cargo build` (paralelo de 5 binários) deve permanecer **abaixo de 6 GB** (alinhado ao orçamento VRAM da RTX 2060m).
- **Patch GGML_CCACHE idempotente**: o `boot.ps1` DEVE re-aplicar o patch em toda execução. `cargo update` que reverter o vendor sem re-patch é regressão — bloquear `cargo update` automático e exigir revisão manual.
- **Defender exclusions ativas**: 8 exclusões (4 paths + 5 processes) devem estar sempre presentes. Validação: `Get-MpPreference | Select-Object ExclusionPath, ExclusionProcess` retorna lista não-vazia antes de qualquer `cargo build` em CI/CD.
- **Disco Z: livre**: manter **≥ 20 GB livres** em Z: (ReFS) para acomodar `target/debug/` (até 27 GB em cold CUDA) + sccache cache (8 GB) + overhead de build scripts. Abaixo disso: `cargo clean -p <crate>` antes de build pesado.
- **`incremental = false` inegociável**: Tauri-codegen + multi-vendor corrompe o cache incremental em silêncio. Habilitar `incremental = true` reverte FinOps de semanas.

## Anti-Slop (Decisões Rejeitadas)

- ❌ **`cargo clean` como rotina** — proibido. Use `cargo clean -p <crate>` para 1 pacote.
- ❌ **Mover `CARGO_HOME` (`.cargo/registry`) para Z:** — risco/benefício desfavorável.
- ❌ **Mover `~/.rustup/toolchains` para Z:** — risco médio, ganho marginal.
- ❌ **Habilitar `incremental = true` em dev** — Tauri-codegen + multi-vendor corrompe cache (lei de ferro).
- ❌ **`cargo nextest` por padrão** — pode ser mais lento no Windows para suítes pequenas.
- ❌ **`opt-level = "s"` em release** — inferência numérica (mistralrs/llama) se beneficia agressivamente de AVX2; opt-level=3 é correto.
- ❌ **`Set-MpPreference -DisableRealtimeMonitoring $true`** — risco de segurança inaceitável.
- ❌ **Migrar para cloud build** — overhead FinOps, sem ganho claro.
- ❌ **Instalar nightly Rust** — perde reprodutibilidade do toolchain.

## Pendentes (Backlog de Investigação — Não Críticos)

- **D1 (tauri handler split)**: wrap cada handler em sua própria função para evitar re-typecheck de TODOS os commands. Adiar para sprint FinOps dedicado.
- **C4 (workspace split)**: avaliar split de `souls_mc_lib` em `core/`, `infer/`, `harvest/`, `mcp/` para criar fronteiras de compilação.
- **F4 (cargo tree --duplicates)**: unificar `base64` 0.21.7/0.22.1, `bitflags` 2.13.0 via features mais específicas em `tauri-utils` (ganho <1%, não urgente).
- **B1 (cargo --timings)**: já executado (1.38s, pouca info útil para o grafo atual). Re-executar após workspace split.
- **O1-O7 (auditorias específicas)**: target/build 2.99 GB, examples 363 MB, vendor pinning, `tauri-plugin-opener` audit, etc.
- **Reportar bug ao upstream**: postar issue draft em `ggml-org/llama.cpp` (decisão do engenheiro humano).

## Referências Operacionais

- Auditoria completa: [`.souls_scratchpad/_CARGO_AUDIT_2026-07-31.md`](file:///z:/souls_mc/.souls_scratchpad/_CARGO_AUDIT_2026-07-31.md) (16 seções, 858 linhas).
- Issue draft: [`.souls_scratchpad/_ISSUE_DRAFT_GGML_CCACHE.md`](file:///z:/souls_mc/.souls_scratchpad/_ISSUE_DRAFT_GGML_CCACHE.md).
- Config build: [`.cargo/config.toml`](file:///z:/souls_mc/.cargo/config.toml).
- Profile release: [`src-tauri/Cargo.toml`](file:///z:/souls_mc/src-tauri/Cargo.toml).
- Bootstrap: [`boot.ps1`](file:///z:/souls_mc/boot.ps1).
- Defender exclusions: [`scripts/add_defender_exclusions.ps1`](file:///z:/souls_mc/scripts/add_defender_exclusions.ps1).
- Vendor patch: [`src-tauri/vendor/llama-cpp-sys-2/llama.cpp/ggml/CMakeLists.txt`](file:///z:/souls_mc/src-tauri/vendor/llama-cpp-sys-2/llama.cpp/ggml/CMakeLists.txt) (linha 125).
- Cross-session memory: [`project_memory.md`](file:///c:/Users/rosas/.trae/memory/projects/-z-souls-mc/project_memory.md).
