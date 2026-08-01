# ============================================================================
# SOULS CANON V5: BOOTSTRAP DO SOULS MC (SYSTEM TRAY DAEMON)
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
$env:RUST_LOG = "souls_mc_lib=info,souls_sast=debug,souls_harvester=debug,headroom_engine=debug,llama_engine=info,hardware_profiler=info,model_manager=debug,souls_ccr=debug,ignore=warn,globset=warn,walkdir=warn"
$env:SOULS_CCR_MAX_RAM_MB = "256"
$env:SOULS_HEADROOM_SAFETY_MARGIN = "512"
$env:SOULS_HEADROOM_OUTPUT_BUFFER = "4096"
# SOULS FinOps (fix/cargo-finops-v1): sccache persiste artefatos rustc entre branches
# e sobrevive a `cargo clean`. Cache vive em Z: (ReFS Dev Drive 80GB) com 8GB de budget.
# Se sccache nao estiver instalado, .cargo/config.toml[build] rustc-wrapper apenas
# e ignorado (cargo emite warning mas nao quebra).
$env:SCCACHE_DIR = "Z:\.sccache"
$env:SCCACHE_CACHE_SIZE = "8G"
$env:RUSTC_WRAPPER = "sccache"
# Reforca paralelismo (defesa em profundidade: .cargo/config.toml[build] jobs=8).
$env:CARGO_BUILD_JOBS = "8"
# Patch idempotente vendor/llama-cpp-sys-2: GGML_CCACHE=ON (default upstream) faz
# cmake wrappear `sccache nvcc`, que falha com "fatbinary: Could not open input
# file '*.ptx'". Mantemos OFF para destravar CUDA. Re-aplicado em todo boot pois
# `cargo update` pode reverter o patch. Tolerante a patch ausente.
$vendorLlamaCmake = Join-Path $PSScriptRoot "src-tauri\vendor\llama-cpp-sys-2\llama.cpp\ggml\CMakeLists.txt"
if (Test-Path $vendorLlamaCmake) {
    $content = Get-Content -LiteralPath $vendorLlamaCmake -Raw -ErrorAction SilentlyContinue
    if ($content -and $content -match 'option\(GGML_CCACHE "ggml: use ccache if available"\s+ON\)') {
        (Get-Content -LiteralPath $vendorLlamaCmake) -replace `
            'option\(GGML_CCACHE "ggml: use ccache if available"\s+ON\)',
            'option(GGML_CCACHE "ggml: use ccache if available"                   OFF)' |
            Set-Content -LiteralPath $vendorLlamaCmake
        Write-Host "[PATCH] vendor/llama-cpp-sys-2 GGML_CCACHE -> OFF (auto-fix CUDA+sccache)" -ForegroundColor DarkGreen
    }
}
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
    $zombies = @("agentgateway", "agentgateway_tcp_proxy", "souls_mc", "mcp_stdio_guard", "souls_mcp_server", "sequential-thinking-server", "sequential-thinking-mcp", "biome", "opengrep", "oxlint")
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
                "--features", "tauri-app,gateway_ccr,llama_backend",
                "--bin", "souls_mcp_server",
                "--bin", "agentgateway_tcp_proxy",
                "--bin", "mcp_stdio_guard",
                "--bin", "scan_local_models_cli",
                "--bin", "souls_ephemeral_infer_cli",
                "--bin", "souls_mc",
                "--locked"
            ) `
            -Label "cargo-build-supervisores" `
            -WorkingDirectory $srcTauriDir

        # 4.5. VARREDURA DE MODELOS LOCAIS (Fase 1.5 Model Manager Sync)
        Write-Host "`n[4.5] Sincronizando inventario de modelos locais no SQLite Vault..." -ForegroundColor Yellow
        $scannerPath = Join-Path $srcTauriDir "target\debug\scan_local_models_cli.exe"
        if (Test-Path $scannerPath) {
            Invoke-TrackedProcess `
                -FilePath $scannerPath `
                -Arguments @("C:\Users\rosas\.lmstudio\models") `
                -Label "scan-local-models" `
                -WorkingDirectory $PSScriptRoot
        } else {
            Write-BootWarn "Binario scan_local_models_cli.exe nao encontrado em $scannerPath"
        }

        # 4.6. COMPILAÇÃO DE CONTEXT DUMPS (Exporta inventario de modelos para TXT)
        Write-Host "`n[4.6] Compilando dumps de contexto (_MODELS_INVENTORY.txt)..." -ForegroundColor Yellow
        $dumpsCompilerScript = Join-Path $PSScriptRoot "docs\scripts\souls_context_dumps_compiler.py"
        $pyCmd = if (Get-Command "python" -ErrorAction SilentlyContinue) { "python" } elseif (Test-Path "$env:LOCALAPPDATA\Programs\Python\Python312\python.exe") { "$env:LOCALAPPDATA\Programs\Python\Python312\python.exe" } else { "py" }
        if (Test-Path $dumpsCompilerScript) {
            Invoke-TrackedProcess `
                -FilePath $pyCmd `
                -Arguments @($dumpsCompilerScript) `
                -Label "compile-context-dumps" `
                -WorkingDirectory $PSScriptRoot
        } else {
            Write-BootWarn "Script souls_context_dumps_compiler.py nao encontrado em $dumpsCompilerScript"
        }

        # 5. IGNIÇÃO DO DAEMON SUPERVISOR COMPILADO (souls_mc)
        Write-Host "`n[5/5] Iniciando o daemon supervisor compilado (souls_mc)..." -ForegroundColor Yellow
        $daemonPath = Join-Path $srcTauriDir "target\debug\souls_mc.exe"
        if (-not (Test-Path $daemonPath)) {
            throw "Binario esperado nao encontrado apos a build: $daemonPath"
        }

        Write-BootOk "Build finalizada. Daemon (supervisor do agentgateway + proxy) sera iniciado."
        $daemonProc = Start-Process -FilePath $daemonPath -WorkingDirectory $PSScriptRoot -NoNewWindow -PassThru
        Write-Host ("[DAEMON] souls_mc iniciado (PID: {0})" -f $daemonProc.Id) -ForegroundColor DarkCyan
        Start-Sleep -Seconds 2
        $stillAlive = Get-Process -Id $daemonProc.Id -ErrorAction SilentlyContinue
        if ($null -eq $stillAlive) {
            throw "souls_mc (PID $daemonProc.Id) morreu em menos de 2s apos o start. Verifique logs do daemon."
        } else {
            Write-Host ("[DAEMON] souls_mc estavel (PID: {0}, WorkingSet: {1:N1} MB)" -f $daemonProc.Id, ($stillAlive.WorkingSet64 / 1MB)) -ForegroundColor Green
        }

        # 7. TRAVA DE PRONTIDÃO (Probe TCP na Porta 3000)
        Write-Host "`n[PROBE] Testando estabilidade do gateway MCP em http://127.0.0.1:3000/..." -ForegroundColor Yellow
        $ready = $false
        $attempts = 0
        while (-not $ready -and $attempts -lt 15) {
            $attempts++
            try {
                $tcp = New-Object System.Net.Sockets.TcpClient
                $tcp.Connect("127.0.0.1", 3000)
                if ($tcp.Connected) {
                    $tcp.Close()
                    $ready = $true
                }
            } catch {
                Start-Sleep -Milliseconds 500
            }
        }

        if ($ready) {
            Write-Host "`n=======================================================" -ForegroundColor Green
            Write-Host " 🚀 SOULS MC ONLINE & PRONTO! (Porta 3000 -> OK)" -ForegroundColor Green
            Write-Host " Pode dar o reload no client MCP da IDE agora!" -ForegroundColor Cyan
            Write-Host "=======================================================\n" -ForegroundColor Green
        } else {
            Write-BootWarn "O daemon iniciou, mas a porta 3000 demorou a responder ao probe."
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
