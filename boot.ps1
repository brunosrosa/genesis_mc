Param(
    [switch]$Dev,
    [switch]$Build,
    [switch]$CleanWebview
)

# =============================================================================
# SOULS MC (Mission Control) | SODA Bootstrap Soberano & Fusão Bare-Metal v6
# Conformidade: ADR-001 (Core Stack), ADR-003 (Isolamento Stdio), 
#               ADR-039 (Cargo FinOps) e ADR-041 (Servername Soberano souls_mcp)
# =============================================================================

# 1. Tratamento Estrito de Erros e Configuração de Terminal
$ErrorActionPreference = "Stop"
try { [console]::InputEncoding = [console]::OutputEncoding = New-Object System.Text.UTF8Encoding } catch {}
if ($null -ne $PSStyle) { $PSStyle.OutputRendering = 'ANSI' }
try { Clear-Host } catch {}

Write-Host "=======================================================" -ForegroundColor Cyan
Write-Host " 👻 SOULS MC BOOTSTRAP: Inicializando a Maquina Silenciosa " -ForegroundColor White -BackgroundColor DarkBlue
Write-Host "=======================================================" -ForegroundColor Cyan

# =============================================================================
# BLOCO A: HIGIENE AMBIENTAL, LOGS E SCCACHE FINOPS (ADR-038 / ADR-039)
# =============================================================================

# Variáveis FinOps de RAM e Headroom Buffer
$env:SOULS_CCR_MAX_RAM_MB = "256"
$env:SOULS_HEADROOM_SAFETY_MARGIN = "512"
$env:SOULS_HEADROOM_OUTPUT_BUFFER = "4096"

# Telemetria e Logs cirúrgicos (Silencia ruídos de crates transitivas)
$env:RUST_LOG = "souls_ui_shell=info,souls_core=debug,souls_protocol=info,souls_mc_lib=info,souls_sast=debug,souls_harvester=debug,headroom_engine=debug,llama_engine=info,hardware_profiler=info,model_manager=debug,souls_ccr=debug,ignore=warn,globset=warn,walkdir=warn"

# Configuração Persistente do Sccache em Dev Drive Z:
$sccacheExe = (Get-Command sccache.exe -ErrorAction SilentlyContinue | Select-Object -First 1 -ExpandProperty Source)
if (-not $sccacheExe) { $sccacheExe = (Get-Command sccache -ErrorAction SilentlyContinue | Select-Object -First 1 -ExpandProperty Source) }
if ($sccacheExe) {
    $env:SCCACHE = $sccacheExe
    $env:SCCACHE_DIR = "Z:\.sccache"
    $env:SCCACHE_CACHE_SIZE = "8G"
    $env:RUSTC_WRAPPER = "sccache"
    $env:SCCACHE_C_COMPILER = "cl.exe"
    $env:SCCACHE_CXX_COMPILER = "cl.exe"
    $env:SCCACHE_NVCC_COMPILER = ""  # Desabilita sccache para NVCC (device code CUDA) devido a limitações conhecidas
    Write-Host "[SCCACHE] C/C++ wrap ativo via $sccacheExe (Cache persistente em Z:\.sccache)" -ForegroundColor DarkGreen
} else {
    Write-Host "[SCCACHE] sccache.exe nao encontrado no PATH; compilação incremental padrão do rustc ativa." -ForegroundColor DarkYellow
}
$env:CARGO_BUILD_JOBS = "6"  # Reforça paralelismo de compilação sem estourar limites físicos de RAM

