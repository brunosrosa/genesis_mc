# ==============================================================================
# SODA ETL IGNITION MATRIX V5 - ORQUESTRADOR BARE-METAL (JANELA DE VIDRO)
# ==============================================================================

param(
    [string]$Choice = "",
    [switch]$DryRun,
    [switch]$Yes,
    [string]$RepoId = ""
)
# ETL eh one-shot, queremos ver TUDO: debug do core, silencia apenas o ruido
# cosmico do `ignore`/`globset`/`walkdir` (~600 linhas de "built glob set").
# Pos-B: crate agora chama souls_mc_lib (renomeada em B).
$env:RUST_LOG = "souls_mc_lib=debug,soda_sast=debug,soda_harvester=debug,ignore=warn,globset=warn,walkdir=warn"
try {
    [console]::InputEncoding = [console]::OutputEncoding = New-Object System.Text.UTF8Encoding
}
catch {}
if ($null -ne $PSStyle) {
    $PSStyle.OutputRendering = 'ANSI'
}
try { Clear-Host } catch {}

Write-Host "======================================================================" -ForegroundColor Cyan
Write-Host " 🦅 SOULS MC (SODA Stack) - PAINEL DE IGNIÇÃO ETL V5 (JANELA DE VIDRO)" -ForegroundColor White -BackgroundColor DarkBlue
Write-Host "======================================================================" -ForegroundColor Cyan

# 1. BLINDAGEM DE AMBIENTE: CARREGA O .ENV PARA A RAM
Write-Host "`n[+] Calibrando Reator: Injetando chaves do .env na memória..." -ForegroundColor DarkGray
$rootCandidate = Join-Path $PSScriptRoot ".."
$rootResolved = $rootCandidate
try { $rootResolved = (Resolve-Path -LiteralPath $rootCandidate -ErrorAction Stop).Path } catch {}
$envPath = Join-Path $rootResolved ".env"
if (Test-Path $envPath) {
    Get-Content $envPath | ForEach-Object {
        if ($_ -match '^\s*([^#=\s]+)\s*=\s*(.*)\s*$') {
            $name = $matches[1].Trim()
            $valueRaw = $matches[2].Trim()
            $value = $valueRaw
            if ($value.StartsWith('"') -or $value.StartsWith("'")) {
                $quote = $value.Substring(0, 1)
                $end = $value.IndexOf($quote, 1)
                if ($end -gt 0) {
                    $value = $value.Substring(1, $end - 1)
                }
                else {
                    $value = $value.Trim($quote)
                }
            }
            else {
                $hash = $value.IndexOf('#')
                if ($hash -ge 0) {
                    $value = $value.Substring(0, $hash)
                }
                $value = $value.Trim()
            }
            $value = $value.Trim('"', "'", ' ')
            if ($name) {
                Set-Item -Path ("Env:{0}" -f $name) -Value $value
            }
        }
    }
    Write-Host "[OK] Chaves de API e Google Sheets injetadas com sucesso." -ForegroundColor Green
}
else {
    Write-Host "[ERRO] Arquivo .env não encontrado na raiz!" -ForegroundColor Red
    exit
}

