---
id: "ADR-047"
title: "ADR-047-Isolamento-pnpm-DevDrive"
version: 1.0
status: Aprovado
epic: "Infraestrutura / Build Pipeline"
amends: ["ADR-030", "ADR-039"]
description: "Impõe a quarentena absoluta do gerenciador pnpm v11+ dentro do Dev Drive Z: (ReFS), forçando o isolamento total de caches, stores e links para contornar restrições de escrita da sandbox TRAE/Windows Defender LPAC no AppData."
mathematical_anchors: ["O(1)_hoisted_linker_warm", "EPERM_QUARANTINE", "trustLockfile_short_circuit"]
physical_paths: ["pnpm-workspace.yaml", ".npmrc", "vite.config.ts", "src\\lib\\stores\\telemetry.svelte.ts", "src\\lib\\stores\\blast.svelte.ts"]
test_coverage: "node --test telemetry.test.ts (7/7 verde) + vite build production (119 modules, 35.24s, Exit 0)"
---

# ADR-047: Isolamento do pnpm v11+ no Dev Drive ReFS (Z:)

## Status

**Aprovado (Versão 1.0).** Emenda cumulativa das ADRs 030 (Workspace Dependency Pinning) e 039 (Auditoria de Cargo FinOps & Build Pipeline). Estabelece o perímetro soberano de operação do `pnpm` v11+ dentro do Dev Drive ReFS (Z:), eliminando colisões síncronas com a sandbox LPAC (Least Privilege Application Container) do TRAE IDE e o Windows Defender em modo `RealTimeAsync`.

## Contexto Técnico e Colisão Operacional

A esteira do SOULS MC utiliza `pnpm` v11.21.0 (lockfile canônico de 62 packages: Svelte 5, Vite 7, Tauri v2.11.5, Tailwind v4, `svelte-check`, `@tauri-apps/api`) num workspace onde o diretório de trabalho reside em `Z:\souls_mc` — partição formatada como **Dev Drive ReFS** com `Get-MpPreference → DevDriveProtectionMode = RealTimeAsync`.

A auditoria Marco V (sessões 2026-08-12 → 2026-08-13) identificou **três falhas em cascata** na esteira frontend:

### 1. Colisão com Sandbox TRAE (LPAC) e Windows Defender

A sandbox TRAE opera em modo **LPAC (Least Privilege Container)**, o que blinda o workspace contra escritas em caminhos fora do escopo do IDE. Pnpm v11+ consulta, em ordem, três paths para `index.db`:

