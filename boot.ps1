Param(
    [switch]$Dev,
    [switch]$Build
)

# 1. Tratamento Estrito de Erros e Configuração de Terminal
$ErrorActionPreference = "Stop"
try { [console]::InputEncoding = [console]::OutputEncoding = New-Object System.Text.UTF8Encoding } catch {}
if ($null -ne $PSStyle) { $PSStyle.OutputRendering = 'ANSI' }
try { Clear-Host } catch {}

Write-Host "=======================================================" -ForegroundColor Cyan
Write-Host " 👻 SOULS MC BOOTSTRAP: Inicializando a Maquina Silenciosa " -ForegroundColor White -BackgroundColor DarkBlue
Write-Host "=======================================================" -ForegroundColor Cyan

# 2. Configurações Globais de Telemetria e Logs (ADR-038)
# Silencia ruídos cosméticos de crates transitivas e foca estritamente em logs lógicos
$env:RUST_LOG = "souls_mc_lib=info,souls_sast=debug,souls_harvester=debug,headroom_engine=debug,llama_engine=info,hardware_profiler=info,model_manager=debug,souls_ccr=debug,ignore=warn,globset=warn,walkdir=warn"
$env:SOULS_CCR_MAX_RAM_MB = "256"
$env:SOULS_HEADROOM_SAFETY_MARGIN = "512"
$env:SOULS_HEADROOM_OUTPUT_BUFFER = "4096"

# 3. Configuração Persistente do Sccache em Dev Drive Z: (ADR-039)
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

# 4. Soluções e Patches de Compilação CUDA & C-Runtime (MSVC) (ADR-039)
# GGML_CCACHE=ON no llama.cpp faz o cmake falhar quando nvcc é wrappado por sccache; forçamos OFF
$env:GGML_CCACHE = "OFF"
$env:CMAKE_CUDA_COMPILER_LAUNCHER = ""
$env:CUDA_CACHE_DISABLE = "1"
$env:CFLAGS = "/MD"
$env:CXXFLAGS = "/MD"
$env:CMAKE_CUDA_ARCHITECTURES = "75"  # Pinho de arquitetura GPU para RTX 2060m (sm_75) para acelerar o build em 10x
Write-Host "[CUDA] GGML_CCACHE=OFF + CMAKE_CUDA_ARCHITECTURES=75 injetados para acelerar build do llama_backend" -ForegroundColor DarkGreen

