# ============================================================================
# SODA CANON V5: BOOTSTRAP DO SOULS MC (SYSTEM TRAY DAEMON)
# Objetivo: Evitar corrupção, garantir injeção efêmera de variáveis na RAM
# e ancorar o Fantasma na bandeja sem validações lentas de ferramentas ETL.
# ============================================================================
try { Clear-Host } catch {}
Write-Host "=======================================================" -ForegroundColor Cyan
Write-Host " 👻 SOULS MC BOOTSTRAP: Inicializando a Máquina Silenciosa " -ForegroundColor White -BackgroundColor DarkBlue
Write-Host "=======================================================" -ForegroundColor Cyan

# 1. EXPURGO DE ZUMBIS (Higiene de RAM)
Write-Host "`n[1/4] Expurgando processos órfãos (node, uvx, python, agentgateway)..." -ForegroundColor Yellow
$zombies = @("node", "uvx", "python", "agentgateway", "agentgateway_tcp_proxy", "genesis_mc", "mcp_stdio_guard")
foreach ($z in $zombies) {
    Stop-Process -Name $z -Force -ErrorAction SilentlyContinue
}
Start-Sleep -Seconds 2
Write-Host "[OK] Memória higienizada e portas TCP liberadas." -ForegroundColor Green

# 2. LIMPEZA LEVE DE CACHE TRANSIENTE
Write-Host "`n[2/4] Limpando caches transitórios da sessão..." -ForegroundColor Yellow
if (Test-Path "$env:APPDATA\npm-cache\_npx") { 
    Remove-Item -Path "$env:APPDATA\npm-cache\_npx" -Recurse -Force -ErrorAction SilentlyContinue 
}
Write-Host "[OK] Cache transitório higienizado." -ForegroundColor Green

# 3. INJEÇÃO EFÊMERA DE AMBIENTE (Parser Robusto Anti-Quebra)
Write-Host "`n[3/4] Injetando chaves do .env na RAM da sessão..." -ForegroundColor Yellow
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
    Write-Host "[OK] Segredos injetados com segurança (Set-Item)." -ForegroundColor Green
} else {
    Write-Host "[!] Arquivo .env não encontrado em: $envPath" -ForegroundColor Red
}

# 4. WARM-UP DELAY E IGNIÇÃO DO DAEMON
Write-Host "`n[4/4] Preparando Ignição Bare-Metal..." -ForegroundColor Yellow
Write-Host "ATENÇÃO: O Souls MC ancorará silenciosamente na bandeja do sistema." -ForegroundColor Cyan
Start-Sleep -Seconds 3

Write-Host "`n[🚀] Forjando Sidecars e Iniciando o Daemon (genesis_mc)..." -ForegroundColor White -BackgroundColor DarkBlue
Push-Location (Join-Path $PSScriptRoot "src-tauri")

$env:CARGO_INCREMENTAL = "0"

function Invoke-CargoChecked {
    param(
        [Parameter(Mandatory = $true)]
        [string[]]$Args
    )

    & cargo @Args
    if ($LASTEXITCODE -ne 0) {
        throw "Falha ao executar: cargo $($Args -join ' ')"
    }
}

# Compilamos os sidecars supervisionados exigidos pela UI antes da ignição do daemon.
Invoke-CargoChecked -Args @("build", "--quiet", "--bin", "soda_mcp_server")
Invoke-CargoChecked -Args @("build", "--quiet", "--bin", "agentgateway_tcp_proxy")
Invoke-CargoChecked -Args @("build", "--quiet", "--bin", "mcp_stdio_guard")

# Ligamos a entidade principal. Ela travará este terminal para vermos a telemetria, 
# mas a UI estará silenciosa perto do relógio.
Invoke-CargoChecked -Args @("run", "--features", "tauri-app", "--bin", "genesis_mc")

Pop-Location