# 1.5. HIGIENE LEVE DE ZUMBIS + EXPORTAÇÃO DE CAMINHOS DE SIDECARS
# Nota: este script roda VIA boot.ps1, portanto NAO podemos matar souls_mc,
# agentgateway, mcp_stdio_guard, soda_mcp_server, sequential-thinking-mcp ou leanctx
# (esses sao o coracao supervisionado e devem continuar vivos).
Write-Host "`n[+] Higiene de sidecars ETL orfaos + resolucao de binarios..." -ForegroundColor DarkGray
$etlZombies = @("opengrep", "biome", "ruff", "oxlint", "mcp-google")
foreach ($z in $etlZombies) {
    $exists = Get-Process -Name $z -ErrorAction SilentlyContinue
    if ($exists) {
        Write-Host ("[HIGIENE] Encerrando sidecar hung: {0} ({1} PIDs)" -f $z, $exists.Count) -ForegroundColor DarkYellow
        Stop-Process -Name $z -Force -ErrorAction SilentlyContinue
    }
}
# Resolve os caminhos absolutos dos sidecars para o fallback host-side (PRD-035).
$binDir = Join-Path $PSScriptRoot "bin"
$sidecarExports = @{
    "SODA_OPENGREP_BIN" = "opengrep*.exe"
    "SODA_BIOME_BIN"    = "biome*.exe"
    "SODA_RUFF_BIN"     = "ruff*.exe"
    "SODA_OXLINT_BIN"   = "oxlint*.exe"
}
foreach ($kv in $sidecarExports.GetEnumerator()) {
    $varName = $kv.Key
    $pattern = $kv.Value
    $resolved = Get-ChildItem -Path $binDir -Filter $pattern -ErrorAction SilentlyContinue |
    Where-Object { $_.Name -notlike "*.pdb" } |
    Select-Object -First 1
    if ($resolved) {
        $abs = $resolved.FullName
        Set-Item -Path ("Env:{0}" -f $varName) -Value $abs
        Write-Host ("[OK] {0} = {1}" -f $varName, $abs) -ForegroundColor Green
    }
    else {
        Write-Host ("[WARN] {0} nao encontrado em {1} (padrao: {2})" -f $varName, $binDir, $pattern) -ForegroundColor Yellow
    }
}

$cargoManifest = Join-Path $PSScriptRoot "Cargo.toml"
if (-not (Test-Path $cargoManifest)) {
    Write-Host "[ERRO] Cargo.toml não encontrado em: $cargoManifest" -ForegroundColor Red
    exit
}

$isDryRun = $false
if ($PSBoundParameters.ContainsKey('DryRun')) {
    $isDryRun = $true
}
else {
    $envDry = $env:SODA_DRY_RUN
    if ($envDry -and ($envDry -match '^(1|true|yes|y|sim|s)$')) {
        $isDryRun = $true
    }
    elseif (-not $PSBoundParameters.ContainsKey('Yes')) {
        $mode = Read-Host "Modo de execução: [ENTER] Normal  [2] Dry-run (1 rodada)"
        $isDryRun = ($mode -match '^\s*2\s*$')
    }
}

if ($isDryRun) {
    Write-Host "`nMODO: DRY-RUN ATIVO" -ForegroundColor Black -BackgroundColor Yellow
}
else {
    Write-Host "`nMODO: EXECUÇÃO NORMAL" -ForegroundColor Black -BackgroundColor DarkGray
}

