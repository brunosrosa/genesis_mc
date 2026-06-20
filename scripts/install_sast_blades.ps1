Set-StrictMode -Version Latest

function Test-CommandPresent {
    param(
        [Parameter(Mandatory = $true)]
        [string]$Name
    )

    return $null -ne (Get-Command $Name -ErrorAction SilentlyContinue)
}

function Add-SessionPathEntry {
    param(
        [Parameter(Mandatory = $true)]
        [string]$PathEntry
    )

    if ([string]::IsNullOrWhiteSpace($PathEntry) -or -not (Test-Path $PathEntry)) {
        return
    }

    $separator = [IO.Path]::PathSeparator
    $existing = @($env:PATH -split [Regex]::Escape($separator)) | Where-Object { -not [string]::IsNullOrWhiteSpace($_) }
    if ($existing -contains $PathEntry) {
        return
    }
    $env:PATH = ($existing + $PathEntry) -join $separator
}

function Ensure-UserToolBin {
    $userBin = Join-Path $env:USERPROFILE ".local\bin"
    New-Item -ItemType Directory -Path $userBin -Force | Out-Null
    Add-SessionPathEntry -PathEntry $userBin
    return $userBin
}

function Resolve-InstalledBinaryCandidate {
    param(
        [Parameter(Mandatory = $true)]
        [string]$CommandName
    )

    $candidates = switch ($CommandName) {
        "cppcheck" {
            @(
                (Join-Path $env:ProgramFiles "Cppcheck\cppcheck.exe"),
                (Join-Path ${env:ProgramFiles(x86)} "Cppcheck\cppcheck.exe")
            )
        }
        "biome" {
            @(
                (Join-Path $env:LOCALAPPDATA "Microsoft\WinGet\Packages\BiomeJS.Biome_Microsoft.Winget.Source_8wekyb3d8bbwe\biome.exe")
            )
        }
        "oxlint" {
            @(
                (Join-Path $env:LOCALAPPDATA "Microsoft\WinGet\Packages\oxc-project.oxlint_Microsoft.Winget.Source_8wekyb3d8bbwe\oxlint-x86_64-pc-windows-msvc.exe")
            )
        }
        default { @() }
    }

    foreach ($candidate in $candidates) {
        if (-not [string]::IsNullOrWhiteSpace($candidate) -and (Test-Path $candidate)) {
            return $candidate
        }
    }
    return $null
}

