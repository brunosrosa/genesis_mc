# Marco 4.1.3 — Audit Completo do `tools/list` (ADRs 026, 037, 041)
# Extrai todas as tools do dispatcher via regex e verifica conformidade.

$ErrorActionPreference = 'Stop'
$src = Get-Content -LiteralPath 'z:\souls_mc\src-tauri\src\bin\souls_mcp_server.rs' -Raw

# Regex: extrair entradas `{ "name": "...", "description": "..." }` do tools/list
# e entradas inline `{ "name": "...", "description": "..." ... }`.
$pattern = '(?ms)\{\s*"name":\s*"([^"]+)",\s*"description":\s*"([^"]+)"[^}]*\}'
$matches_found = [regex]::Matches($src, $pattern)

Write-Host ""
Write-Host "=== AUDITORIA COMPLETA — $($matches_found.Count) tools no tools/list ===" -ForegroundColor Cyan
Write-Host ""

$issues = New-Object System.Collections.Generic.List[string]
$tools = @()

foreach ($m in $matches_found) {
    $name = $m.Groups[1].Value
    $desc = $m.Groups[2].Value
    $nameLen = $name.Length
    $descLen = $desc.Length

    $tools += [PSCustomObject]@{ Name = $name; NameLen = $nameLen; Desc = $desc; DescLen = $descLen }

    # ADR-041 §1: teto de 32 chars
    if ($nameLen -gt 32) {
        $issues.Add("[ADR-041 §1] '$name' excede 32 chars ($nameLen)") | Out-Null
    }
    # ADR-041 §2: teto de 120 chars
    if ($descLen -gt 120) {
        $issues.Add("[ADR-041 §2] '$name' desc excede 120 chars ($descLen): $desc") | Out-Null
    }
    # ADR-026: prefixos proibidos no NOME
    if ($name -like 'souls_*') {
        $issues.Add("[ADR-026 §2] '$name' tem prefixo 'souls_' (canibalizacao cirurgica: aliases OK no dispatcher, NAO no tools/list)") | Out-Null
    }
    if ($name -like 'ctx_*') {
        $issues.Add("[ADR-026 §2] '$name' tem prefixo 'ctx_' (proibido por ADR-026 §4)") | Out-Null
    }
    if ($name -like 'tool_*' -or $name -like 'mcp_*') {
        $issues.Add("[ADR-026 §4] '$name' tem prefixo 'tool_'/'mcp_' (guilhotina de pleonasmos)") | Out-Null
    }
    # ADR-026: brand 'SOULS' na descricao
    if ($desc -match 'Canone SOULS' -or $desc -match 'Cânone SOULS') {
        $issues.Add("[ADR-026 §2] '$name' desc menciona brand 'SOULS' (Zero-Brand)") | Out-Null
    }
    # FALSO VERDE
    if ($desc -match 'not_implemented_yet') {
        $issues.Add("[FALSO VERDE] '$name' ainda e stub: $desc") | Out-Null
    }
    if ($desc -match 'sandbox_audit_pending') {
        $issues.Add("[FALSO VERDE] '$name' tem pendencia: $desc") | Out-Null
    }
    if ($desc -match 'Pendente') {
        $issues.Add("[FALSO VERDE] '$name' marcado como Pendente: $desc") | Out-Null
    }

    $flag = if ($issues.Count -gt 0) { "[!]" } else { "[ ]" }
    Write-Host ("  {0} [n={1,2}] [d={2,3}] {3}" -f $flag, $nameLen, $descLen, $name) -ForegroundColor $(if ($flag -eq '[!]') { 'Yellow' } else { 'DarkGray' })
}

# Checagem de DUPLICATAS: descricoes identicas
$dupGroups = $tools | Group-Object -Property Desc | Where-Object { $_.Count -gt 1 }
foreach ($g in $dupGroups) {
    $names = ($g.Group | ForEach-Object { $_.Name }) -join ', '
    $issues.Add("[DUPLICATA] descricao IDENTICA em $($g.Count) tools: $names") | Out-Null
}

# Checagem de CANONICAL: o tool canonico deve ser o mais curto / sem prefixo
# (heuristica: o tool canonico nao tem prefixo 'souls_' nem 'ctx_')

Write-Host ""
Write-Host "=== ISSUES ENCONTRADOS: $($issues.Count) ===" -ForegroundColor $(if ($issues.Count -gt 0) { 'Red' } else { 'Green' })
Write-Host ""
foreach ($i in $issues) {
    Write-Host "  $i" -ForegroundColor Yellow
}

Write-Host ""
Write-Host "=== ESTATISTICAS ===" -ForegroundColor Cyan
Write-Host ("  Total tools:            {0}" -f $tools.Count)
Write-Host ("  Tools canonicos:        {0} (sem prefixo)" -f ($tools | Where-Object { $_.Name -notlike 'souls_*' -and $_.Name -notlike 'ctx_*' }).Count)
Write-Host ("  Tools souls_* (BUG):    {0}" -f ($tools | Where-Object { $_.Name -like 'souls_*' }).Count)
Write-Host ("  Tools ctx_* (BUG):      {0}" -f ($tools | Where-Object { $_.Name -like 'ctx_*' }).Count)
Write-Host ("  Stubs (FALSO VERDE):    {0}" -f ($tools | Where-Object { $_.Desc -match 'not_implemented_yet' }).Count)
Write-Host ("  Descricoes duplicadas:  {0}" -f ($dupGroups | Measure-Object).Count)
Write-Host ""