# 2. O MENU DE MÁQUINA DE ESTADOS (DAG V5)
Write-Host "`nSELECIONE A ENGRENAGEM DE EXECUÇÃO:" -ForegroundColor Yellow
Write-Host "----------------------------------------------------------------" -ForegroundColor Cyan
Write-Host " [0] 👁️  N0 - Daemon Watcher (Cron Job)" -ForegroundColor White
Write-Host "             (Acorda o Olheiro Assíncrono para verificar novos links)"
Write-Host " [1] 🛡️  N1 - Guardião (Fase -1)" -ForegroundColor White
Write-Host "             (Prioriza NOVO_LINK_OK; depois roda o batch amplo) (Custo Zero)"
Write-Host " [2] 🛰️  N2 - Batedor FinOps (Fase -0.5) (IA Flash)" -ForegroundColor White
Write-Host "             (Busca README truncado + JSON Mode barato + Triagem Estruturada)"
Write-Host " [3] 📭 Outbox Sync (SQLite -> Sheets) - Carga Atômica Anti-429" -ForegroundColor White
Write-Host "             (Sincronização assíncrona do Outbox Pattern SQLite -> GSheets)"
Write-Host " [4] 🚜  N3 - Harvester Local (Fase 0)" -ForegroundColor White
Write-Host "             (Extração local O(1) para o SQLite Vault do RAW (Blobs)) (Custo Zero) (gatilho: APROVADO_PARA_HARVESTER)"
Write-Host " [5] 🧠  N4 - Motor Cloud Cognitivo (Fases 1, 2, 3 e 4) (IA Heavy)" -ForegroundColor White
Write-Host "             (Destilador + Enxame + Sintetizador + Injeção no GSheets) (gatilho: APROVADO_PARA_ENXAME)"
Write-Host " [6] 🤹🏻‍♀️  N5 - Revisão ETL Cognitivo Pesado (Fases 3 e 4) (IA Heavy)" -ForegroundColor White
Write-Host "             (Sintetizador + Escrita (Injeção) no GSheets) (gatilho: APROVADO_PARA_ENXAME)"
Write-Host " [7] 🔬  N6 - Deep Components Formatter (Fase 5)" -ForegroundColor White
Write-Host "             (Escreve a aba DEEP_COMPONENTS) (gatilho: APROVADO_DEEP_COMPONENTS_ANALYSIS)"
Write-Host " [C] 🌊 Cascata Automática (Esteira Contínua N0 -> N1 -> N2 -> Outbox)" -ForegroundColor White
Write-Host "             (Executa sequencialmente N0, N1, N2 e Outbox Sync de forma atômica)"
Write-Host " [X] 🛑  Abortar Ignição" -ForegroundColor Red
Write-Host "----------------------------------------------------------------" -ForegroundColor Cyan

$choice = $Choice
if (-not $choice) {
    $choice = Read-Host "`nArquiteto, informe a rota de voo"
}

$pipeline = @()

