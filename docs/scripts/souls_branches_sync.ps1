<#
.SYNOPSIS
    Sincroniza as branches IDE/Solo persistentes com a última main.

.DESCRIPTION
    Rito canônico SOULS (R3 — Híbrido):
      - A3: Stash automático silencioso via `git rebase --autostash` (nativo git 2.14+).
      - B1: Rebase automático se a branch local divergiu de main.
      - C1: Push com `--force-with-lease` (seguro contra sobrescrita acidental).

    Para cada branch alvo, o script:
      1. Faz `git fetch` do remote.
      2. Detecta o estado em relação a `origin/main`:
         - UP-TO-DATE: nada a fazer.
         - FAST-FORWARD POSSÍVEL: `reset --hard origin/main` (sem perda).
         - DIVERGED: `rebase --autostash origin/main` (reaplica commits locais).
      3. Confirma interativamente antes de rebase divergente.
      4. Push com `--force-with-lease` para propagar.
      5. Retorna à branch original.

    Lei Zero-Slop:
      - NUNCA descarta trabalho: usa `git rebase --abort` em caso de conflito.
      - NUNCA força push sem lease: `--force-with-lease` aborta se o remote
        avançou sem você (evita sobrescrita acidental de PR de colega).
      - SEMPRE preserva uncommitted work via autostash nativo.

.PARAMETER TargetBranches
    Lista de branches a sincronizar. Default: TRAE-IDE, TRAE-SOLO,
    ANTIGRAVITY-IDE, ANTIGRAVITY-Solo (todos os workspaces IDE/Solo).

.PARAMETER MainBranch
    Branch fonte de verdade. Default: main.

.PARAMETER Remote
    Remote git. Default: origin.

.PARAMETER NoPush
    Se especificado, NÃO faz push. Útil para validar o sync local primeiro.

.PARAMETER NoConfirmRebase
    Se especificado, pula a confirmação interativa de rebase divergente.
    Default: false (sempre confirma).

.PARAMETER AutoYes
    Assume 'yes' para todos os prompts. Use em CI/automation. CUIDADO.

.EXAMPLE
    pwsh ./souls_branches_sync.ps1
    Sincroniza todas as branches IDE/Solo com main, com confirmação
    interativa para rebase divergente.

.EXAMPLE
    pwsh ./souls_branches_sync.ps1 -NoPush
    Apenas sincroniza localmente (sem propagar para origin). Bom para
    dry-run de grandes rebases.

.EXAMPLE
    pwsh ./souls_branches_sync.ps1 -TargetBranches TRAE-IDE,ANTIGRAVITY-IDE
    Sincroniza apenas 2 branches específicas.

.EXAMPLE
    pwsh ./souls_branches_sync.ps1 -AutoYes
    Modo "fire-and-forget" para automação noturna. ⚠️ destrutivo.

.NOTES
    Marco 3.9.2: scripts de higiene git para workspaces IDE/Solo.
    Autor: Antigravity/SOLO. Compatível com PowerShell 7+ em Windows.
#>

[CmdletBinding(SupportsShouldProcess = $true)]
param(
    [string[]]$TargetBranches = @('TRAE-IDE', 'TRAE-Solo', 'ANTIGRAVITY-IDE', 'ANTIGRAVITY-Solo'),
    [string]$MainBranch = 'main',
    [string]$Remote = 'origin',
    [switch]$NoPush = $false,
    [switch]$NoConfirmRebase = $false,
    [switch]$AutoYes = $false
)

# =============================================================================
# GUARD: pré-condições.
# =============================================================================

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest

if ($IsWindows -eq $false) {
    Write-Warning "Este script assume PowerShell 7+ no Windows. Continuando sob risco..."
}

if (-not (Get-Command git -ErrorAction SilentlyContinue)) {
    throw "git não encontrado no PATH. Instale git-for-windows."
}

# Working dir deve ser a raiz do repo.
$gitRoot = git rev-parse --show-toplevel 2>$null
if ($LASTEXITCODE -ne 0 -or [string]::IsNullOrEmpty($gitRoot)) {
    throw "Não estamos em um repositório git. Execute da raiz do projeto."
}
Set-Location -LiteralPath $gitRoot