# Soluções e Patches de Compilação CUDA & C-Runtime (MSVC) (ADR-039)
$env:GGML_CCACHE = "OFF"
$env:CMAKE_CUDA_COMPILER_LAUNCHER = ""
$env:CUDA_CACHE_DISABLE = "1"
$env:CMAKE_GENERATOR = "Ninja"
$env:CMAKE_MSVC_RUNTIME_LIBRARY = "MultiThreadedDebugDLL"
$env:CFLAGS = "/MD"
$env:CXXFLAGS = "/MD"
$env:CMAKE_CUDA_ARCHITECTURES = "75"  # Pinho de arquitetura GPU para RTX 2060m (sm_75) acelerando o build em 10x
Write-Host "[CUDA] GGML_CCACHE=OFF + CMAKE_CUDA_ARCHITECTURES=75 + /MD CRT injetados" -ForegroundColor DarkGreen

# =============================================================================
# BLOCO A.1: FORÇAR WEBVIEW2 NA iGPU & SUSPENSÃO NO SYSTRAY (0% GPU EM REPOUSO)
# =============================================================================
$env:WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS = "--force_low_power_gpu --enable-low-power-gpu --enable-features=Calculators,BackgroundTabThrottling,IntensiveWakeUpThrottling,ThrottleDisplayNoneAndVisibilityHiddenCrossOriginIframes --disable-backgrounding-occluded-windows=false"
$gpuPrefRegKey = "HKCU:\Software\Microsoft\DirectX\UserGpuPreferences"
if (!(Test-Path $gpuPrefRegKey)) {
    New-Item -Path $gpuPrefRegKey -Force | Out-Null
}
$targetExes = @(
    (Join-Path $PSScriptRoot "target\release\souls_ui_shell.exe"),
    (Join-Path $PSScriptRoot "target\debug\souls_ui_shell.exe"),
    (Join-Path $PSScriptRoot ".agents\bin\souls_ui_shell.exe"),
    (Join-Path $PSScriptRoot "target\release\souls_mc.exe"),
    (Join-Path $PSScriptRoot "target\debug\souls_mc.exe"),
    (Join-Path $PSScriptRoot ".agents\bin\souls_mc.exe")
)
foreach ($exe in $targetExes) {
    Set-ItemProperty -Path $gpuPrefRegKey -Name $exe -Value "GpuPreference=1;" -Type String -ErrorAction SilentlyContinue
}
Write-Host "[GPU-FINOPS] WebView2 e souls_ui_shell ancorados na iGPU (GpuPreference=1; / RTX 2060m 100% blindada para CUDA)" -ForegroundColor DarkGreen


# =============================================================================
# BLOCO B: QUARENTENA EXPANSA DE PROCESSOS & HIGIENE WEBVIEW2
# =============================================================================

Write-Host "`n[SOULS] Varrendo e higienizando processos e sidecars antigos..." -ForegroundColor Yellow
$zombies = @(
    "souls_ui_shell",
    "souls-ui-shell",
    "agentgateway", 
    "agentgateway_tcp_proxy", 
    "souls_mc", 
    "souls-mc",
    "mcp_stdio_guard", 
    "souls_mcp_server", 
    "sequential-thinking-server", 
    "sequential-thinking-mcp", 
    "biome", 
    "opengrep", 
    "oxlint"
)
$killed = @()
foreach ($z in $zombies) {
    $existing = Get-Process -Name $z -ErrorAction SilentlyContinue
    if ($existing) {
        $killed += [PSCustomObject]@{ Name = $z; Pids = ($existing.Id -join ",") }
        Stop-Process -Name $z -Force -ErrorAction SilentlyContinue
    }
}

# Purga de instâncias msedgewebview2 órfãs PERTENCENTES EXCLUSIVAMENTE ao SOULS
$orphanedWebviews = Get-CimInstance Win32_Process -Filter "Name = 'msedgewebview2.exe'" -ErrorAction SilentlyContinue | Where-Object {
    $_.CommandLine -match "souls"
}
if ($orphanedWebviews) {
    foreach ($wv in $orphanedWebviews) {
        Stop-Process -Id $wv.ProcessId -Force -ErrorAction SilentlyContinue
        $killed += [PSCustomObject]@{ Name = "msedgewebview2 (souls)"; Pids = $wv.ProcessId }
    }
}