# 3. ROTEAMENTO DE COMANDOS RUST (HITL)
switch ($choice) {
    '0' {
        $binArgs = @()
        if ($isDryRun) { $binArgs = @("--dry-run") }
        $pipeline += @{ Bin = "n0_daemon_watcher"; Name = "DAEMON WATCHER"; Args = $binArgs }
    }
    '1' {
        $binArgs = @()
        if ($isDryRun) { $binArgs = @("--dry-run") }
        $pipeline += @{ Bin = "f_minus_1_guardian"; Name = "Fase -1 (GUARDIÃO)"; Args = $binArgs }
    }
    '2' {
        $binArgs = @()
        if ($isDryRun) { $binArgs = @("--dry-run") }
        $pipeline += @{ Bin = "f_minus_0_5_batedor_cli"; Name = "Fase -0.5 (BATEDOR FINOPS)"; Args = $binArgs }
    }
    '3' {
        $binArgs = @()
        if ($isDryRun) { $binArgs = @("--dry-run") }
        $pipeline += @{ Bin = "outbox_sync"; Name = "OUTBOX SYNC (SQLITE -> SHEETS)"; Args = $binArgs }
    }
    '4' {
        $effectiveRepo = $RepoId
        if ($effectiveRepo) {
            $binArgs = @("--repo", $effectiveRepo)
            $pName = "Fase 0 (HARVESTER LOCAL)"
        }
        else {
            $loteCustom = Read-Host "Informe o nome do Lote (Ex: LOTE_01_UX) ou deixe em branco para o padrao"
            $binArgs = @("--batch")
            if (-not [string]::IsNullOrWhiteSpace($loteCustom)) {
                $binArgs += "--lote-id"
                $binArgs += $loteCustom
            }
            $pName = "Fase 0 (HARVESTER LOCAL) [BATCH]"
        }
        $pipeline += @{ Bin = "f0_harvester_cli"; Name = $pName; Args = $binArgs }
    }
    '5' {
        $effectiveRepo = $RepoId
        if ($effectiveRepo) {
            $binArgs = @("--repo", $effectiveRepo, "--e2e-full", "--skip-harvester")
            $pName = "Fases 1 a 4 (MOTOR CLOUD COGNITIVO)"
        }
        else {
            $loteCustom = Read-Host "Informe o nome do Lote (Ex: LOTE_02_INFRA) ou deixe em branco para o padrao"
            $binArgs = @("--batch", "--e2e-full", "--skip-harvester")
            if (-not [string]::IsNullOrWhiteSpace($loteCustom)) {
                $binArgs += "--lote-id"
                $binArgs += $loteCustom
            }
            $pName = "Fases 1 a 4 (MOTOR CLOUD COGNITIVO) [BATCH Sheets]"
        }
        $pipeline += @{ Bin = "f3_synthesizer_cli"; Name = $pName; Args = $binArgs }
    }
    '6' {
        $effectiveRepo = $RepoId
        if ($effectiveRepo -and $effectiveRepo.Trim().ToUpperInvariant() -eq "BATCH") {
            $binArgs = @("--batch", "--resume-f3")
            $pName = "Fases 3 a 4 (REVISÃO ETL COGNITIVO PESADO) [BATCH RESUME_F3]"
        }
        elseif (-not $effectiveRepo) {
            $mode = Read-Host "BATCH (Enter) ou RepoId (owner/repo)"
            if ([string]::IsNullOrWhiteSpace($mode) -or $mode.Trim().ToUpperInvariant() -eq "BATCH") {
                $binArgs = @("--batch", "--resume-f3")
                $pName = "Fases 3 a 4 (REVISÃO ETL COGNITIVO PESADO) [BATCH RESUME_F3]"
            }
            else {
                $binArgs = @("--repo", $mode)
                $pName = "Fases 3 a 4 (REVISÃO ETL COGNITIVO PESADO)"
            }
        }
        else {
            $binArgs = @("--repo", $effectiveRepo)
            $pName = "Fases 3 a 4 (REVISÃO ETL COGNITIVO PESADO)"
        }
        $pipeline += @{ Bin = "f3_synthesizer_cli"; Name = $pName; Args = $binArgs }
    }
    '7' {
        $binArgs = @()
        if ($isDryRun) { $binArgs = @("--dry-run") }
        $pipeline += @{ Bin = "f5_deep_formatter_cli"; Name = "Fase 5 (DEEP COMPONENTS FORMATTER)"; Args = $binArgs }
    }
    'C' {
        $dry = if ($isDryRun) { @("--dry-run") } else { @() }
        $pipeline += @{ Bin = "n0_daemon_watcher"; Name = "CASCATA [1/4]: N0 (DAEMON WATCHER)"; Args = $dry }
        $pipeline += @{ Bin = "f_minus_1_guardian"; Name = "CASCATA [2/4]: N1 (GUARDIÃO)"; Args = $dry }
        $pipeline += @{ Bin = "f_minus_0_5_batedor_cli"; Name = "CASCATA [3/4]: N2 (BATEDOR FINOPS)"; Args = $dry }
        $pipeline += @{ Bin = "outbox_sync"; Name = "CASCATA [4/4]: OUTBOX SYNC"; Args = $dry }
    }
    'X' {
        Write-Host "`n🛑 Ignição abortada. O motor permanece em repouso." -ForegroundColor Yellow
        exit
    }
    default {
        Write-Host "`n❌ Comando inválido. Abortando." -ForegroundColor Red
        exit
    }
}

Push-Location $PSScriptRoot