# HEAD não pode estar detached.
$headRef = git symbolic-ref --short HEAD 2>$null
if ($LASTEXITCODE -ne 0) {
    throw "HEAD está detached. Faça checkout de uma branch antes de rodar o script."
}

Write-Host ""
Write-Host "=== SOULS BRANCHES SYNC (R3 — Híbrido) ===" -ForegroundColor Cyan
Write-Host "Repo:        $gitRoot" -ForegroundColor Gray
Write-Host "Remote:      $Remote" -ForegroundColor Gray
Write-Host "Main:        $MainBranch" -ForegroundColor Gray
Write-Host "Targets:     $($TargetBranches -join ', ')" -ForegroundColor Gray
Write-Host "Mode:        $(if ($NoPush) {'LOCAL-ONLY (no push)'} else {'LOCAL + PUSH (force-with-lease)'})" -ForegroundColor Gray
Write-Host "Rebase:      $(if ($NoConfirmRebase) {'AUTO'} else {'CONFIRM'})" -ForegroundColor Gray
Write-Host ""

# =============================================================================
# FUNÇÕES UTILITÁRIAS.
# =============================================================================

function Test-YesNo {
    param([string]$Prompt)
    if ($AutoYes) { return $true }
    if ($NoConfirmRebase) { return $true }
    while ($true) {
        $answer = Read-Host "$Prompt [y/N]"
        if ([string]::IsNullOrWhiteSpace($answer)) { return $false }
        switch ($answer.ToLower()) {
            'y' { return $true }
            'n' { return $false }
            'yes' { return $true }
            'no' { return $false }
        }
    }
}

function Get-BranchState {
    # Classifica o estado de uma branch em relação a origin/<main>.
    # Returns: 'UP-TO-DATE' | 'FAST-FORWARD' | 'DIVERGED' | 'BEHIND' | 'ERROR'
    param(
        [string]$Branch,
        [string]$MainRef
    )

    $localTip = git rev-parse --verify $Branch 2>$null
    if ($LASTEXITCODE -ne 0) { return 'ERROR' }

    $mainTip = git rev-parse --verify $MainRef 2>$null
    if ($LASTEXITCODE -ne 0) { return 'ERROR' }

    if ($localTip -eq $mainTip) {
        return 'UP-TO-DATE'
    }

    # merge-base: o ponto de divergência.
    $mergeBase = git merge-base $Branch $MainRef 2>$null
    if ($LASTEXITCODE -ne 0) { return 'ERROR' }

    if ($mergeBase -eq $localTip) {
        return 'FAST-FORWARD'   # local é ancestral de main
    }

    if ($mergeBase -eq $mainTip) {
        return 'BEHIND'         # main é ancestral de local (raro; main moveu backward)
    }

    return 'DIVERGED'           # ambos divergiram
}

function Get-DirtyFiles {
    # Detecta trabalho não commitado (modificados + staged + untracked).
    $dirty = git status --porcelain 2>$null
    if ([string]::IsNullOrEmpty($dirty)) {
        return @()
    }
    return $dirty -split "`n" | Where-Object { $_ -match '^(\?\?|.[MD])' }
}

function Get-AheadBehind {
    # Retorna @{ahead=N; behind=M} relativo a origin/main.
    param([string]$Branch)
    $counts = git rev-list --left-right --count "$Branch...$MainBranch" 2>$null
    if ($LASTEXITCODE -ne 0) { return $null }
    $parts = $counts -split "`t"
    return @{ ahead = [int]$parts[0]; behind = [int]$parts[1] }
}

# =============================================================================
# STEP 1: FETCH.
# =============================================================================

Write-Host "[1/3] Fetching $Remote..." -ForegroundColor Yellow
git fetch $Remote 2>&1 | Where-Object { $_ } | ForEach-Object {
    Write-Host "       $_" -ForegroundColor DarkGray
}
if ($LASTEXITCODE -ne 0) {
    throw "git fetch falhou (exit=$LASTEXITCODE). Verifique conectividade."
}
Write-Host ""

