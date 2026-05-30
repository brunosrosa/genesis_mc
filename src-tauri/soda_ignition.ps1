# ==============================================================================
# SODA IGNITION MATRIX V5 - ORQUESTRADOR BARE-METAL (JANELA DE VIDRO)
# ==============================================================================

param(
    [ValidateSet('0','1','2','3','X','x')]
    [string]$Choice,
    [switch]$DryRun,
    [switch]$Yes
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
            $value = $matches[2].Trim()
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
Write-Host " [0] 👁️  N0 - Daemon Watcher" -ForegroundColor White
Write-Host "        (Acorda o Olheiro Assíncrono para varrer a planilha 24/7)"
Write-Host " [1] 🛡️  N1 - Guardião (Fase -1)" -ForegroundColor White
Write-Host "        (Puxa nomes oficiais e versões do GitHub a custo zero)"
Write-Host " [2] 🧱  Refresh Blob10 (Canon Context)" -ForegroundColor White
Write-Host "        (Regera o blob_10_soda_canon_context no SQLite Vault)"
Write-Host " [3] 🚜  N3/N4 - Trator ETL Cognitivo (Fases 0 a 4)" -ForegroundColor White
Write-Host "        (Harvester O(1) + Enxame Cognitivo + Pydantic SGR)"
Write-Host " [X] 🛑  Abortar Ignição" -ForegroundColor Red
Write-Host "----------------------------------------------------------------" -ForegroundColor Cyan

$choice = $Choice
if (-not $choice) {
    $choice = Read-Host "`nArquiteto, informe a rota de voo"
}

$isDryRun = $false
if ($PSBoundParameters.ContainsKey('DryRun')) {
    $isDryRun = $true
} else {
    $dryRun = Read-Host "🧪 Ativar dry-run (1 rodada, sem loop infinito)? (S/N)"
    $isDryRun = ($dryRun -match '^[sS]$')
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
        $bin = "refresh_blob10_cli"
        $binArgs = @()
        $phaseName = "Refresh Blob10 (Canon Context)"
    }
    '3' {
        $bin = "f3_synthesizer_cli"
        $binArgs = @("--e2e-full")
        $phaseName = "Fase 0 a 4 (ETL COGNITIVO PESADO)"
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