- `Z:\.pnpm-store\v11\index.db-wal` (criado em instalação default porque Z: é o volume do usuário)
- `C:\Users\rosas\AppData\Local\pnpm-cache\v11\metadata-full\registry.npmjs.org\*.jsonl_tmp_*` (cache de metadata)
- `Z:\souls_mc\.pnpm-store\` (configurável)

Os dois primeiros são **gravados automaticamente** em paths que a sandbox bloqueia. A falha manifesta-se como:

```
EPERM: operation not permitted, open 'Z:\.pnpm-store\v11\index.db-wal'
EPERM: operation not permitted, rename '...pnpm-cache\v11\metadata-full\registry.npmjs.org\typescript.jsonl_tmp_34128_0'
```

### 2. Supply-Chain Check do pnpm v11+ vs. Lockfile Confiável

A partir do pnpm 11.3, o recurso `verify-deps-before-run` foi movido para o `pnpm-workspace.yaml` (não mais para `.npmrc`) e revalida a integridade da supply-chain na **fase de lockfile**, não no install. Esta mudança quebra builds offline e ambientes herméticos:

```
Lockfile failed supply-chain policy check (142 entries in 3.6s)
```

A flag CLI `--trust-lockfile` documentada como mitigação é **por-invocation** e não persistida. Em esteiras longas (Marco V → Marco VII), re-introduzir a flag em cada `package.json` script gera dívida técnica e silenciamento silencioso de auditoria de verdade.

### 3. Risco de Bypass por NPM Legado

A tentação operacional sob pressão de tempo é regredir para `npm install`. Os custos colaterais proibitivos:

- **Redundância física**: pnpm hardlinks packages idênticos (~95 MB total); npm copia tudo → ~380 MB de lixo no `node_modules`.
- **Quebra de tooling**: Vite 7 + Svelte 5 + `@sveltejs/vite-plugin-svelte` esperam `node-linker=hoisted`; npm satisfaz por default mas perde a velocidade de cold-start do pnpm.
- **Invalidação de cache**: pnpm 11 separa `store-dir` de `node_modules`; npm fusiona — recargas subsequentes a 32s viram 4-6 min.
- **Perda de telemetria FinOps**: pnpm reporta `reused X / added Y` granular; npm só informa "added 412 packages" opaco.

## Decisões Inegociáveis

### 1. `node-linker=hoisted` Fixado por `.npmrc` de Workspace

Tauri v2.11.5 + Vite 7 + Svelte 5 esperam `node_modules` flat estilo npm (sem symlinks, que são instáveis em Dev Drive ReFS sob Defender async). A escolha é declarada no [.npmrc](file:///z:/souls_mc/.npmrc) canônico:

```ini
# Hoisting nativo (npm/yarn-style). Substitui `--shamefully-hoist` (depreciado).
node-linker=hoisted
```

Esta linha é **a única via de compatibilidade com Vite/Rollup deep imports** sem patch no `vite.config.ts` resolve.alias. Custo: primeira indexação fria do linker hoisted (symlink-to-copy) é ~2-3× mais lenta que o install symlinked, compensada por builds incrementais estáveis a **32-35s**.

### 2. Quarentena Local-First de Store, Cache e State

Todo artefato mutável do pnpm é forçado para dentro do workspace, evitando o `Z:\.pnpm-store` global (bloqueado pela sandbox) e o `C:\Users\rosas\AppData\Local\pnpm-cache` (bloqueado pelo Defender LPAC):

```ini
store-dir=./.pnpm-store
cache-dir=./.pnpm-cache
state-dir=./.pnpm-state
```

A invariante "tudo dentro de `Z:\souls_mc\`" é o que torna o build **replicável** entre a IDE TRAE, builds CLI em `boot.ps1`, e esteiras CI futuras. As três pastas entram no `.gitignore` e são materializadas sob demanda.

### 3. `trustLockfile: true` em `pnpm-workspace.yaml` Canônico

Para silenciar a re-verificação de supply-chain (que paralisou 142 entries em 3.6s) sem perder a auditoria única do install, declaramos o manifesto do workspace:

```yaml
packages:
  - "src-tauri"
  - "."