# =============================================================================
# STEP 2: VERIFICAR BRANCHES.
# =============================================================================

$mainRef = "$Remote/$MainBranch"
$mainExists = git rev-parse --verify $mainRef 2>$null
if ($LASTEXITCODE -ne 0) {
    throw "Remote main '$mainRef' não existe. Push main antes de sincronizar."
}

$localBranches = git branch --format='%(refname:short)' 2>$null | ForEach-Object { $_ }
$remoteBranches = git branch -r --format='%(refname:short)' 2>$null | ForEach-Object { $_ -replace "^$Remote/", "" }

$branchesToProcess = @()
foreach ($t in $TargetBranches) {
    if ($localBranches -contains $t) {
        $branchesToProcess += [pscustomobject]@{
            Name = $t
            Source = 'local'
        }
    } elseif ($remoteBranches -contains $t) {
        Write-Host "⚠️  Branch '$t' existe apenas em $Remote. Criando local tracking..." -ForegroundColor Yellow
        git branch --track $t "$Remote/$t" 2>&1 | Where-Object { $_ } | ForEach-Object {
            Write-Host "       $_" -ForegroundColor DarkGray
        }
        $branchesToProcess += [pscustomobject]@{
            Name = $t
            Source = 'remote-tracked'
        }
    } else {
        Write-Host "⚠️  Branch '$t' não existe nem em local nem em $Remote. Pulando." -ForegroundColor Yellow
    }
}

if ($branchesToProcess.Count -eq 0) {
    Write-Host "Nenhuma branch alvo encontrada. Saindo." -ForegroundColor Yellow
    exit 0
}

Write-Host "[2/3] Branches alvo: $($branchesToProcess.Name -join ', ')" -ForegroundColor Yellow
Write-Host ""

# =============================================================================
# STEP 3: PROCESSAR CADA BRANCH.
# =============================================================================

$originalBranch = $headRef
$results = @()

