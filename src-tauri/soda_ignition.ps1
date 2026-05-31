# ==============================================================================
# SODA IGNITION MATRIX V5 - ORQUESTRADOR BARE-METAL (JANELA DE VIDRO)
# ==============================================================================

param(
    [string]$Choice = "",
    [switch]$DryRun,
    [switch]$Yes,
    [string]$RepoId = "aaif-goose/goose"
)
try { Clear-Host } catch {}

Write-Host "================================================================" -ForegroundColor Cyan
Write-Host " 🦅 SODA GENESIS MC - PAINEL DE IGNIÇÃO V5 (JANELA DE VIDRO)" -ForegroundColor White -BackgroundColor DarkBlue
Write-Host "================================================================" -ForegroundColor Cyan

# 1. BLINDAGEM DE AMBIENTE: CARREGA O .ENV PARA A RAM
Write-Host "`n[+] Calibrando Reator: Injetando chaves do .env na memória..." -ForegroundColor DarkGray
$envPath = Join-Path (Resolve-Path (Join-Path $PSScriptRoot "..")) ".env"
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

$cargoManifest = Join-Path $PSScriptRoot "Cargo.toml"
if (-not (Test-Path $cargoManifest)) {
    Write-Host "[ERRO] Cargo.toml não encontrado em: $cargoManifest" -ForegroundColor Red
    exit
}

# 2. O MENU DE MÁQUINA DE ESTADOS (DAG V5)
Write-Host "`nSELECIONE A ENGRENAGEM DE EXECUÇÃO:" -ForegroundColor Yellow
Write-Host "----------------------------------------------------------------" -ForegroundColor Cyan
Write-Host " [0] 👁️  N0 - Daemon Watcher (Cron Job)" -ForegroundColor White
Write-Host "             (Acorda o Olheiro Assíncrono para verificar novos links)"
Write-Host " [1] 🛡️  N1 - Guardião (Fase -1)" -ForegroundColor White
Write-Host "             (Puxa nomes oficiais e versões do GitHub via Idempotência) (Custo Zero)"
Write-Host " [2] 🛰️  N2 - Batedor FinOps (Fase -0.5) (IA Flash)" -ForegroundColor White
Write-Host "             (Busca README truncado + JSON Mode barato + Triagem Estruturada)"
Write-Host " [3] 🚜  N3 - Harvester Local (Fase 0)" -ForegroundColor White
Write-Host "             (Extração local O(1) para o SQLite Vault do RAW (Blobs)) (Custo Zero)"
Write-Host " [4] 🧠  N4 - Motor Cloud Cognitivo (Fases 1, 2, 3 e 4) (IA Heavy)" -ForegroundColor White
Write-Host "             (Destilador de Essências + Enxame IAs + Sintetizador + Injeção no GSheets)"
Write-Host " [5] 🤹🏻‍♀️  N5 - Revisão ETL Cognitivo Pesado (Fases 3 e 4) (IA Heavy)" -ForegroundColor White
Write-Host "             (Sintetizador com Pydantic + Escrita (Injeção) no GSheets)"
Write-Host " [6] 🔬  N6 - Motor de Decomposição (Fase 5 - DEEP COMPONENTS) (ESQUELETO)" -ForegroundColor White
Write-Host "             (Fatia ferramentas CORE em subcomponentes na aba Deep - ESQUELETO)"
Write-Host " [X] 🛑  Abortar Ignição" -ForegroundColor Red
Write-Host "----------------------------------------------------------------" -ForegroundColor Cyan

$choice = $Choice
if (-not $choice) {
    $choice = Read-Host "`nArquiteto, informe a rota de voo"
}

$isDryRun = $false
if ($PSBoundParameters.ContainsKey('DryRun')) {
    $isDryRun = $true
} elseif ($PSBoundParameters.ContainsKey('Yes') -and $choice) {
    $isDryRun = $false
} else {
    $dryRunInput = Read-Host "🧪 Ativar dry-run (1 rodada, sem loop infinito)? (S/N)"
    $isDryRun = ($dryRunInput -match '^[sS]$')
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
        $binArgs = @("--repo", $RepoId)
        $phaseName = "Fase 0 (HARVESTER LOCAL)"
    }
    '4' {
        $bin = "f3_synthesizer_cli"
        $binArgs = @("--repo", $RepoId, "--e2e-full", "--skip-harvester")
        $phaseName = "Fases 1 a 4 (MOTOR CLOUD COGNITIVO)"
    }
    '5' {
        $bin = "f3_synthesizer_cli"
        $binArgs = @("--repo", $RepoId)
        $phaseName = "Fases 3 a 4 (REVISÃO ETL COGNITIVO PESADO)"
    }
    '6' {
        Write-Host "`n🔬 N6 (Fase 5) ainda é ESQUELETO. Nenhuma ação foi executada." -ForegroundColor Yellow
        exit
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
$confirmation = "N"
if ($PSBoundParameters.ContainsKey('Yes')) {
    $confirmation = "S"
} else {
    $confirmation = Read-Host "🔥 Autoriza a ativação do motor para [$phaseName]? (S/N)"
}

if ($confirmation -match "^[sS]$") {
    Write-Host "`n🚀 DISPARANDO O MOTOR EM RUST (TOKIO EVENT LOOP)...`n" -ForegroundColor Red
    Push-Location $PSScriptRoot
    try {
        $env:CARGO_INCREMENTAL = "0"
        if ($binArgs.Count -gt 0) {
            & cargo run --manifest-path $cargoManifest --bin $bin -- @binArgs
        } else {
            & cargo run --manifest-path $cargoManifest --bin $bin
        }
    } finally {
        Pop-Location
    }
} else {
    Write-Host "`n🛑 Execução cancelada pelo Arquiteto (HITL)." -ForegroundColor Yellow
}