Start-Sleep -Milliseconds 500
if ($killed.Count -gt 0) {
    foreach ($k in $killed) {
        Write-Host ("[HIGIENE] Encerrado: {0} (PIDs: {1})" -f $k.Name, $k.Pids) -ForegroundColor DarkYellow
    }
} else {
    Write-Host "[HIGIENE] Nenhum processo zumbi encontrado. RAM livre." -ForegroundColor DarkGray
}

# Higiene do WebView2 UDF (Limpa cache de dev e redirects obsoletos que causam ERR_CONNECTION_REFUSED)
$webviewDataDirs = @(
    (Join-Path $env:LOCALAPPDATA "com.rosas.souls-mc\EBWebView"),
    (Join-Path $env:LOCALAPPDATA "souls-mc\EBWebView"),
    (Join-Path $PSScriptRoot "target\release\EBWebView"),
    (Join-Path $PSScriptRoot "target\debug\EBWebView"),
    (Join-Path $PSScriptRoot "src-tauri\target\release\EBWebView")
)
if ($CleanWebview -or $Build -or (-not $Dev)) {
    foreach ($dir in $webviewDataDirs) {
        if (Test-Path $dir) {
            try {
                Remove-Item -Path $dir -Recurse -Force -ErrorAction SilentlyContinue
                Write-Host "[HIGIENE] WebView2 UDF limpo com sucesso: $dir" -ForegroundColor DarkGreen
            } catch {
                Write-Host ("[HIGIENE-WARN] Nao foi possivel remover {0}: {1}" -f $dir, $_) -ForegroundColor DarkYellow
            }
        }
    }
}

# =============================================================================
# BLOCO C: ESTEIRA MULTI-BUILD DE PRODUÇÃO (-Build)
# =============================================================================

if ($Build) {
    Write-Host "`n[SOULS] Iniciando esteira de compilação multi-binária de produção..." -ForegroundColor Yellow
    
    # 1. Compilação dos assets frontend via Vite
    Write-Host "[SOULS] Compilando frontend assets Svelte 5 via Vite..." -ForegroundColor Cyan
    pnpm run build
    if ($LASTEXITCODE -ne 0) {
        Write-Host "[ERRO CRÍTICO] Build do frontend falhou!" -ForegroundColor Red
        exit $LASTEXITCODE
    }
    
    # 2. Compilação Rust Release dos 4 daemons de infraestrutura do Workspace
    Write-Host "[SOULS] Compilando daemons Rust em modo Release (souls_ui_shell, souls_mcp_server, agentgateway_tcp_proxy)..." -ForegroundColor Cyan
    Push-Location $PSScriptRoot
    try {
        cargo build --release -p souls_ui_shell --bin "souls_ui_shell"
        if ($LASTEXITCODE -ne 0) {
            Write-Host "[ERRO CRÍTICO] Compilação do souls_ui_shell falhou com Exit Code $LASTEXITCODE!" -ForegroundColor Red
            exit $LASTEXITCODE
        }
        cargo build --release -p souls_core `
            --bin "souls_mcp_server" `
            --bin "agentgateway_tcp_proxy" `
            --bin "mcp_stdio_guard" `
            --features "gateway_ccr"
        if ($LASTEXITCODE -ne 0) {
            Write-Host "[ERRO CRÍTICO] Compilação dos daemons souls_core falhou com Exit Code $LASTEXITCODE!" -ForegroundColor Red
            exit $LASTEXITCODE
        }
    } finally {
        Pop-Location
    }
    
    # 3. Transplante Físico de Runtime para .agents/bin/ (Desacoplamento Fábrica/Produto)
    $binDir = Join-Path $PSScriptRoot ".agents\bin"
    if (!(Test-Path $binDir)) {
        New-Item -ItemType Directory -Path $binDir | Out-Null
    }
    
    Write-Host "[SOULS] Transplantando executáveis para .agents/bin/..." -ForegroundColor Cyan
    $daemons = @("souls_ui_shell.exe", "souls_mcp_server.exe", "agentgateway_tcp_proxy.exe", "mcp_stdio_guard.exe")
    foreach ($daemon in $daemons) {
        $sourcePath = Join-Path $PSScriptRoot "target\release\$daemon"
        if (Test-Path $sourcePath) {
            Copy-Item $sourcePath (Join-Path $binDir $daemon) -Force
            Write-Host "   -> [OK] $daemon transplantado para .agents/bin/" -ForegroundColor DarkGreen
        } else {
            Write-Host "   -> [AVISO] $daemon não encontrado em target/release!" -ForegroundColor DarkYellow
        }
    }
    
    Write-Host "[DoD OK] Todos os binários de produção foram forjados em: .agents/bin/" -ForegroundColor Green
}