foreach ($branch in $branchesToProcess) {
    $name = $branch.Name
    Write-Host "─────────────────────────────────────────────" -ForegroundColor DarkGray
    Write-Host "→ Branch: $name" -ForegroundColor Cyan

    # 3.1 Checkout (se não estamos nela)
    if ($originalBranch -ne $name) {
        Write-Host "  · Checkout..." -ForegroundColor Gray
        git checkout $name 2>&1 | Where-Object { $_ } | ForEach-Object {
            Write-Host "    $_" -ForegroundColor DarkGray
        }
        if ($LASTEXITCODE -ne 0) {
            $results += [pscustomobject]@{ Branch = $name; Status = 'ERROR'; Detail = 'checkout falhou' }
            Write-Host "  ✗ Checkout falhou. Pulando branch." -ForegroundColor Red
            continue
        }
    }

    # 3.2 Detectar trabalho não commitado
    $dirty = Get-DirtyFiles
    if ($dirty.Count -gt 0) {
        Write-Host "  ⚠️  Trabalho não commitado detectado ($($dirty.Count) arquivos):" -ForegroundColor Yellow
        $dirty | ForEach-Object { Write-Host "    $_" -ForegroundColor DarkGray }
        $autoStash = Test-YesNo "  → Fazer autostash antes de sync?"
        if (-not $autoStash) {
            $results += [pscustomobject]@{ Branch = $name; Status = 'SKIPPED'; Detail = 'trabalho não commitado, usuário recusou stash' }
            Write-Host "  ⊘ Pulando (trabalho não commitado, usuário recusou stash)." -ForegroundColor Yellow
            continue
        }
        # Será stashed via --autostash no rebase OU stash manual antes do FF.
        $needsStash = $true
    } else {
        $needsStash = $false
    }

    # 3.3 Classificar estado
    $state = Get-BranchState -Branch $name -MainRef $mainRef
    $aheadBehind = Get-AheadBehind -Branch $name
    Write-Host "  · Estado: $state (ahead=$($aheadBehind.ahead), behind=$($aheadBehind.behind))" -ForegroundColor Gray

    switch ($state) {
        'UP-TO-DATE' {
            Write-Host "  ✓ Já está em sync com $mainRef. Nada a fazer." -ForegroundColor Green
            $results += [pscustomobject]@{ Branch = $name; Status = 'UP-TO-DATE'; Detail = "ahead=$($aheadBehind.ahead), behind=$($aheadBehind.behind)" }
            continue
        }
        'FAST-FORWARD' {
            Write-Host "  · Fast-forward possível (sem commits divergentes). Resetting..." -ForegroundColor Gray

            if ($needsStash) {
                $stashResult = git stash push -u -m "souls_branches_sync: $name pre-ff-$(Get-Date -Format yyyyMMdd-HHmmss)" 2>&1
                if ($LASTEXITCODE -ne 0) {
                    $results += [pscustomobject]@{ Branch = $name; Status = 'ERROR'; Detail = 'stash antes de FF falhou' }
                    Write-Host "  ✗ Stash falhou. Pulando." -ForegroundColor Red
                    continue
                }
            }

            git reset --hard $mainRef 2>&1 | Where-Object { $_ } | ForEach-Object {
                Write-Host "    $_" -ForegroundColor DarkGray
            }
            if ($LASTEXITCODE -ne 0) {
                $results += [pscustomobject]@{ Branch = $name; Status = 'ERROR'; Detail = 'reset --hard falhou' }
                Write-Host "  ✗ Reset falhou. Abortando." -ForegroundColor Red
                continue
            }

            if ($needsStash) {
                git stash pop 2>&1 | Where-Object { $_ } | ForEach-Object {
                    Write-Host "    $_" -ForegroundColor DarkGray
                }
                if ($LASTEXITCODE -ne 0) {
                    Write-Host "  ⚠️  Stash pop teve conflitos! Resolva manualmente." -ForegroundColor Yellow
                }
            }
        }
        'DIVERGED' {
            Write-Host "  · Branch divergiu de main ($($aheadBehind.ahead) ahead, $($aheadBehind.behind) behind)." -ForegroundColor Yellow
            $confirm = Test-YesNo "  → Rebase $name em $mainRef (reaplicar $($aheadBehind.ahead) commits)?"
            if (-not $confirm) {
                $results += [pscustomobject]@{ Branch = $name; Status = 'SKIPPED'; Detail = 'usuário recusou rebase divergente' }
                Write-Host "  ⊘ Pulando (rebase divergente recusado)." -ForegroundColor Yellow
                continue
            }

            Write-Host "  · Rebase --autostash em $mainRef..." -ForegroundColor Gray
            git -c rebase.autoStash=true rebase $mainRef 2>&1 | Where-Object { $_ } | ForEach-Object {
                Write-Host "    $_" -ForegroundColor DarkGray
            }
            $rebaseExit = $LASTEXITCODE
            if ($rebaseExit -ne 0) {
                Write-Host "  ✗ Rebase falhou (conflitos?). Abortando rebase..." -ForegroundColor Red
                git rebase --abort 2>&1 | Where-Object { $_ } | ForEach-Object {
                    Write-Host "    $_" -ForegroundColor DarkGray
                }
                $results += [pscustomobject]@{ Branch = $name; Status = 'CONFLICT'; Detail = "rebase falhou (exit=$rebaseExit), abortado" }
                continue
            }
        }
        'BEHIND' {
            # main moveu para trás (raro; alguém fez force-push em main).
            # Tratamos como diverged para forçar o rebase.
            Write-Host "  · $mainRef está atrás de $name (force-push em main?). Rebase necessário." -ForegroundColor Yellow
            git -c rebase.autoStash=true rebase $mainRef 2>&1 | Where-Object { $_ } | ForEach-Object {
                Write-Host "    $_" -ForegroundColor DarkGray
            }
            $rebaseExit = $LASTEXITCODE
            if ($rebaseExit -ne 0) {
                git rebase --abort 2>&1 | Where-Object { $_ } | ForEach-Object {
                    Write-Host "    $_" -ForegroundColor DarkGray
                }
                $results += [pscustomobject]@{ Branch = $name; Status = 'CONFLICT'; Detail = 'rebase falhou em estado BEHIND' }
                continue
            }
        }
        'ERROR' {
            $results += [pscustomobject]@{ Branch = $name; Status = 'ERROR'; Detail = 'estado indeterminável' }
            Write-Host "  ✗ Não foi possível determinar o estado. Pulando." -ForegroundColor Red
            continue
        }
    }

    # 3.4 Push (se aplicável)
    if ($NoPush) {
        Write-Host "  ⊘ Push suprimido (-NoPush). Branch local atualizada." -ForegroundColor Yellow
        $results += [pscustomobject]@{ Branch = $name; Status = 'OK-LOCAL-ONLY'; Detail = 'reset/rebase OK, push suprimido' }
        continue
    }

    Write-Host "  · Push com --force-with-lease..." -ForegroundColor Gray
    git push --force-with-lease $Remote $name 2>&1 | Where-Object { $_ } | ForEach-Object {
        Write-Host "    $_" -ForegroundColor DarkGray
    }
    $pushExit = $LASTEXITCODE
    if ($pushExit -ne 0) {
        $results += [pscustomobject]@{ Branch = $name; Status = 'PUSH-FAILED'; Detail = "push falhou (exit=$pushExit), reescreva manualmente se necessário" }
        Write-Host "  ✗ Push falhou. Use `git push --force-with-lease $Remote $name` para retry." -ForegroundColor Red
        continue
    }

    Write-Host "  ✓ Sincronizada com sucesso." -ForegroundColor Green
    $results += [pscustomobject]@{ Branch = $name; Status = 'OK'; Detail = "fast-forwarded/rebased + pushed" }
}

