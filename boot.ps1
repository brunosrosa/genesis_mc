# Lê o arquivo .env, ignora comentários e injeta as variáveis na RAM do terminal
Get-Content .env | Where-Object { $_ -match '^[^#]' -and $_.Trim() -ne '' } | Foreach-Object {
    $name, $value = $_.Split('=', 2)
    
    # Limpa os nomes e os valores corretamente
    $cleanName = $name.Trim()
    $cleanValue = $value.Trim().Trim('"', "'")
    
    # Injeta na sessão do terminal
    Set-Content "env:$cleanName" $cleanValue
}

Write-Host "🚀 Iniciando AgentGateway na porta 3000..." -ForegroundColor Green

# Executa o roteador
agentgateway.exe -f gateway-config.yaml