# Patch idempotente vendor/llama-cpp-sys-2 (Evita quebra de sccache + NVCC)
$vendorLlamaCmake = Join-Path $PSScriptRoot "src-tauri\vendor\llama-cpp-sys-2\llama.cpp\ggml\CMakeLists.txt"
if (Test-Path $vendorLlamaCmake) {
    $content = Get-Content -LiteralPath $vendorLlamaCmake -Raw -ErrorAction SilentlyContinue
    if ($content -and $content -match 'option\(GGML_CCACHE "ggml: use ccache if available"\s+ON\)') {
        (Get-Content -LiteralPath $vendorLlamaCmake) -replace `
            'option\(GGML_CCACHE "ggml: use ccache if available"\s+ON\)', 
            'option(GGML_CCACHE "ggml: use ccache if available"                   OFF)' | Set-Content -LiteralPath $vendorLlamaCmake
        Write-Host "[PATCH] vendor/llama-cpp-sys-2 GGML_CCACHE -> OFF aplicado com sucesso." -ForegroundColor DarkGreen
    }
}

# Patch idempotente ik-llama-cpp-sys (Satisfaz o git rev-parse em caches sem .git)
$ikLlamaCrateRoot = Join-Path $env:USERPROFILE ".cargo\registry\src\index.crates.io-1949cf8c6b5b557f\ik-llama-cpp-sys-*"
$ikLlamaDir = Get-Item -Path $ikLlamaCrateRoot -ErrorAction SilentlyContinue | Select-Object -First 1
if ($ikLlamaDir) {
    $gitDir = Join-Path $ikLlamaDir.FullName "ik_llama.cpp\.git"
    if (-not (Test-Path $gitDir)) {
        Push-Location $ikLlamaDir.FullName\ik_llama.cpp
        try {
            git init -q
            git -c user.email=souls@souls_mc -c user.name=SOULS commit --allow-empty -m "souls-mc placeholder" -q 2>$null
            Write-Host "[PATCH] ik-llama-cpp-sys/ik_llama.cpp git init aplicado para evitar erros de rev-parse." -ForegroundColor DarkGreen
        } catch {
            Write-Host "[PATCH-SKIP] git init falhou ou ignorado: $_" -ForegroundColor DarkYellow
        } finally {
            Pop-Location
        }
    }
}

# 5. Expurgo de Zumbis e Higiene de RAM (Amplo)
Write-Host "`n[SOULS] Varrendo e higienizando processos e sidecars antigos..." -ForegroundColor Yellow
$zombies = @("agentgateway", "agentgateway_tcp_proxy", "souls_mc", "mcp_stdio_guard", "souls_mcp_server", "sequential-thinking-server", "sequential-thinking-mcp", "biome", "opengrep", "oxlint")
$killed = @()
foreach ($z in $zombies) {
    $existing = Get-Process -Name $z -ErrorAction SilentlyContinue
    if ($existing) {
        $killed += [PSCustomObject]@{ Name = $z; Pids = ($existing.Id -join ",") }
        Stop-Process -Name $z -Force -ErrorAction SilentlyContinue
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

# 6. Esteira de Compilação Standalone de Produção (-Build)
if ($Build) {
    Write-Host "`n[SOULS] Iniciando esteira de compilação standalone de produção..." -ForegroundColor Yellow
    
    Write-Host "[SOULS] Compilando frontend assets via Vite..." -ForegroundColor Cyan
    pnpm run build
    
    Write-Host "[SOULS] Compilando Rust backend em modo Release (Features: tauri-app)..." -ForegroundColor Cyan
    Push-Location (Join-Path $PSScriptRoot "src-tauri")
    try {
        cargo build --release --features tauri-app
    } finally {
        Pop-Location
    }
    
    # Garantir diretório de distribuição
    $binDir = Join-Path $PSScriptRoot ".agents\bin"
    if (!(Test-Path $binDir)) {
        New-Item -ItemType Directory -Path $binDir | Out-Null
    }
    
    Write-Host "[SOULS] Movendo executável para quarentena standalone em $binDir..." -ForegroundColor Cyan
    Copy-Item (Join-Path $PSScriptRoot "src-tauri\target\release\souls_mc.exe") (Join-Path $binDir "souls_mc.exe") -Force
    
    Write-Host "[DoD OK] Executável com UI embarcada gerado em: .agents/bin/souls_mc.exe" -ForegroundColor Green
}

# 7. Inicialização de Ambientes (Dev vs. Standalone)
if ($Dev) {
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
    
    Write-Host "[SOULS] Localhost ativo! Disparando janela de debug..." -ForegroundColor Green
    $debugPath = Join-Path $PSScriptRoot "src-tauri\target\debug\souls_mc.exe"
    Start-Process $debugPath
    
} else {
    # Inicialização Standalone de Produção (Padrão)
    $standalonePath = Join-Path $PSScriptRoot ".agents\bin\souls_mc.exe"
    if (Test-Path $standalonePath) {
        Write-Host "`n[SOULS] Disparando executável standalone offline de produção..." -ForegroundColor Green
        Write-Host "Carregando Svelte 5 nativamente a partir dos recursos embutidos. Zero dependências de rede." -ForegroundColor Gray
        Start-Process $standalonePath
    } else {
        Write-Host "[ERRO] Binário standalone de produção não encontrado em: $standalonePath" -ForegroundColor Red
        Write-Host "-> Execute '.\boot.ps1 -Build' primeiro para compilar o sistema com a interface embarcada!" -ForegroundColor Yellow
        exit 1
    }
}
