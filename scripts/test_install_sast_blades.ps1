Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

. (Join-Path $PSScriptRoot "install_sast_blades.ps1")

function Assert-True {
    param(
        [bool]$Condition,
        [string]$Message
    )

    if (-not $Condition) {
        throw "ASSERT_TRUE falhou: $Message"
    }
}

function Assert-Equal {
    param(
        $Actual,
        $Expected,
        [string]$Message
    )

    if ($Actual -ne $Expected) {
        throw "ASSERT_EQUAL falhou: $Message. Actual='$Actual' Expected='$Expected'"
    }
}

$missingPlan = Get-SastBladeInstallPlan -CommandMap @{
    uv = $true
    winget = $true
    go = $true
    mix = $false
    ruff = $false
    bandit = $false
    govulncheck = $false
    cppcheck = $false
    biome = $false
    oxlint = $false
    opengrep = $false
    sobelow = $false
}

$ruff = $missingPlan | Where-Object { $_.Command -eq "ruff" } | Select-Object -First 1
$bandit = $missingPlan | Where-Object { $_.Command -eq "bandit" } | Select-Object -First 1
$govulncheck = $missingPlan | Where-Object { $_.Command -eq "govulncheck" } | Select-Object -First 1
$cppcheck = $missingPlan | Where-Object { $_.Command -eq "cppcheck" } | Select-Object -First 1
$biome = $missingPlan | Where-Object { $_.Command -eq "biome" } | Select-Object -First 1
$oxlint = $missingPlan | Where-Object { $_.Command -eq "oxlint" } | Select-Object -First 1
$opengrep = $missingPlan | Where-Object { $_.Command -eq "opengrep" } | Select-Object -First 1
$sobelow = $missingPlan | Where-Object { $_.Command -eq "sobelow" } | Select-Object -First 1

Assert-Equal $ruff.Manager "uv-tool" "ruff deve preferir uv tool"
Assert-Equal $bandit.Manager "uv-tool" "bandit deve preferir uv tool"
Assert-Equal $govulncheck.Manager "go-install" "govulncheck deve preferir go install"
Assert-Equal $cppcheck.Manager "winget" "cppcheck deve preferir winget"
Assert-Equal $biome.Manager "winget" "biome deve preferir winget"
Assert-Equal $oxlint.Manager "winget" "oxlint deve preferir winget"
Assert-Equal $opengrep.Manager "powershell-web" "opengrep deve usar install.ps1 oficial"
Assert-Equal $sobelow.Manager "elixir-bootstrap" "sobelow deve depender do bootstrap Elixir"
Assert-True ($sobelow.Prerequisites -contains "erlang") "sobelow deve exigir Erlang"
Assert-True ($sobelow.Prerequisites -contains "elixir") "sobelow deve exigir Elixir"
Assert-True ($sobelow.Prerequisites -contains "mix") "sobelow deve exigir mix"

$installedPlan = Get-SastBladeInstallPlan -CommandMap @{
    uv = $true
    winget = $true
    go = $true
    mix = $true
    ruff = $true
    bandit = $true
    govulncheck = $true
    cppcheck = $true
    biome = $true
    oxlint = $true
    opengrep = $true
    sobelow = $true
}

Assert-Equal @($installedPlan).Count 0 "plano deve ser vazio quando tudo ja estiver instalado"
Write-Host "[OK] test_install_sast_blades.ps1" -ForegroundColor Green