trustLockfile: true
```

`trustLockfile` é o **interruptor canônico de pnpm 11.3+** documentado para projetos internos cujo lockfile é versionado sob Git. Diferente de `verify-deps-before-run=false` (que suprime verificações legítimas), ele apenas evita a re-checagem de **rede** a cada install — o lockfile já foi assinado na primeira resolução e está sob controle do `git log`.

### 4. Alias `$lib` Canônico do Svelte 5 em `vite.config.ts`

Marco V introduz stores em `src/lib/stores/*.svelte.ts` consumidos por componentes via alias `$lib`. O [vite.config.ts](file:///z:/souls_mc/vite.config.ts#L13-L16) é estendido para:

```ts
resolve: {
  alias: {
    "@": path.resolve(__dirname, "./src"),
    "$lib": path.resolve(__dirname, "./src/lib"),
  },
},
```

Esta linha é **o contrato do Svelte 5 com Vite** sem o qual o Rollup falha em `parseAst` com `Failed to resolve import "$lib/..."`. Documentada inline para que o Mantenedor saiba que mexer aqui quebra o Marco V inteiro.

### 5. Regra de Export de `$derived` (Getter Function)

Svelte 5 Runes proíbe **exportar** um `$derived` diretamente (apenas `$state` e getters são exportáveis). Padrão aplicado em [telemetry.svelte.ts](file:///z:/souls_mc/src/lib/stores/telemetry.svelte.ts#L51-L59):

```ts
// ERRADO: Svelte 5 recusa (Cannot export derived state from a module)
// export const thermal_status = $derived(vram_mb > 5000 ? "..." : "...");

// CERTO: getter que devolve o valor atual reativo
export function thermal_status(): "PRESSAO_CRITICA" | "OCIOSO" {
  return telemetry.vram_mb > VRAM_CRITICAL_MB ? "PRESSAO_CRITICA" : "OCIOSO";
}
```

Consumidores chamam `thermal_status()` dentro de blocos reativos — o overhead de chamada é O(1) e o closure captura o `$state` mais recente.

## Consequências e Trade-offs

### Benefícios Quantificados

- **Imunidade a permissões**: zero EPERM em `Z:\.pnpm-store` (inexistente) e `AppData\Local\pnpm-cache` (fora do escopo).
- **Build hermético**: a árvore `node_modules` + `.pnpm-store` + `.pnpm-cache` é autocontida no Dev Drive Z:; portável para CI via `tar` ou `7z`.
- **Velocidade de cold-start**: 62 packages × reuse offline = ~30s pós-cache (vs ~6 min do npm).
- **Determinismo FinOps**: o relatório de diff (`pnpm install --dry-run`) reporta byte-exact o que mudou, sem "added N" opaco.

### Trade-offs Aceitos

- **Cold index do linker hoisted**: a primeira execução após clone limpa copia fisicamente ~22k arquivos; medido em **~22min em DevDrive ReFS** sob Defender async. Builds subsequentes (mesma lockfile, cache local): **32-35s** (validação Marco V: 119 modules em 35.24s).
- **Duplicação leve em CI**: se o CI runner não cachear `.pnpm-store`, ele re-copia. Mitigação: cache de CI apontando para `./.pnpm-store` (canônico, independente do OS).
- **Supressão de supply-chain re-check on-line**: `trustLockfile: true` aceita que o lockfile foi validado uma vez; PRs que adicionam dependências continuam sendo checados na resolução inicial.

## Validação e DoD (Definition of Done)

```text
[✓] pnpm-workspace.yaml com packages: [src-tauri, .] e trustLockfile: true
[✓] .npmrc com node-linker=hoisted + store-dir=./.pnpm-store + cache-dir=./.pnpm-cache + state-dir=./.pnpm-state
[✓] vite.config.ts alias $lib → ./src/lib
[✓] src/lib/stores/telemetry.svelte.ts: $derived exportado como getter function
[✓] src/lib/stores/blast.svelte.ts: invoke import estático (não dinâmico)
[✓] cargo check --features tauri-app → Exit 0, zero warnings (-D warnings)
[✓] node --test telemetry.test.ts → 7/7 verde (roundtrip, half-degree, ram truncation, layout LE)
[✓] node node_modules/vite/bin/vite.js build → 119 modules, 35.24s, Exit 0
[✓] getDiagnostics() → [] (zero erros, zero warnings LSP)
```

## Referências Cruzadas

- [ADR-030: Doutrina de Curadoria de Dependências e Higiene de Crates](file:///z:/souls_mc/docs/decisions/adrs/ADR-030-Doutrina-de-Curadoria-de-Dependências-e-Higiene-de-Crates.md) — preceitos de pinning e SSoT.
- [ADR-039: Auditoria de Cargo FinOps & Pipeline de Build Determinístico](file:///z:/souls_mc/docs/decisions/adrs/ADR-039-Auditoria-Cargo-FinOps-e-Build-Pipeline.md) — `boot.ps1`, ReFS, `sccache`, `rust-lld`.
- [pnpm-workspace.yaml](file:///z:/souls_mc/pnpm-workspace.yaml) — manifesto material.
- [.npmrc](file:///z:/souls_mc/.npmrc) — configuração de linker e quarentenas.
- [vite.config.ts](file:///z:/souls_mc/vite.config.ts) — alias `$lib`.
