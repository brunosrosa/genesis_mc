# ============================================================================
# SODA CANON V5: BOOTSTRAP DO SOULS MC (SYSTEM TRAY DAEMON)
# Objetivo: Evitar corrupção, garantir injeção efêmera de variáveis na RAM
# e ancorar o Fantasma na bandeja sem validações lentas de ferramentas ETL.
# ============================================================================
try {
    [console]::InputEncoding = [console]::OutputEncoding = New-Object System.Text.UTF8Encoding
} catch {}
if ($null -ne $PSStyle) {
    $PSStyle.OutputRendering = 'ANSI'
}
try { Clear-Host } catch {}
# Filtro cirurgico: preserva debug do core, silencia o ruido do `ignore`/`globset`/`walkdir`
# que polui logs com ~600 linhas de "built glob set" durante o build.
# Pos-B: crate agora chama souls_mc_lib (renomeada em B).
$env:RUST_LOG = "souls_mc_lib=info,soda_sast=debug,soda_harvester=debug,ignore=warn,globset=warn,walkdir=warn"
$ErrorActionPreference = "Stop"
$ProgressPreference = "SilentlyContinue"

function Write-BootOk {
    param([string]$Message)
    Write-Host "[OK] $Message" -ForegroundColor Green
}

function Write-BootWarn {
    param([string]$Message)
    Write-Host "[WARN] $Message" -ForegroundColor Yellow
}

function Assert-CommandAvailable {
    param([Parameter(Mandatory = $true)][string]$Command)
    if (-not (Get-Command $Command -ErrorAction SilentlyContinue)) {
        throw "Dependência ausente no PATH: $Command"
    }
}

