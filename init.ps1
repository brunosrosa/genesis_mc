# init.ps1 - Script de Inicialização do Ambiente SODA (Windows PowerShell)
Write-Host "Iniciando sequencia de boot do Genesis MC (SODA)..." -ForegroundColor Cyan

# 1. Carregar variáveis do .env para a sessão atual
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

# 2. Patch de Segurança para o Roteador MCP Vault (Quebra de API FastMCP)
Write-Host "Injetando blindagem ambiental para o FastMCP..." -ForegroundColor Green
[Environment]::SetEnvironmentVariable("FASTMCP_LOG_LEVEL", "DEBUG", "Process")

# 3. Iniciar Docker Engine e Sidecars Efêmeros
Write-Host "Verificando Docker Engine..." -ForegroundColor Green
$dockerState = Get-Service -Name com.docker.service -ErrorAction SilentlyContinue
if ($dockerState -and $dockerState.Status -ne 'Running') {
    Start-Service -Name com.docker.service
    Write-Host "Aguardando inicializacao do Docker..."
    Start-Sleep -Seconds 5
}

# 4. Levantar Sidecar do Browser-Use (Necessário para a rota 'docker exec' do Vault)
$sidecarPath = "C:\Users\rosas\Dev_Projects\soda-browser-sidecar"
if (Test-Path $sidecarPath) {
    Write-Host "Orquestrando contêineres Sidecar em background..." -ForegroundColor Green
    Set-Location -Path $sidecarPath
    docker-compose up -d
    Set-Location -Path "C:\Users\rosas\Dev_Projects\genesis_mc"
}
else {
    Write-Host "AVISO: Diretorio do Sidecar do Browser nao encontrado em $sidecarPath" -ForegroundColor Yellow
}

# 5. Lançar o Antigravity IDE no diretório raiz
Write-Host "Injetando ambiente no Antigravity IDE..." -ForegroundColor Green
antigravity .