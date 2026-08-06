# souls_deps_snapshot.ps1
# Script determinístico para sanear e documentar o estado de dependências do SOULS.

$scriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$rootDir = (Get-Item "$scriptDir/../..").FullName

$manifestPath = Join-Path $rootDir "src-tauri/Cargo.toml"
$targetDir = Join-Path $rootDir "docs/observability/context_dumps/crates"
$cargoStatePath = Join-Path $targetDir "_CARGO_TOML_STATE.txt"
$duplicateDepsPath = Join-Path $targetDir "_DUPLICATE_DEPS.txt"

# Cria pasta de estado se não existir
if (-not (Test-Path $targetDir)) {
    New-Item -ItemType Directory -Force -Path $targetDir | Out-Null
}

# 1. Copiar manifest para o estado
if (Test-Path $manifestPath) {
    Copy-Item -Path $manifestPath -Destination $cargoStatePath -Force
}
else {
    Write-Error "Cargo.toml não encontrado em $manifestPath"
    exit 1
}

# 2. Rodar cargo tree -d e gerar relatório de duplicatas
$srcTauriDir = Join-Path $rootDir "src-tauri"
Push-Location $srcTauriDir
try {
    $duplicates = cargo tree -d 2>$null
    if ([string]::IsNullOrWhiteSpace($duplicates)) {
        "Nenhuma dependência duplicada encontrada." | Out-File -FilePath $duplicateDepsPath -Encoding utf8 -Force
    }
    else {
        $duplicates | Out-File -FilePath $duplicateDepsPath -Encoding utf8 -Force
    }
}
finally {
    Pop-Location
}

# 3. Output de sucesso
Write-Host "[OK] Snapshot de dependências atualizado com sucesso em $targetDir"