function Invoke-TrackedProcess {
    param(
        [Parameter(Mandatory = $true)][string]$FilePath,
        [Parameter(Mandatory = $true)][string[]]$Arguments,
        [Parameter(Mandatory = $true)][string]$Label,
        [Parameter(Mandatory = $true)][string]$WorkingDirectory,
        [int]$HeartbeatSeconds = 20
    )

    $safeLabel = ($Label -replace '[^a-zA-Z0-9_-]', '_')
    $stdoutPath = Join-Path $env:TEMP ("souls_boot_{0}_stdout.log" -f $safeLabel)
    $stderrPath = Join-Path $env:TEMP ("souls_boot_{0}_stderr.log" -f $safeLabel)
    Remove-Item -LiteralPath $stdoutPath, $stderrPath -Force -ErrorAction SilentlyContinue

    Write-Host ("[PROC] {0}: {1} {2}" -f $Label, $FilePath, ($Arguments -join ' ')) -ForegroundColor DarkCyan
    $process = Start-Process `
        -FilePath $FilePath `
        -ArgumentList $Arguments `
        -WorkingDirectory $WorkingDirectory `
        -RedirectStandardOutput $stdoutPath `
        -RedirectStandardError $stderrPath `
        -PassThru `
        -NoNewWindow
    # Garante que o handle nativo seja materializado para leitura confiavel do ExitCode.
    $null = $process.Handle

    $startedAt = Get-Date
    $lastHeartbeat = $startedAt

    while (-not $process.HasExited) {
        Start-Sleep -Seconds 1
        $now = Get-Date
        if (($now - $lastHeartbeat).TotalSeconds -ge $HeartbeatSeconds) {
            $elapsed = [int](($now - $startedAt).TotalSeconds)
            Write-Host ("[PROC] {0} ainda vivo apos {1}s..." -f $Label, $elapsed) -ForegroundColor DarkCyan

            foreach ($path in @($stdoutPath, $stderrPath)) {
                if (-not (Test-Path $path)) {
                    continue
                }
                $prefix = "[LOG]"
                $color = if ($path -eq $stdoutPath) { "DarkGray" } else { "Yellow" }
                Get-Content -LiteralPath $path -Tail 3 -ErrorAction SilentlyContinue | ForEach-Object {
                    if ($_ -and $_.Trim()) {
                        Write-Host ("{0} {1}" -f $prefix, $_) -ForegroundColor $color
                    }
                }
            }

            $lastHeartbeat = $now
        }
    }

    $process.WaitForExit()
    $process.Refresh()
    $exitCode = $process.ExitCode
    if ($null -eq $exitCode) {
        throw "Falha ao ler o exit code de $Label apos o termino do processo."
    }
    if ($exitCode -ne 0) {
        foreach ($path in @($stdoutPath, $stderrPath)) {
            if (-not (Test-Path $path)) {
                continue
            }
            $prefix = if ($path -eq $stdoutPath) { "[OUT]" } else { "[ERR]" }
            $color = if ($path -eq $stdoutPath) { "DarkGray" } else { "Red" }
            Get-Content -LiteralPath $path -Tail 30 -ErrorAction SilentlyContinue | ForEach-Object {
                if ($_ -and $_.Trim()) {
                    Write-Host ("{0} {1}" -f $prefix, $_) -ForegroundColor $color
                }
            }
        }
        throw "Falha ao executar $Label (exit code $exitCode)."
    }

    Write-BootOk ("{0} concluido (exit code {1})." -f $Label, $exitCode)
    Remove-Item -LiteralPath $stdoutPath, $stderrPath -Force -ErrorAction SilentlyContinue
}

Write-Host "=======================================================" -ForegroundColor Cyan
Write-Host " 👻 SOULS MC BOOTSTRAP: Inicializando a Maquina Silenciosa " -ForegroundColor White -BackgroundColor DarkBlue
Write-Host "=======================================================" -ForegroundColor Cyan

try {
    # 1. EXPURGO DE ZUMBIS (Higiene de RAM)
    Write-Host "`n[1/5] Expurgando processos supervisionados do ecossistema Souls..." -ForegroundColor Yellow
    $zombies = @("agentgateway", "agentgateway_tcp_proxy", "souls_mc", "mcp_stdio_guard", "soda_mcp_server", "sequential-thinking-mcp", "leanctx", "biome", "opengrep", "oxlint")
    $killed = @()
    foreach ($z in $zombies) {
        $existing = Get-Process -Name $z -ErrorAction SilentlyContinue
        if ($existing) {
            $killed += [PSCustomObject]@{ Name = $z; Pids = ($existing.Id -join ",") }
            Stop-Process -Name $z -Force -ErrorAction SilentlyContinue
        }
    }
    Start-Sleep -Seconds 1
    if ($killed.Count -gt 0) {
        foreach ($k in $killed) {
            Write-Host ("[HIGIENE] Encerrado: {0} (PIDs: {1})" -f $k.Name, $k.Pids) -ForegroundColor DarkYellow
        }
    } else {
        Write-Host "[HIGIENE] Nenhum zumbi encontrado, sessoes anteriores limpas." -ForegroundColor DarkGray
    }
    Write-BootOk "Supervisores antigos encerrados e portas locais liberadas."

    # 2. HIGIENE LEVE SEM DESTRUIR CACHE DO MCP REMOTO
    Write-Host "`n[2/5] Validando premissas da sessao..." -ForegroundColor Yellow
    Write-BootWarn "O cache do npx sera preservado para nao rebaixar o bootstrap do mcp-remote."
    Assert-CommandAvailable -Command "cargo"
    Assert-CommandAvailable -Command "agentgateway.exe"
    Write-BootOk "Dependencias essenciais resolvidas (cargo, agentgateway.exe)."

    # 3. INJEÇÃO EFÊMERA DE AMBIENTE (Parser Robusto Anti-Quebra)
    Write-Host "`n[3/5] Injetando chaves do .env na RAM da sessao..." -ForegroundColor Yellow
    $envPath = Join-Path $PSScriptRoot ".env"
    if (Test-Path $envPath) {
        $injectedKeys = @()
        $skippedKeys = @()
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
                if ($name) {
                    Set-Item -Path ("Env:{0}" -f $name) -Value $value
                    if ($value.Length -gt 0) { $injectedKeys += $name } else { $skippedKeys += $name }
                }
            }
        }
        Write-Host ("[ENV] Injetadas: {0}" -f ($injectedKeys -join ", ")) -ForegroundColor DarkCyan
        if ($skippedKeys.Count -gt 0) {
            Write-Host ("[ENV] Vazias (skip): {0}" -f ($skippedKeys -join ", ")) -ForegroundColor DarkYellow
        }
        Write-BootOk "Segredos injetados com seguranca (Set-Item)."
    } else {
        throw "Arquivo .env nao encontrado em: $envPath"
    }

    # 4. BUILD OBSERVÁVEL DOS BINÁRIOS SUPERVISIONADOS
    Write-Host "`n[4/5] Forjando binarios supervisionados com telemetria viva..." -ForegroundColor Yellow
    Write-Host "ATENCAO: heartbeats surgirao a cada 20s se o compilador ficar silencioso." -ForegroundColor Cyan
    $srcTauriDir = Join-Path $PSScriptRoot "src-tauri"
    Push-Location $srcTauriDir
    try {
        $env:CARGO_INCREMENTAL = "0"
        Invoke-TrackedProcess `
            -FilePath "cargo" `
            -Arguments @(
                "build",
                "--message-format", "short",
                "--features", "tauri-app",
                "--bin", "soda_mcp_server",
                "--bin", "agentgateway_tcp_proxy",
                "--bin", "mcp_stdio_guard",
                "--bin", "souls_mc"
            ) `
            -Label "cargo-build-supervisores" `
            -WorkingDirectory $srcTauriDir

        Invoke-TrackedProcess `
            -FilePath "cargo" `
            -Arguments @(
                "build",
                "--message-format", "short",
                "--manifest-path", "third_party/lean-ctx/Cargo.toml",
                "--bin", "lean-ctx",
                "--target-dir", "target"
            ) `
            -Label "cargo-build-lean-ctx" `
            -WorkingDirectory $srcTauriDir

        # 5. PRE-AQUECIMENTO DE CACHE EM BACKGROUND
        Write-Host "`n[5/5] Pre-aquecendo o cache do lean-ctx em background..." -ForegroundColor Yellow
        $leanCtxPath = Join-Path $srcTauriDir "target\debug\lean-ctx.exe"
        if (Test-Path $leanCtxPath) {
            $leanProc = Start-Process -FilePath $leanCtxPath -ArgumentList "graph", "build" -WorkingDirectory $PSScriptRoot -NoNewWindow -PassThru -ErrorAction SilentlyContinue
            if ($leanProc -and $leanProc.Id) {
                Write-Host ("[BG] lean-ctx iniciado em background (PID: {0})" -f $leanProc.Id) -ForegroundColor DarkCyan
            } else {
                Write-BootWarn "lean-ctx nao conseguiu iniciar em background (verifique permissoes ou binario)."
            }
        } else {
            Write-BootWarn "Binario lean-ctx.exe nao encontrado em $leanCtxPath - pre-aquecimento pulado."
        }

        # 6. IGNIÇÃO DO DAEMON JÁ COMPILADO
        Write-Host "`n[6/6] Iniciando o daemon compilado (souls_mc)..." -ForegroundColor Yellow
        $daemonPath = Join-Path $srcTauriDir "target\debug\souls_mc.exe"
        if (-not (Test-Path $daemonPath)) {
            throw "Binario esperado nao encontrado apos a build: $daemonPath"
        }

        Write-BootOk "Build finalizada. Daemon sera iniciado sem passar de novo pelo cargo run."
        $daemonProc = Start-Process -FilePath $daemonPath -WorkingDirectory $PSScriptRoot -NoNewWindow -PassThru
        Write-Host ("[DAEMON] souls_mc iniciado (PID: {0})" -f $daemonProc.Id) -ForegroundColor DarkCyan
        # Daemon deve continuar vivo; se cair em <2s, algo esta muito errado.
        Start-Sleep -Seconds 2
        $stillAlive = Get-Process -Id $daemonProc.Id -ErrorAction SilentlyContinue
        if ($null -eq $stillAlive) {
            throw "souls_mc (PID $daemonProc.Id) morreu em menos de 2s apos o start. Verifique logs do daemon."
        } else {
            Write-Host ("[DAEMON] souls_mc estavel apos 2s (PID: {0}, WorkingSet: {1:N1} MB)" -f $daemonProc.Id, ($stillAlive.WorkingSet64 / 1MB)) -ForegroundColor Green
        }
    }
    finally {
        Pop-Location
    }
}
catch {
    Write-Host ("[ERR] Bootstrap falhou: {0}" -f $_.Exception.Message) -ForegroundColor Red
    exit 1
}