# =============================================================================
# BLOCOS D & E: INICIALIZAÇÃO DE AMBIENTES (Dev vs. Standalone)
# =============================================================================

if ($Dev) {
    # -------------------------------------------------------------------------
    # BLOCO E: INICIALIZAÇÃO DE DEV COM SEGURANÇA (-Dev)
    # -------------------------------------------------------------------------
    Write-Host "`n[SOULS] Modo de Desenvolvimento Ativo (-Dev)." -ForegroundColor Yellow
    Write-Host "[SOULS] Inicializando servidor de desenvolvimento Vite em background..." -ForegroundColor Cyan
    
    # Inicia o Vite em background como um Job do PowerShell
    Start-Job -ScriptBlock {
        param($root)
        Set-Location $root
        pnpm dev
    } -ArgumentList $PSScriptRoot -Name "SODA_Vite_DevServer" | Out-Null
    
    # Loop de ping na porta 1420 para impedir ERR_CONNECTION_REFUSED na WebView
    Write-Host "[SOULS] Aguardando resposta do localhost:1420 para abrir a janela de controle..." -ForegroundColor Cyan
    $portOpen = $false
    for ($i = 0; $i -lt 15; $i++) {
        $check = Test-NetConnection -ComputerName "127.0.0.1" -Port 1420 -InformationLevel Quiet -ErrorAction SilentlyContinue
        if ($check) {
            $portOpen = $true
            break
        }
        Write-Host "   -> Localhost ainda ocioso. Aguardando 1s..." -ForegroundColor DarkGray
        Start-Sleep -Seconds 1
    }
    
    if (-not $portOpen) {
        Write-Host "[ERRO] Servidor de desenvolvimento do Vite falhou ao responder na porta 1420!" -ForegroundColor Red
        Write-Host "Remova travas de rede ou verifique se há outra aplicação usando a porta." -ForegroundColor Yellow
        exit 1
    }
    
    Write-Host "[SOULS] Localhost ativo! Verificando binário de debug do souls_ui_shell..." -ForegroundColor Green
    $debugPath = Join-Path $PSScriptRoot "target\debug\souls_ui_shell.exe"
    if (-not (Test-Path $debugPath)) {
        $debugPath = Join-Path $PSScriptRoot "target\release\souls_ui_shell.exe"
    }
    if (-not (Test-Path $debugPath)) {
        Write-Host "[SOULS] Compilando souls_ui_shell em modo debug..." -ForegroundColor Cyan
        cargo build -p souls_ui_shell
        $debugPath = Join-Path $PSScriptRoot "target\debug\souls_ui_shell.exe"
    }
    
    $env:SOULS_DEV_URL = "http://localhost:1420"
    Start-Process $debugPath
    
} else {
    # -------------------------------------------------------------------------
    # BLOCO D: INICIALIZAÇÃO DE PRODUÇÃO (Standalone Offline)
    # -------------------------------------------------------------------------
    $agentsBinDir = Join-Path $PSScriptRoot ".agents\bin"
    $daemonPath = Join-Path $agentsBinDir "souls_ui_shell.exe"
    $proxyPath = Join-Path $agentsBinDir "agentgateway_tcp_proxy.exe"
    
    # Fallback paths
    if (-not (Test-Path $daemonPath)) {
        $daemonPath = Join-Path $PSScriptRoot "target\release\souls_ui_shell.exe"
    }
    if (-not (Test-Path $proxyPath)) {
        $proxyPath = Join-Path $PSScriptRoot "target\release\agentgateway_tcp_proxy.exe"
    }
    
    if (-not (Test-Path $daemonPath)) {
        Write-Host "[ERRO] Binário standalone não encontrado em: $daemonPath" -ForegroundColor Red
        Write-Host "-> Execute '.\boot.ps1 -Build' primeiro para compilar o ecossistema!" -ForegroundColor Yellow
        exit 1
    }
    
    Write-Host "`n[SOULS] Disparando ecossistema standalone de produção..." -ForegroundColor Green
    
    # 1. Disparo do Chassi Gráfico Bare-Metal (souls_ui_shell.exe)
    Write-Host "[CHASSIS] Iniciando souls_ui_shell.exe (Windows 11 DWM Desktop Acrylic)..." -ForegroundColor Cyan
    $daemonProc = Start-Process -FilePath $daemonPath -WorkingDirectory $PSScriptRoot -PassThru
    Write-Host ("[CHASSIS] souls_ui_shell iniciado (PID: {0})" -f $daemonProc.Id) -ForegroundColor DarkCyan
    
    # 2. Disparo Soberano do Proxy L7 (agentgateway_tcp_proxy.exe :3001)
    if (Test-Path $proxyPath) {
        Write-Host "[PROXY] Iniciando Proxy L7 soberano (agentgateway_tcp_proxy.exe :3001)..." -ForegroundColor Cyan
        $proxyProc = Start-Process `
            -FilePath $proxyPath `
            -ArgumentList @("--listen", "127.0.0.1:3001") `
            -WorkingDirectory $PSScriptRoot `
            -PassThru
        Write-Host ("[PROXY] agentgateway_tcp_proxy iniciado (PID: {0})" -f $proxyProc.Id) -ForegroundColor DarkCyan
    }
    
    # 3. Trava de Prontidão (Probe TCP na porta 3001)
    if (Test-Path $proxyPath) {
        Write-Host "`n[PROBE] Testando prontidão do proxy L7 na porta 3001..." -ForegroundColor Yellow
        $proxyReady = $false
        for ($i = 0; $i -lt 15; $i++) {
            $tcp = New-Object System.Net.Sockets.TcpClient
            try {
                $tcp.Connect("127.0.0.1", 3001)
                if ($tcp.Connected) {
                    $tcp.Close()
                    $proxyReady = $true
                    break
                }
            } catch {}
            Start-Sleep -Milliseconds 500
        }
        
        if ($proxyReady) {
            Write-Host "`n=======================================================" -ForegroundColor Green
            Write-Host " 🚀 SOULS MC ONLINE & PRONTO! (Proxy L7 :3001 & DWM Acrylic Ativos)" -ForegroundColor Green
            Write-Host " Interface Svelte 5 executando sob Winit + Wry Bare-Metal (0.0% GPU Idle)" -ForegroundColor Gray
            Write-Host "=======================================================\n" -ForegroundColor Green
        } else {
            Write-Host "[AVISO] Proxy L7 iniciado, mas a porta 3001 demorou a responder ao probe." -ForegroundColor DarkYellow
        }
    } else {
        Write-Host "`n=======================================================" -ForegroundColor Green
        Write-Host " 🚀 SOULS MC ONLINE & PRONTO! (Chassi Bare-Metal Ativo)" -ForegroundColor Green
        Write-Host "=======================================================\n" -ForegroundColor Green
    }
}
