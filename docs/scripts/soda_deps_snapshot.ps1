# soda_deps_snapshot.ps1
# Script determinístico para sanear e documentar o estado de dependências do SODA.

$manifestPath = "src-tauri/Cargo.toml"
$cargoStatePath = "docs/audits/crates/_CARGO_TOML_STATE.txt"
$duplicateDepsPath = "docs/audits/crates/_DUPLICATE_DEPS.txt"

# Cria pasta de estado se não existir
$stateDir = Split-Path -Parent $cargoStatePath
if (-not (Test-Path $stateDir)) {
    New-Item -ItemType Directory -Force -Path $stateDir | Out-Null
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
Push-Location "src-tauri"
try {
    $duplicates = cargo tree -d 2>$null
    if ([string]::IsNullOrWhiteSpace($duplicates)) {
        "Nenhuma dependência duplicada encontrada." | Out-File -FilePath "../$duplicateDepsPath" -Encoding utf8 -Force
    }
    else {
        $duplicates | Out-File -FilePath "../$duplicateDepsPath" -Encoding utf8 -Force
    }
}
finally {
    Pop-Location
}

# 3. Output de sucesso
Write-Host "[OK] Snapshot de dependências atualizado com sucesso."
