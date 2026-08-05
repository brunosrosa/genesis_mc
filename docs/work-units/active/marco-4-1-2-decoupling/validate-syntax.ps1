$tokens = $null
$null = [System.Management.Automation.PSParser]::Tokenize((Get-Content -LiteralPath 'z:\souls_mc\boot.ps1' -Raw), [ref]$tokens)
Write-Host 'OK: boot.ps1 PowerShell syntax is valid.'

# YAML sanity check
try {
    $yaml = Get-Content -LiteralPath 'z:\souls_mc\gateway-config.yaml' -Raw
    Write-Host ('YAML length: {0} bytes' -f $yaml.Length)
    if ($yaml -match 'cmd:.*\.agents/bin/souls_mcp_server.exe') {
        Write-Host 'OK: gateway-config.yaml aponta para .agents/bin/ (Marco 4.1.2).'
    } else {
        Write-Host 'ERR: gateway-config.yaml NAO aponta para .agents/bin/.'
    }
    if ($yaml -notmatch 'target/debug/souls_mcp_server.exe') {
        Write-Host 'OK: gateway-config.yaml NAO referencia mais target/debug/.'
    } else {
        Write-Host 'ERR: gateway-config.yaml AINDA referencia target/debug/.'
    }
} catch {
    Write-Host ('YAML read failed: {0}' -f $_.Exception.Message)
}
