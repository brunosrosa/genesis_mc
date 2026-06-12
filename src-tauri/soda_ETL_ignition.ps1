# ==============================================================================
# SODA ETL IGNITION MATRIX V5 - ORQUESTRADOR BARE-METAL (JANELA DE VIDRO)
# ==============================================================================

param(
    [string]$Choice = "",
    [switch]$DryRun,
    [switch]$Yes,
    [string]$RepoId = ""
)
[console]::InputEncoding = [console]::OutputEncoding = New-Object System.Text.UTF8Encoding
$PSStyle.OutputRendering = 'ANSI'
try { Clear-Host } catch {}

Write-Host "================================================================" -ForegroundColor Cyan
Write-Host " 🦅 SODA GENESIS MC - PAINEL DE IGNIÇÃO ETL V5 (JANELA DE VIDRO)" -ForegroundColor White -BackgroundColor DarkBlue
Write-Host "================================================================" -ForegroundColor Cyan

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
                } else {
                    $value = $value.Trim($quote)
                }
            } else {
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
} else {
    Write-Host "[ERRO] Arquivo .env não encontrado na raiz!" -ForegroundColor Red
    exit
}

$packagedBinDir = Join-Path $PSScriptRoot "bin"
$packagedExe = Join-Path $packagedBinDir "mcp-google.exe"
if (Test-Path $packagedExe) {
    $env:MCP_GOOGLE_WORKSPACE_BIN = $packagedExe
} else {
    $repoRoot = $rootResolved
    try {
        Push-Location $PSScriptRoot
        try {
            $gitRootCandidate = (git rev-parse --show-toplevel 2>$null | Select-Object -First 1)
            if ($gitRootCandidate) {
                $repoRoot = $gitRootCandidate.Trim()
            }
        } finally {
            Pop-Location
        }
    } catch {}
    $vendorRoot = Join-Path $PSScriptRoot "vendor"
    $vendorRepo = Join-Path $vendorRoot "mcp-google-workspace"
    $vendorRepoRelative = "src-tauri/vendor/mcp-google-workspace"
    if (-not (Test-Path $vendorRepo)) {
        New-Item -ItemType Directory -Path $vendorRoot -Force | Out-Null
        Write-Host "`n[+] Canibalizando mcp-google-workspace (Rust) para vendor/..." -ForegroundColor DarkGray
        $subrepoOk = $false
        try {
            $null = (git subrepo --version 2>$null)
            $subrepoOk = $true
        } catch {}
        if ($subrepoOk) {
            Push-Location $repoRoot
            try {
                git subrepo clone https://github.com/distrihub/mcp-google-workspace $vendorRepoRelative
            } finally {
                Pop-Location
            }
        } else {
            Push-Location $repoRoot
            try {
                git clone --depth 1 https://github.com/distrihub/mcp-google-workspace $vendorRepoRelative
            } finally {
                Pop-Location
            }
        }
    }

    $shadowRoot = Join-Path ([System.IO.Path]::GetTempPath()) ".souls_workspaces"
    New-Item -ItemType Directory -Path $shadowRoot -Force | Out-Null
    $shadowBuild = Join-Path $shadowRoot ("mcp-google-workspace-build_" + [guid]::NewGuid().ToString("N"))
    New-Item -ItemType Directory -Path $shadowBuild -Force | Out-Null

    Write-Host "`n[+] Build isolado (Shadow Workspace) do mcp-google-workspace..." -ForegroundColor DarkGray
    try {
        robocopy $vendorRepo $shadowBuild /MIR /XD ".git" "target" | Out-Null
        $env:CARGO_TARGET_DIR = Join-Path $shadowBuild "target"
        Push-Location $shadowBuild
        try {
            cargo build --release --locked --bin mcp-google
        } finally {
            Pop-Location
        }
        $builtExe = Join-Path $env:CARGO_TARGET_DIR "release\\mcp-google.exe"
        if (-not (Test-Path $builtExe)) {
            Write-Host "[ERRO] mcp-google.exe não foi gerado em: $builtExe" -ForegroundColor Red
            exit 1
        }
        New-Item -ItemType Directory -Path $packagedBinDir -Force | Out-Null
        Copy-Item -LiteralPath $builtExe -Destination $packagedExe -Force
        $env:MCP_GOOGLE_WORKSPACE_BIN = $packagedExe
        Write-Host "[OK] mcp-google-workspace empacotado em src-tauri/bin/." -ForegroundColor Green
    } finally {
        try { Remove-Item -LiteralPath $shadowBuild -Recurse -Force -ErrorAction SilentlyContinue } catch {}
        Remove-Item Env:CARGO_TARGET_DIR -ErrorAction SilentlyContinue
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
} else {
    $envDry = $env:SODA_DRY_RUN
    if ($envDry -and ($envDry -match '^(1|true|yes|y|sim|s)$')) {
        $isDryRun = $true
    } elseif (-not $PSBoundParameters.ContainsKey('Yes')) {
        $mode = Read-Host "Modo de execução: [ENTER] Normal  [2] Dry-run (1 rodada)"
        $isDryRun = ($mode -match '^\s*2\s*$')
    }
}

if ($isDryRun) {
    Write-Host "`nMODO: DRY-RUN ATIVO" -ForegroundColor Black -BackgroundColor Yellow
} else {
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
Write-Host " [3] 🚜  N3 - Harvester Local (Fase 0)" -ForegroundColor White
Write-Host "             (Extração local O(1) para o SQLite Vault do RAW (Blobs)) (Custo Zero) (gatilho: APROVADO_PARA_HARVESTER)"
Write-Host " [4] 🧠  N4 - Motor Cloud Cognitivo (Fases 1, 2, 3 e 4) (IA Heavy)" -ForegroundColor White
Write-Host "             (Destilador + Enxame + Sintetizador + Injeção no GSheets) (gatilho: APROVADO_PARA_ENXAME)"
Write-Host " [5] 🤹🏻‍♀️  N5 - Revisão ETL Cognitivo Pesado (Fases 3 e 4) (IA Heavy)" -ForegroundColor White
Write-Host "             (Sintetizador + Escrita (Injeção) no GSheets) (gatilho: APROVADO_PARA_ENXAME)"
Write-Host " [6] 🔬  N6 - Deep Components Formatter (Fase 5)" -ForegroundColor White
Write-Host "             (Escreve a aba DEEP_COMPONENTS) (gatilho: APROVADO_DEEP_COMPONENTS_ANALYSIS)"
Write-Host " [X] 🛑  Abortar Ignição" -ForegroundColor Red
Write-Host "----------------------------------------------------------------" -ForegroundColor Cyan

$choice = $Choice
if (-not $choice) {
    $choice = Read-Host "`nArquiteto, informe a rota de voo"
}

# 3. ROTEAMENTO DE COMANDOS RUST (HITL)
switch ($choice) {
    '0' {
        $bin = "n0_daemon_watcher"
        $binArgs = @()
        if ($isDryRun) { $binArgs = @("--dry-run") }
        $phaseName = "DAEMON WATCHER"
    }
    '1' {
        $bin = "f_minus_1_guardian"
        $binArgs = @()
        if ($isDryRun) { $binArgs = @("--dry-run") }
        $phaseName = "Fase -1 (GUARDIÃO)"
    }
    '2' {
        $bin = "f_minus_0_5_batedor_cli"
        $binArgs = @()
        if ($isDryRun) { $binArgs = @("--dry-run") }
        $phaseName = "Fase -0.5 (BATEDOR FINOPS)"
    }
    '3' {
        $bin = "f0_harvester_cli"
        $effectiveRepo = $RepoId
        if ($effectiveRepo) {
            $binArgs = @("--repo", $effectiveRepo)
            $phaseName = "Fase 0 (HARVESTER LOCAL)"
        } else {
            $loteCustom = Read-Host "Informe o nome do Lote (Ex: LOTE_01_UX) ou deixe em branco para o padrao"
            $binArgs = @("--batch")
            if (-not [string]::IsNullOrWhiteSpace($loteCustom)) {
                $binArgs += "--lote-id"
                $binArgs += $loteCustom
            }
            $phaseName = "Fase 0 (HARVESTER LOCAL) [BATCH]"
        }
    }
    '4' {
        $bin = "f3_synthesizer_cli"
        $effectiveRepo = $RepoId
        if ($effectiveRepo) {
            $binArgs = @("--repo", $effectiveRepo, "--e2e-full", "--skip-harvester")
            $phaseName = "Fases 1 a 4 (MOTOR CLOUD COGNITIVO)"
        } else {
            $loteCustom = Read-Host "Informe o nome do Lote (Ex: LOTE_02_INFRA) ou deixe em branco para o padrao"
            $binArgs = @("--batch", "--e2e-full", "--skip-harvester")
            if (-not [string]::IsNullOrWhiteSpace($loteCustom)) {
                $binArgs += "--lote-id"
                $binArgs += $loteCustom
            }
            $phaseName = "Fases 1 a 4 (MOTOR CLOUD COGNITIVO) [BATCH Sheets]"
        }
    }
    '5' {
        $bin = "f3_synthesizer_cli"
        $effectiveRepo = $RepoId
        if ($effectiveRepo -and $effectiveRepo.Trim().ToUpperInvariant() -eq "BATCH") {
            $binArgs = @("--batch", "--resume-f3")
            $phaseName = "Fases 3 a 4 (REVISÃO ETL COGNITIVO PESADO) [BATCH RESUME_F3]"
            break
        }
        if (-not $effectiveRepo) {
            $mode = Read-Host "BATCH (Enter) ou RepoId (owner/repo)"
            if ([string]::IsNullOrWhiteSpace($mode) -or $mode.Trim().ToUpperInvariant() -eq "BATCH") {
                $binArgs = @("--batch", "--resume-f3")
                $phaseName = "Fases 3 a 4 (REVISÃO ETL COGNITIVO PESADO) [BATCH RESUME_F3]"
                break
            }
            $effectiveRepo = $mode
        }
        $binArgs = @("--repo", $effectiveRepo)
        $phaseName = "Fases 3 a 4 (REVISÃO ETL COGNITIVO PESADO)"
    }
    '6' {
        $bin = "f5_deep_formatter_cli"
        $binArgs = @()
        if ($isDryRun) { $binArgs = @("--dry-run") }
        $phaseName = "Fase 5 (DEEP COMPONENTS FORMATTER)"
    }
    'X' {
        Write-Host "`n🛑 Ignição abortada. O motor permanece em repouso." -ForegroundColor Yellow
        exit
    }
    'x' {
        Write-Host "`n🛑 Ignição abortada. O motor permanece em repouso." -ForegroundColor Yellow
        exit
    }
    default {
        Write-Host "`n❌ Comando inválido. Abortando." -ForegroundColor Red
        exit
    }
}

Write-Host "`n================================================================" -ForegroundColor Cyan
Write-Host "`n🚀 DISPARANDO O MOTOR EM RUST (TOKIO EVENT LOOP)...`n" -ForegroundColor Red
Push-Location $PSScriptRoot

try {
    $env:CARGO_INCREMENTAL = "0"
    if ($binArgs.Count -gt 0) {
        & cargo run --manifest-path $cargoManifest --bin $bin -- @binArgs
    } else {
        & cargo run --manifest-path $cargoManifest --bin $bin
    }
    
    # Trava de Segurança do Exit Code
    if ($LASTEXITCODE -ne 0) {
        Write-Host "`n[FALHA LETAL] O Motor Rust abortou com Exit Code $LASTEXITCODE." -ForegroundColor Red
        exit $LASTEXITCODE
    }
    
} finally {
    Pop-Location
}
