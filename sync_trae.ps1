$ErrorActionPreference = "Stop"

$root = Split-Path -Parent $MyInvocation.MyCommand.Path
$agentsDir = Join-Path $root ".agents"
$rulesDir = Join-Path $agentsDir "rules"

$traeDir = Join-Path $root ".trae"
$traeRulesDir = Join-Path $traeDir "rules"
$userRulesSrc = "C:\Users\rosas\.gemini\GEMINI.md"
$userRulesDst = Join-Path $traeRulesDir "user_rules.md"
$projectRulesPath = Join-Path $traeRulesDir "project_rules.md"
$dstSkillsDir = Join-Path $traeDir "skills"
$srcSkillsDir = Join-Path $agentsDir "skills"

New-Item -ItemType Directory -Force -Path $traeDir | Out-Null
New-Item -ItemType Directory -Force -Path $traeRulesDir | Out-Null

if (-not (Test-Path $userRulesSrc)) {
    throw "Arquivo de regras globais nao encontrado: $userRulesSrc"
}

$userRulesContent = Get-Content -Raw -Path $userRulesSrc
Set-Content -Path $userRulesDst -Value $userRulesContent -Encoding utf8

$legacyUserRules = Join-Path $traeDir "user_rules.md"
if (Test-Path $legacyUserRules) {
    Remove-Item -Force -Path $legacyUserRules
}

$filesToConcat = @()
if (Test-Path $rulesDir) {
    $filesToConcat += Get-ChildItem -Path $rulesDir -Filter "*.md" -File -Recurse | Sort-Object FullName
}

$combined = ($filesToConcat | ForEach-Object { Get-Content -Raw -Path $_.FullName }) -join "`r`n`r`n"
Set-Content -Path $projectRulesPath -Value $combined -Encoding utf8

$legacyProjectRules = Join-Path $traeDir "project_rules.md"
if (Test-Path $legacyProjectRules) {
    Remove-Item -Force -Path $legacyProjectRules
}

if (-not (Test-Path $srcSkillsDir)) {
    throw "Pasta de origem nao encontrada: $srcSkillsDir"
}

New-Item -ItemType Directory -Force -Path $dstSkillsDir | Out-Null

function Get-RelPath([string]$Base, [string]$Path) {
    return [System.IO.Path]::GetRelativePath($Base, $Path)
}

$srcItems = Get-ChildItem -Path $srcSkillsDir -Recurse -Force

foreach ($item in $srcItems) {
    $rel = Get-RelPath $srcSkillsDir $item.FullName
    $dst = Join-Path $dstSkillsDir $rel

    if ($item.PSIsContainer) {
        New-Item -ItemType Directory -Force -Path $dst | Out-Null
        continue
    }

    $dstParent = Split-Path -Parent $dst
    if (-not (Test-Path $dstParent)) {
        New-Item -ItemType Directory -Force -Path $dstParent | Out-Null
    }

    Copy-Item -Path $item.FullName -Destination $dst -Force
}

$dstItems = Get-ChildItem -Path $dstSkillsDir -Recurse -Force -ErrorAction SilentlyContinue

$dstFiles = $dstItems | Where-Object { -not $_.PSIsContainer } | Sort-Object FullName -Descending
foreach ($item in $dstFiles) {
    $rel = Get-RelPath $dstSkillsDir $item.FullName
    $src = Join-Path $srcSkillsDir $rel
    if (-not (Test-Path $src)) {
        Remove-Item -Force -Path $item.FullName
    }
}

$dstDirs = $dstItems | Where-Object { $_.PSIsContainer } | Sort-Object FullName -Descending
foreach ($item in $dstDirs) {
    $rel = Get-RelPath $dstSkillsDir $item.FullName
    $src = Join-Path $srcSkillsDir $rel
    if (-not (Test-Path $src)) {
        Remove-Item -Force -Recurse -Path $item.FullName
    }
}

Write-Host "OK: .trae/project_rules.md atualizado e .trae/skills espelhado a partir de .agents/skills."
