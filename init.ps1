# init.ps1 - Script de Inicialização do Ambiente SODA (Windows PowerShell)
Write-Host "Iniciando sequencia de boot do Genesis MC (SODA)..." -ForegroundColor Cyan

# 1. Carregar variáveis do .env para a sessão atual (sem gravar no registro do Windows)
if (Test-Path .env) {
    Write-Host "Carregando chaves do .env..." -ForegroundColor Green
    Get-Content .env | Where-Object { $_ -match '^[^#]' -and $_ -match '=' } | ForEach-Object {
        $name, $value = $_.Split('=', 2)
        $value = $value -replace '^"|"$', ''
        [Environment]::SetEnvironmentVariable($name.Trim(), $value.Trim(), "Process")
    }
}
else {
    Write-Host "AVISO: Arquivo .env nao encontrado!" -ForegroundColor Yellow
}

# 2. Iniciar Docker Desktop (caso os sidecars efêmeros precisem rodar)
Write-Host "Verificando Docker Engine (Sidecars MCP)..." -ForegroundColor Green
$dockerState = Get-Service -Name com.docker.service -ErrorAction SilentlyContinue
if ($dockerState -and $dockerState.Status -ne 'Running') {
    Start-Service -Name com.docker.service
    Write-Host "Aguardando inicializacao do Docker..."
    Start-Sleep -Seconds 5
}

# 3. Lançar o Antigravity IDE no diretório atual
Write-Host "Injetando ambiente no Antigravity IDE..." -ForegroundColor Green
antigravity .