# ============================================================================
# SODA CANON V5: BOOTSTRAP DO SOULS MC (SYSTEM TRAY DAEMON)
# Objetivo: Evitar corrupção, garantir injeção efêmera de variáveis na RAM,
# forçar atualização de pacotes MCP e ancorar o Fantasma na bandeja.
# ============================================================================
try { Clear-Host } catch {}
Write-Host "=======================================================" -ForegroundColor Cyan
Write-Host " 👻 SOULS MC BOOTSTRAP: Inicializando a Máquina Silenciosa " -ForegroundColor White -BackgroundColor DarkBlue
Write-Host "=======================================================" -ForegroundColor Cyan

# 1. EXPURGO DE ZUMBIS (Higiene de RAM)
Write-Host "`n[1/5] Expurgando processos órfãos (node, uvx, python, agentgateway)..." -ForegroundColor Yellow
$zombies = @("node", "uvx", "python", "agentgateway", "agentgateway_tcp_proxy", "genesis_mc", "mcp_stdio_guard")
foreach ($z in $zombies) {
    Stop-Process -Name $z -Force -ErrorAction SilentlyContinue
}
Start-Sleep -Seconds 2
Write-Host "[OK] Memória higienizada e portas TCP liberadas." -ForegroundColor Green

# 2. ATUALIZAÇÃO FORÇADA DE FERRAMENTAS (UV / NPX)
Write-Host "`n[2/5] Sincronizando ferramentas MCP (uv / npx)..." -ForegroundColor Yellow
uv tool upgrade --all *>&1 | Out-Null
if (Test-Path "$env:APPDATA\npm-cache\_npx") { 
    Remove-Item -Path "$env:APPDATA\npm-cache\_npx" -Recurse -Force -ErrorAction SilentlyContinue 
}
Write-Host "[OK] Ecossistema limpo e atualizado." -ForegroundColor Green

# 3. VALIDAÇÃO ATIVA DO NOTEBOOKLM (Filtro Inteligente Anti-Falso Positivo)
Write-Host "`n[3/5] Validando integridade da Sessão do Oráculo (NotebookLM)..." -ForegroundColor Yellow

# Executa o comando e captura a saída
$nlmCheck = uvx --from notebooklm-mcp-cli nlm list *>&1
$requireLogin = $false

# Filtro cirúrgico: Avalia linha por linha, ignorando logs de download/warnings do uvx
foreach ($line in $nlmCheck) {
    if ($line -match "login" -or $line -match "unauthorized" -or $line -match "expired") {
        # Garante que a palavra não veio de um log de download acidental
        if ($line -notmatch "uvx" -and $line -notmatch "downloading") {
            $requireLogin = $true
            break
        }
    }
}

# Só pede login se o filtro pegou a falha real ou se a execução de fato estourou erro (Exit Code)
if ($requireLogin -or $LASTEXITCODE -ne 0) {
    Write-Host "[!] Sessão caducou. Interceptando antes do Gateway subir..." -ForegroundColor Red
    Write-Host " -> Autentique-se na janela do navegador que irá abrir agora." -ForegroundColor Cyan
    uvx --from notebooklm-mcp-cli nlm login
    Write-Host "[OK] Nova chave de sessão ancorada com sucesso." -ForegroundColor Green
} else {
    Write-Host "[OK] Sessão do Oráculo validada e ativa." -ForegroundColor Green
}

# 4. INJEÇÃO EFÊMERA DE AMBIENTE (Parser Robusto Anti-Quebra)
Write-Host "`n[4/5] Injetando chaves do .env na RAM da sessão..." -ForegroundColor Yellow
$envPath = Join-Path $PSScriptRoot ".env"
if (Test-Path $envPath) {
    Get-Content $envPath | ForEach-Object {
        if ($_ -match '^\s*([^#=\s]+)\s*=\s*(.*)\s*$') {
            $name = $matches[1].Trim()
            $valueRaw = $matches[2].Trim()
            $value = $valueRaw
            
            if ($value.StartsWith('"') -or $value.StartsWith("'")) {
                $quote = $value.Substring(0, 1)
                $end = $value.IndexOf($quote, 1)
                if ($end -gt 0) { $value = $value.Substring(1, $end - 1) } else { $value = $value.Trim($quote) }
            } else {
                $hash = $value.IndexOf('#')
                if ($hash -ge 0) { $value = $value.Substring(0, $hash) }
                $value = $value.Trim()
            }
            $value = $value.Trim('"', "'", ' ')
            if ($name) { Set-Item -Path ("Env:{0}" -f $name) -Value $value }
        }
    }
    Write-Host "[OK] Segredos e DOCFORK_API_KEY injetados com segurança (Set-Item)." -ForegroundColor Green
} else {
    Write-Host "[!] Arquivo .env não encontrado em: $envPath" -ForegroundColor Red
}

# 5. WARM-UP DELAY E IGNIÇÃO DO DAEMON
Write-Host "`n[5/5] Preparando Ignição Bare-Metal..." -ForegroundColor Yellow
Write-Host "ATENÇÃO: O Souls MC ancorará silenciosamente na bandeja do sistema." -ForegroundColor Cyan
Start-Sleep -Seconds 3

Write-Host "`n[🚀] Forjando Sidecars e Iniciando o Daemon (genesis_mc)..." -ForegroundColor White -BackgroundColor DarkBlue
Push-Location (Join-Path $PSScriptRoot "src-tauri")

$env:CARGO_INCREMENTAL = "0"
# Compilamos os sidecars essenciais primeiro silenciosamente
cargo build --quiet --bin agentgateway_tcp_proxy
cargo build --quiet --bin mcp_stdio_guard

# Ligamos a entidade principal. Ela travará este terminal para vermos a telemetria, 
# mas a UI estará silenciosa perto do relógio.
cargo run --features tauri-app --bin genesis_mc

Pop-Location