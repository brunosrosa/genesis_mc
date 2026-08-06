# Marco 4.1.2 — Prova de Fogo Isola (NAO toca o supervisor)
# Executa APENAS o step 1.5/5 (Transplante) sem matar daemons.
# Proposito: validar que a rotina produz os 3 .exe em .agents/bin/
# sem interferir com o gateway que ja esta rodando.

$ErrorActionPreference = 'Stop'
$projectRoot = (Resolve-Path (Join-Path $PSScriptRoot '..\..\..\..')).Path
$srcTauri = Join-Path $projectRoot 'src-tauri'
$agentsBin = Join-Path $projectRoot '.agents\bin'

Write-Host ("[ISOLA] src-tauri: {0}" -f $srcTauri) -ForegroundColor DarkGray
Write-Host ("[ISOLA] destino:  {0}" -f $agentsBin) -ForegroundColor DarkGray

if (-not (Test-Path $agentsBin)) {
    New-Item -ItemType Directory -Path $agentsBin -Force | Out-Null
    Write-Host ("[ISOLA] criado: {0}" -f $agentsBin) -ForegroundColor Green
}

# O build incremental dos 3 daemons (pode levar 1-3min se target/debug/ estiver frio).
Write-Host "[ISOLA] cargo build --bin souls_mcp_server --bin agentgateway_tcp_proxy --bin mcp_stdio_guard" -ForegroundColor Cyan
$buildStart = Get-Date
$proc = Start-Process -FilePath 'cargo' `
    -ArgumentList @('build', '--message-format', 'short', '--features', 'tauri-app,gateway_ccr,llama_backend', '--bin', 'souls_mcp_server', '--bin', 'agentgateway_tcp_proxy', '--bin', 'mcp_stdio_guard', '--locked') `
    -WorkingDirectory $srcTauri `
    -RedirectStandardOutput (Join-Path $env:TEMP 'isola_build.out.log') `
    -RedirectStandardError (Join-Path $env:TEMP 'isola_build.err.log') `
    -PassThru -NoNewWindow -Wait
$buildElapsed = [int]((Get-Date) - $buildStart).TotalSeconds
Write-Host ("[ISOLA] build exit code: {0} em {1}s" -f $proc.ExitCode, $buildElapsed) -ForegroundColor $(if ($proc.ExitCode -eq 0) { 'Green' } else { 'Red' })

if ($proc.ExitCode -ne 0) {
    Write-Host "[ISOLA] FAIL-CLOSED: abortando transplante." -ForegroundColor Red
    Get-Content -LiteralPath (Join-Path $env:TEMP 'isola_build.err.log') -Tail 30
    exit 1
}

# R2 da Linha Vermelha: NTFS handle release
Write-Host "[ISOLA] Start-Sleep 1s (NTFS handle release)..." -ForegroundColor DarkGray
Start-Sleep -Seconds 1

# R3 da Linha Vermelha: Copy-Item -Force
$bins = @('souls_mcp_server.exe', 'agentgateway_tcp_proxy.exe', 'mcp_stdio_guard.exe')
$sourceDir = Join-Path $srcTauri 'target\debug'
foreach ($b in $bins) {
    $src = Join-Path $sourceDir $b
    $dst = Join-Path $agentsBin $b
    if (Test-Path $src) {
        Copy-Item -LiteralPath $src -Destination $dst -Force
        $info = Get-Item -LiteralPath $dst
        Write-Host ("[ISOLA] transplantado: {0} ({1:N0} bytes)" -f $b, $info.Length) -ForegroundColor Green
    } else {
        Write-Host ("[ISOLA] FALTA EM target/debug/: {0}" -f $b) -ForegroundColor Yellow
    }
}

Write-Host ""
Write-Host "[ISOLA] === LISTAGEM DE .agents/bin/ ===" -ForegroundColor Cyan
Get-ChildItem -LiteralPath $agentsBin -Filter '*.exe' -ErrorAction SilentlyContinue | ForEach-Object {
    Write-Host ("  {0}  {1:N0} bytes" -f $_.Name, $_.Length)
}
