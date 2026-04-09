# Lê o arquivo .env ignorando comentários (linhas que começam com #) e linhas em branco
Get-Content .env | Where-Object { $_ -match '^[^#]' -and $_.Trim() -ne '' } | Foreach-Object {
    $name, $value = $_.Split('=', 2)
    # Limpa espaços em branco e injeta na sessão
    Set-Content "env:$name.Trim()" $value.Trim()
}

# Injeta os caminhos essenciais no PATH temporário desta sessão
$env:PATH = "$env:SODA_UV_PATH;$env:SODA_CARGO_PATH;" + $env:PATH

Write-Host "🚀 Iniciando AgentGateway com PATH turbinado..." -ForegroundColor Green

# Executa o Gateway usando o PATH global (sem o .\)
agentgateway.exe -f gateway-config.yaml