function Ensure-LocalShim {
    param(
        [Parameter(Mandatory = $true)]
        [string]$CommandName,
        [Parameter(Mandatory = $true)]
        [string]$SourcePath
    )

    if (-not (Test-Path $SourcePath)) {
        return
    }

    $shimDir = Ensure-UserToolBin
    $legacyExe = Join-Path $shimDir ($CommandName + ".exe")
    if (Test-Path $legacyExe) {
        Remove-Item -LiteralPath $legacyExe -Force -ErrorAction SilentlyContinue
    }
    $shimPath = Join-Path $shimDir ($CommandName + ".cmd")
    $wrapper = "@echo off`r`n`"$SourcePath`" %*`r`n"
    [System.IO.File]::WriteAllText($shimPath, $wrapper, [System.Text.Encoding]::ASCII)
}

function Get-CurrentCommandMap {
    $map = @{}
    foreach ($command in @(
        "uv", "winget", "go", "mix", "ruff", "bandit", "govulncheck", "cppcheck",
        "biome", "oxlint", "opengrep", "sobelow", "erl", "elixir"
    )) {
        $map[$command] = Test-CommandPresent -Name $command
    }
    return $map
}

function Get-SastBladeInstallPlan {
    param(
        [hashtable]$CommandMap = @{}
    )

    $definitions = @(
        [pscustomobject]@{
            Command = "ruff"
            Manager = "uv-tool"
            InstallRef = "ruff"
            Prerequisites = @("uv")
        },
        [pscustomobject]@{
            Command = "bandit"
            Manager = "uv-tool"
            InstallRef = "bandit"
            Prerequisites = @("uv")
        },
        [pscustomobject]@{
            Command = "govulncheck"
            Manager = "go-install"
            InstallRef = "golang.org/x/vuln/cmd/govulncheck@latest"
            Prerequisites = @("go")
        },
        [pscustomobject]@{
            Command = "cppcheck"
            Manager = "winget"
            InstallRef = "Cppcheck.Cppcheck"
            Prerequisites = @("winget")
        },
        [pscustomobject]@{
            Command = "biome"
            Manager = "winget"
            InstallRef = "BiomeJS.Biome"
            Prerequisites = @("winget")
        },
        [pscustomobject]@{
            Command = "oxlint"
            Manager = "winget"
            InstallRef = "oxc-project.oxlint"
            Prerequisites = @("winget")
        },
        [pscustomobject]@{
            Command = "opengrep"
            Manager = "powershell-web"
            InstallRef = "https://raw.githubusercontent.com/opengrep/opengrep/main/install.ps1"
            Prerequisites = @()
        },
        [pscustomobject]@{
            Command = "sobelow"
            Manager = "elixir-bootstrap"
            InstallRef = "mix archive.install hex sobelow --force"
            Prerequisites = @("erlang", "elixir", "mix")
        }
    )

    $plan = New-Object System.Collections.Generic.List[object]
    foreach ($definition in $definitions) {
        $installed = $false
        if ($CommandMap.ContainsKey($definition.Command)) {
            $installed = [bool]$CommandMap[$definition.Command]
        }
        if ($installed) {
            continue
        }

        $plan.Add([pscustomobject]@{
            Command = $definition.Command
            Manager = $definition.Manager
            InstallRef = $definition.InstallRef
            Prerequisites = @($definition.Prerequisites)
        })
    }
    return $plan.ToArray()
}

function Ensure-SastBladePaths {
    Ensure-UserToolBin | Out-Null
    Add-SessionPathEntry -PathEntry (Join-Path $env:USERPROFILE ".cargo\bin")
    Add-SessionPathEntry -PathEntry (Join-Path $env:USERPROFILE ".mix\escripts")
    Add-SessionPathEntry -PathEntry (Join-Path $env:LOCALAPPDATA "Microsoft\WinGet\Links")
    Add-SessionPathEntry -PathEntry (Join-Path $env:USERPROFILE ".opengrep\cli\latest")

    if (Test-CommandPresent -Name "go") {
        try {
            $goPath = (& go env GOPATH | Select-Object -First 1).Trim()
            if (-not [string]::IsNullOrWhiteSpace($goPath)) {
                Add-SessionPathEntry -PathEntry (Join-Path $goPath "bin")
            }
        } catch {}
    }

    foreach ($root in @($env:LOCALAPPDATA, $env:ProgramFiles, ${env:ProgramFiles(x86)})) {
        if ([string]::IsNullOrWhiteSpace($root) -or -not (Test-Path $root)) {
            continue
        }
        $erlCandidate = Get-ChildItem -Path $root -Filter "erl.exe" -Recurse -ErrorAction SilentlyContinue |
            Select-Object -First 1
        if ($erlCandidate) {
            Add-SessionPathEntry -PathEntry $erlCandidate.Directory.FullName
        }
        $mixCandidate = Get-ChildItem -Path $root -Filter "mix.bat" -Recurse -ErrorAction SilentlyContinue |
            Select-Object -First 1
        if ($mixCandidate) {
            Add-SessionPathEntry -PathEntry $mixCandidate.Directory.FullName
        }
    }

    foreach ($shimCommand in @("cppcheck", "biome", "oxlint")) {
        $candidate = Resolve-InstalledBinaryCandidate -CommandName $shimCommand
        if ($candidate) {
            Ensure-LocalShim -CommandName $shimCommand -SourcePath $candidate
        }
    }
}

function Invoke-InstallCommand {
    param(
        [Parameter(Mandatory = $true)]
        [scriptblock]$Script,
        [Parameter(Mandatory = $true)]
        [string]$FailureMessage
    )

    & $Script
    if ($LASTEXITCODE -ne 0) {
        throw "$FailureMessage (exit=$LASTEXITCODE)"
    }
}

function Install-WithWinget {
    param(
        [Parameter(Mandatory = $true)]
        [string]$PackageId
    )

    Invoke-InstallCommand -FailureMessage "Falha ao instalar $PackageId via winget" -Script {
        & winget install --id $PackageId --accept-package-agreements --accept-source-agreements --disable-interactivity --silent
    }
}

function Install-WithUvTool {
    param(
        [Parameter(Mandatory = $true)]
        [string]$PackageName
    )

    Invoke-InstallCommand -FailureMessage "Falha ao instalar $PackageName via uv tool" -Script {
        & uv tool install --upgrade $PackageName
    }
}

function Install-WithGo {
    param(
        [Parameter(Mandatory = $true)]
        [string]$ModuleRef
    )

    Invoke-InstallCommand -FailureMessage "Falha ao instalar $ModuleRef via go install" -Script {
        & go install $ModuleRef
    }
}

function Install-OpenGrep {
    Invoke-Expression "& ([scriptblock]::Create((Invoke-RestMethod 'https://raw.githubusercontent.com/opengrep/opengrep/main/install.ps1')))"
}

function Ensure-ElixirToolchain {
    Ensure-SastBladePaths

    if (-not (Test-CommandPresent -Name "erl")) {
        Install-WithWinget -PackageId "Erlang.ErlangOTP"
        Ensure-SastBladePaths
    }

    if (-not (Test-CommandPresent -Name "mix")) {
        $downloadDir = Join-Path $env:TEMP "soda-sast-installer"
        $installerPath = Join-Path $downloadDir "elixir-websetup.exe"
        New-Item -ItemType Directory -Path $downloadDir -Force | Out-Null
        Invoke-WebRequest -Uri "https://github.com/elixir-lang/elixir-windows-setup/releases/download/v2.4/elixir-websetup.exe" -OutFile $installerPath
        Invoke-InstallCommand -FailureMessage "Falha ao instalar Elixir via elixir-websetup.exe" -Script {
            & $installerPath /VERYSILENT /SUPPRESSMSGBOXES /NORESTART /SP-
        }
        Ensure-SastBladePaths
    }

    if (-not (Test-CommandPresent -Name "mix")) {
        throw "Bootstrap Elixir concluido sem expor 'mix' no PATH da sessao"
    }

    Invoke-InstallCommand -FailureMessage "Falha ao instalar Hex localmente" -Script {
        & mix local.hex --force
    }
}

function Ensure-SastBladesInstalled {
    param(
        [switch]$Quiet
    )

    Ensure-SastBladePaths
    $results = New-Object System.Collections.Generic.List[object]
    $plan = Get-SastBladeInstallPlan -CommandMap (Get-CurrentCommandMap)

    foreach ($item in $plan) {
        $status = "ok"
        $errorText = $null
        try {
            switch ($item.Manager) {
                "uv-tool" {
                    Install-WithUvTool -PackageName $item.InstallRef
                }
                "go-install" {
                    Install-WithGo -ModuleRef $item.InstallRef
                }
                "winget" {
                    Install-WithWinget -PackageId $item.InstallRef
                }
                "powershell-web" {
                    Install-OpenGrep
                }
                "elixir-bootstrap" {
                    Ensure-ElixirToolchain
                    Invoke-InstallCommand -FailureMessage "Falha ao instalar sobelow via mix" -Script {
                        & mix archive.install hex sobelow --force
                    }
                }
                default {
                    throw "Manager desconhecido: $($item.Manager)"
                }
            }
            Ensure-SastBladePaths
            if (-not (Test-CommandPresent -Name $item.Command)) {
                throw "Instalacao concluida, mas '$($item.Command)' nao ficou acessivel no PATH da sessao"
            }
        } catch {
            $status = "failed"
            $errorText = $_.Exception.Message
        }

        $result = [pscustomobject]@{
            Command = $item.Command
            Manager = $item.Manager
            Status = $status
            Error = $errorText
        }
        $results.Add($result)
        if (-not $Quiet) {
            if ($status -eq "ok") {
                Write-Host "[OK] Lâmina pronta: $($item.Command) via $($item.Manager)" -ForegroundColor Green
            } else {
                Write-Host "[WARN] Lâmina com falha: $($item.Command) via $($item.Manager) :: $errorText" -ForegroundColor Yellow
            }
        }
    }

    return $results.ToArray()
}
