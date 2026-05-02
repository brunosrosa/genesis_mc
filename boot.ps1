# ============================================================================
# SODA CANON V3: BOOTSTRAP DO AGENT GATEWAY
# Objetivo: Evitar corrupção de libuv (Node.js) e garantir injeção efêmera.
# ============================================================================

Write-Host "=======================================================" -ForegroundColor Cyan
Write-Host " SODA BOOTSTRAP: Inicializando AgentGateway " -ForegroundColor Cyan
Write-Host "=======================================================" -ForegroundColor Cyan

# 1. EXPURGO DE ZUMBIS (A Cura do libuv e portas presas)
Write-Host "`n[1/4] Expurgando processos órfãos (node, uvx, python, agentgateway)..." -ForegroundColor Yellow
$zombies = @("node", "uvx", "python", "agentgateway")
foreach ($z in $zombies) {
    Stop-Process -Name $z -Force -ErrorAction SilentlyContinue
}
Start-Sleep -Seconds 2 # Tempo estrito para o SO Windows liberar a porta TCP 3000
Write-Host "[OK] Memória higienizada e Porta 3000 liberada." -ForegroundColor Green

# 2. INJEÇÃO EFÊMERA DE AMBIENTE (Sua lógica de parser original e blindada)
Write-Host "`n[2/4] Injetando variáveis do .env na RAM da sessão..." -ForegroundColor Yellow
if (Test-Path ".env") {
    Get-Content .env | Where-Object { $_ -match '^[^#]' -and $_.Trim() -ne '' } | Foreach-Object {
        $name, $value = $_.Split('=', 2)
        
        # Limpa os nomes e os valores corretamente
        $cleanName = $name.Trim()
        $cleanValue = $value.Trim().Trim('"', "'")
        
        # Injeta na sessão do terminal
        Set-Content "env:$cleanName" $cleanValue
    }
    Write-Host "[OK] Segredos injetados com segurança via Set-Content." -ForegroundColor Green
}
else {
    Write-Host "[!] Arquivo .env não encontrado na raiz." -ForegroundColor Red
}

# 3. WARM-UP DELAY (Prevenção de Timeout no Cliente MCP)
Write-Host "`n[3/4] Preparando Ignição e Aquecimento de Sockets..." -ForegroundColor Yellow
Write-Host "ATENÇÃO: Após o servidor subir, AGUARDE 5 SEGUNDOS antes de conectar o Antigravity IDE!" -ForegroundColor Red
Start-Sleep -Seconds 3

# 4. IGNIÇÃO DO GATEWAY (Seu comando original e testado)
Write-Host "`n[4/4] Iniciando AgentGateway na porta 3000..." -ForegroundColor Cyan
agentgateway.exe -f gateway-config.yaml