# =============================================================================
# STEP 4: RESTAURAR BRANCH ORIGINAL.
# =============================================================================

Write-Host ""
Write-Host "─────────────────────────────────────────────" -ForegroundColor DarkGray
if ($originalBranch) {
    Write-Host "[3/3] Restaurando branch original: $originalBranch" -ForegroundColor Yellow
    git checkout $originalBranch 2>&1 | Where-Object { $_ } | ForEach-Object {
        Write-Host "       $_" -ForegroundColor DarkGray
    }
}

# =============================================================================
# STEP 5: RELATÓRIO FINAL.
# =============================================================================

Write-Host ""
Write-Host "=== RELATÓRIO ===" -ForegroundColor Cyan

$okCount = ($results | Where-Object { $_.Status -in @('OK', 'UP-TO-DATE', 'OK-LOCAL-ONLY') }).Count
$errCount = ($results | Where-Object { $_.Status -in @('ERROR', 'CONFLICT', 'PUSH-FAILED') }).Count
$skipCount = ($results | Where-Object { $_.Status -eq 'SKIPPED' }).Count

$results | Sort-Object Branch | Format-Table -AutoSize Branch, Status, Detail | Out-String -Stream | ForEach-Object {
    if ($_ -match '(OK|UP-TO-DATE|OK-LOCAL-ONLY)') {
        Write-Host $_ -ForegroundColor Green
    } elseif ($_ -match '(CONFLICT|ERROR|PUSH-FAILED)') {
        Write-Host $_ -ForegroundColor Red
    } elseif ($_ -match 'SKIPPED') {
        Write-Host $_ -ForegroundColor Yellow
    } else {
        Write-Host $_
    }
}

Write-Host ""
Write-Host "Resumo: " -NoNewline
Write-Host "$okCount OK" -ForegroundColor Green -NoNewline
Write-Host " / " -NoNewline
Write-Host "$errCount ERROS" -ForegroundColor Red -NoNewline
Write-Host " / " -NoNewline
Write-Host "$skipCount PULOS" -ForegroundColor Yellow
Write-Host ""

if ($errCount -gt 0) {
    exit 1
}
exit 0
