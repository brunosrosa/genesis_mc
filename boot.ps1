# ============================================================================
# SODA CANON V3: BOOTSTRAP DO AGENT GATEWAY
# Objetivo: Evitar corrupção de libuv (Node.js), garantir injeção efêmera,
# forçar atualização de pacotes MCP e validar a sessão do NotebookLM.
# ============================================================================

Write-Host "=======================================================" -ForegroundColor Cyan
Write-Host " SODA BOOTSTRAP: Inicializando AgentGateway " -ForegroundColor Cyan
Write-Host "=======================================================" -ForegroundColor Cyan

# 1. EXPURGO DE ZUMBIS (A Cura do libuv e portas presas)
Write-Host "`n[1/5] Expurgando processos órfãos (node, uvx, python, agentgateway)..." -ForegroundColor Yellow
$zombies = @("node", "uvx", "python", "agentgateway")
foreach ($z in $zombies) {
    Stop-Process -Name $z -Force -ErrorAction SilentlyContinue
}
Start-Sleep -Seconds 2 # Tempo estrito para o SO Windows liberar a porta TCP 3000
Write-Host "[OK] Memória higienizada e Porta 3000 liberada." -ForegroundColor Green

# 2. ATUALIZAÇÃO FORÇADA DE FERRAMENTAS (UV / NPX)
Write-Host "`n[2/5] Sincronizando ferramentas MCP para as versões mais recentes..." -ForegroundColor Yellow
Write-Host " -> Executando upgrade nas tools globais do UV..." -ForegroundColor DarkGray
uv tool upgrade --all *>&1 | Out-Null
Write-Host " -> Limpando cache do NPX para forçar pull na execução..." -ForegroundColor DarkGray
# Remove o cache do npx silenciosamente para garantir que 'npx -y' baixe a versão mais atual
if (Test-Path "$env:APPDATA\npm-cache\_npx") { Remove-Item -Path "$env:APPDATA\npm-cache\_npx" -Recurse -Force -ErrorAction SilentlyContinue }
Write-Host "[OK] Ecossistema limpo e pronto para instanciar as últimas versões." -ForegroundColor Green

# 3. VALIDAÇÃO ATIVA DO NOTEBOOKLM (ANTI-FAIL-CLOSED)
Write-Host "`n[3/5] Validando integridade dos cookies do Oráculo (NotebookLM)..." -ForegroundColor Yellow
# O script tenta listar os cadernos passivamente. Se o cookie expirou, ele joga erro.
$nlmCheck = uvx --from notebooklm-mcp-cli nlm list *>&1

if ($nlmCheck -match "login" -or $nlmCheck -match "unauthorized" -or $nlmCheck -match "Error" -or $LASTEXITCODE -ne 0) {
    Write-Host "[!] Sessão caducou. Interceptando antes do Gateway subir..." -ForegroundColor Red
    Write-Host " -> Por favor, autentique-se na janela do navegador que irá abrir agora." -ForegroundColor Cyan
    
    # Chama o processo de login isoladamente e espera o usuário
    uvx --from notebooklm-mcp-cli nlm login
    
    Write-Host "[OK] Nova chave de sessão ancorada com sucesso no ambiente local." -ForegroundColor Green
}
else {
    Write-Host "[OK] Sessão do Oráculo (NotebookLM) validada e ativa." -ForegroundColor Green
}

# 4. INJEÇÃO EFÊMERA DE AMBIENTE (Sua lógica de parser original e blindada)
Write-Host "`n[4/5] Injetando variáveis do .env na RAM da sessão..." -ForegroundColor Yellow
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

# 5. WARM-UP DELAY E IGNIÇÃO
Write-Host "`n[5/5] Preparando Ignição e Aquecimento de Sockets..." -ForegroundColor Yellow
Write-Host "ATENÇÃO: Após o servidor subir, AGUARDE 5 SEGUNDOS antes de conectar o Antigravity IDE!" -ForegroundColor Red
Start-Sleep -Seconds 3

Write-Host "`n[🚀] Iniciando AgentGateway na porta 3000..." -ForegroundColor Cyan
agentgateway.exe -f gateway-config.yaml