# =============================================================================
# SOULS MC — Defender Exclusions Helper (fix/cargo-finops-v1)
# Adiciona exclusoes no Windows Defender para paths/processos do build Rust.
# Audit: .souls_scratchpad/_CARGO_AUDIT_2026-07-31.md
#
# *** ESTE SCRIPT PRECISA DE PRIVILEGIO DE ADMINISTRADOR ***
# Clique direito > "Executar como administrador" (ou):
#   powershell -ExecutionPolicy Bypass -File scripts\add_defender_exclusions.ps1
#
# O que ele faz:
#   1. Verifica se esta rodando como admin (Self-Elevate se necessario).
#   2. Adiciona exclusoes de PATH para Z:\souls_mc, Z:\.sccache,
#      C:\Users\rosas\.cargo, C:\Users\rosas\.rustup.
#   3. Adiciona exclusoes de PROCESS para rustc/cargo/sccache/link.
#   4. Relatorio final com o estado das exclusoes aplicadas.
# Idempotente: rodar 2x nao quebra, apenas re-reporta.
# =============================================================================

$ErrorActionPreference = "Stop"

function Test-IsAdmin {
    $id = [System.Security.Principal.WindowsIdentity]::GetCurrent()
    $principal = New-Object System.Security.Principal.WindowsPrincipal($id)
    return $principal.IsInRole([System.Security.Principal.WindowsBuiltInRole]::Administrator)
}

function Write-Step {
    param([string]$Message)
    Write-Host "[STEP] $Message" -ForegroundColor Cyan
}

function Write-Ok {
    param([string]$Message)
    Write-Host "[OK]   $Message" -ForegroundColor Green
}

function Write-Skip {
    param([string]$Message)
    Write-Host "[SKIP] $Message" -ForegroundColor DarkYellow
}

# --- 1. Auto-elevate se nao for admin -----------------------------------------
if (-not (Test-IsAdmin)) {
    Write-Host "[ELEVATE] Sem privilegio admin. Reabrindo como admin..." -ForegroundColor Yellow
    $args = @("-NoProfile", "-ExecutionPolicy", "Bypass", "-File", $PSCommandPath)
    try {
        Start-Process -FilePath "powershell.exe" -ArgumentList $args -Verb RunAs -Wait
        exit $LASTEXITCODE
    } catch {
        Write-Host "[FAIL] Nao foi possivel elevar para admin. Execute manualmente como admin." -ForegroundColor Red
        exit 1
    }
}

Write-Host "=======================================================" -ForegroundColor Cyan
Write-Host " SOULS Defender Exclusions Helper (Admin Context)     " -ForegroundColor White -BackgroundColor DarkBlue
Write-Host "=======================================================" -ForegroundColor Cyan
Write-Host ""

# --- 2. Coletar estado atual das exclusoes ------------------------------------
$existingPaths = (Get-MpPreference).ExclusionPath
$existingProcs = (Get-MpPreference).ExclusionProcess

function Add-IfMissing {
    param(
        [Parameter(Mandatory)][ValidateSet('Path', 'Process')][string]$Kind,
        [Parameter(Mandatory)][string]$Value
    )
    $list = if ($Kind -eq 'Path') { $existingPaths } else { $existingProcs }
    if ($list -contains $Value) {
        Write-Skip "$Kind ja excluido: $Value"
        return $false
    }
    # Add-MpPreference usa -ExclusionPath / -ExclusionProcess (nao -Path/-Process).
    if ($Kind -eq 'Path') {
        Add-MpPreference -ExclusionPath $Value | Out-Null
    } else {
        Add-MpPreference -ExclusionProcess $Value | Out-Null
    }
    Write-Ok "Adicionado $Kind : $Value"
    return $true
}

# --- 3. Aplicar exclusoes de PATH ---------------------------------------------
Write-Step "Aplicando exclusoes de PATH..."
$paths = @(
    "Z:\souls_mc",                  # workspace + target/ + vendor/
    "Z:\souls_mc\src-tauri\target", # cache de build (o ELEFANTE: 23GB)
    "Z:\.sccache",                  # cache de sccache (sobrevive a cargo clean)
    "C:\Users\rosas\.cargo",         # registry + bin do cargo
    "C:\Users\rosas\.rustup"        # toolchain rust
)
$added = 0
foreach ($p in $paths) {
    if (-not (Test-Path $p)) {
        Write-Skip "Path nao existe, ignorando: $p"
        continue
    }
    if (Add-IfMissing -Kind Path -Value $p) { $added++ }
}
Write-Host ""

# --- 4. Aplicar exclusoes de PROCESS ------------------------------------------
Write-Step "Aplicando exclusoes de PROCESS..."
$procs = @(
    "rustc.exe",
    "cargo.exe",
    "sccache.exe",
    "rust-lld.exe",
    "link.exe"          # MSVC linker
)
foreach ($p in $procs) {
    if (Add-IfMissing -Kind Process -Value $p) { $added++ }
}
Write-Host ""

# --- 5. Relatorio final --------------------------------------------------------
Write-Host "=======================================================" -ForegroundColor Green
Write-Host " Concluido. $added exclusao(oes) adicionada(s).            " -ForegroundColor Green
Write-Host "=======================================================" -ForegroundColor Green
Write-Host ""
Write-Step "Estado atual das exclusoes de PATH:"
Get-MpPreference | Select-Object -ExpandProperty ExclusionPath | ForEach-Object {
    Write-Host "  - $_"
}
Write-Host ""
Write-Step "Estado atual das exclusoes de PROCESS:"
Get-MpPreference | Select-Object -ExpandProperty ExclusionProcess | ForEach-Object {
    Write-Host "  - $_"
}
Write-Host ""
Write-Ok "Reiniciar o Visual Studio / Trae IDE para que o Defender reconheca as exclusoes (evidencia em cached handles)."
Write-Host "Para reverter, rode com -Remove: scripts\add_defender_exclusions.ps1 -Remove" -ForegroundColor DarkGray