try {
    $env:CARGO_INCREMENTAL = "0"

    foreach ($step in $pipeline) {
        $bin = $step.Bin
        $phaseName = $step.Name
        $binArgs = if ($step.Args) { $step.Args } else { @() }

        Write-Host "`n================================================================" -ForegroundColor Cyan
        Write-Host "`n🚀 DISPARANDO O MOTOR EM RUST: $phaseName (TOKIO EVENT LOOP)...`n" -ForegroundColor Red

        # ==== WRAPPER DE TRACKING (espelho do Invoke-TrackedProcess do boot.ps1) ====
        $etlLog = Join-Path $env:TEMP "soda_etl_cargo.out.log"
        $etlErr = Join-Path $env:TEMP "soda_etl_cargo.err.log"
        Remove-Item -LiteralPath $etlLog, $etlErr -Force -ErrorAction SilentlyContinue

        $cargoArgs = @("run", "--manifest-path", $cargoManifest, "--bin", $bin)
        if ($binArgs.Count -gt 0) {
            $cargoArgs += "--"
            $cargoArgs += $binArgs
        }

        Write-Host ("[PROC] LANCAMENTO: cargo {0}" -f ($cargoArgs -join ' ')) -ForegroundColor DarkCyan
        $proc = Start-Process `
            -FilePath "cargo" `
            -ArgumentList $cargoArgs `
            -WorkingDirectory $PSScriptRoot `
            -RedirectStandardOutput $etlLog `
            -RedirectStandardError $etlErr `
            -PassThru `
            -NoNewWindow
        $null = $proc.Handle  # materializa o handle para ExitCode confiavel

        $startedAt = Get-Date
        $lastBeat = $startedAt
        $HeartbeatSeconds = 30

        while (-not $proc.HasExited) {
            Start-Sleep -Seconds 5
            $now = Get-Date
            if (($now - $lastBeat).TotalSeconds -ge $HeartbeatSeconds) {
                $elapsed = [int](($now - $startedAt).TotalSeconds)
                Write-Host ("[ETL] {0} ainda rodando apos {1}s (heartbeat)..." -f $phaseName, $elapsed) -ForegroundColor DarkCyan
                foreach ($p in @($etlLog, $etlErr)) {
                    if (-not (Test-Path $p)) { continue }
                    $prefix = if ($p -eq $etlLog) { "[OUT]" } else { "[ERR]" }
                    $color = if ($p -eq $etlLog) { "DarkGray" } else { "Yellow" }
                    Get-Content -LiteralPath $p -Tail 5 -ErrorAction SilentlyContinue | ForEach-Object {
                        if ($_ -and $_.Trim()) {
                            Write-Host ("{0} {1}" -f $prefix, $_) -ForegroundColor $color
                        }
                    }
                }
                $lastBeat = $now
            }
        }

        $proc.WaitForExit()
        $proc.Refresh()
        $LASTEXITCODE = $proc.ExitCode
        if ($null -eq $LASTEXITCODE) {
            Write-Host "`n[FALHA LETAL] Nao foi possivel ler o Exit Code do Motor Rust." -ForegroundColor Red
            exit 1
        }
        if ($LASTEXITCODE -ne 0) {
            Write-Host "`n[FALHA LETAL] O Motor Rust ($phaseName) abortou com Exit Code $LASTEXITCODE." -ForegroundColor Red
            foreach ($p in @($etlLog, $etlErr)) {
                if (-not (Test-Path $p)) { continue }
                $prefix = if ($p -eq $etlLog) { "[OUT-FINAL]" } else { "[ERR-FINAL]" }
                $color = if ($p -eq $etlLog) { "DarkGray" } else { "Red" }
                Write-Host ("----- {0} -----" -f $p) -ForegroundColor $color
                Get-Content -LiteralPath $p -Tail 50 -ErrorAction SilentlyContinue | ForEach-Object {
                    if ($_ -and $_.Trim()) {
                        Write-Host ("{0} {1}" -f $prefix, $_) -ForegroundColor $color
                    }
                }
            }
            exit $LASTEXITCODE
        }
        else {
            Write-Host "`n[OK] Motor Rust ($phaseName) concluido com sucesso (Exit Code 0)." -ForegroundColor Green
            if (Test-Path $etlLog) {
                Write-Host "----- ULTIMAS LINHAS DO STDOUT -----" -ForegroundColor DarkGray
                Get-Content -LiteralPath $etlLog -Tail 10 -ErrorAction SilentlyContinue | ForEach-Object {
                    if ($_ -and $_.Trim()) {
                        Write-Host ("[OUT] {0}" -f $_) -ForegroundColor DarkGray
                    }
                }
            }
        }
    }
}
finally {
    Pop-Location
}
