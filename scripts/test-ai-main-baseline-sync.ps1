param()

$ErrorActionPreference = 'Stop'
$repoRoot = Split-Path -Parent $PSScriptRoot
$testRoot = Join-Path ([IO.Path]::GetTempPath()) ('elon-main-sync-test-' + [Guid]::NewGuid().ToString('N'))
$seed = Join-Path $testRoot 'seed'
$remote = Join-Path $testRoot 'remote.git'
$baseline = Join-Path $testRoot 'baseline'
$tasks = Join-Path $testRoot 'tasks'

function Invoke-FixtureGit {
    param([string]$Root, [string[]]$Arguments)
    $previous = $ErrorActionPreference
    $ErrorActionPreference = 'Continue'
    try { $output = @(& git -C $Root @Arguments 2>&1); $code = $LASTEXITCODE }
    finally { $ErrorActionPreference = $previous }
    if ($code -ne 0) { throw "Fixture git failed: $($output -join ' ')" }
    return ($output -join "`n").Trim()
}

function Preflight {
    param([switch]$Create)
    $arguments = @('-WindowStyle','Hidden','-NoProfile','-ExecutionPolicy','Bypass','-File',
        (Join-Path $baseline 'scripts\ai-task-preflight.ps1'),'-SkipAutoCleanup','-WorktreeParent',$tasks)
    if ($Create) { $arguments += '-CreateWorktree' }
    Push-Location -LiteralPath $baseline
    $previous = $ErrorActionPreference
    $ErrorActionPreference = 'Continue'
    try { $output = @(& powershell @arguments 2>&1); $code = $LASTEXITCODE }
    finally { $ErrorActionPreference = $previous; Pop-Location }
    [pscustomobject]@{Code=$code;Text=($output -join "`n")}
}

function Check([bool]$Condition, [string]$Message) {
    if (-not $Condition) { throw $Message }
}

try {
    New-Item -ItemType Directory -Path $seed -Force | Out-Null
    Invoke-FixtureGit $seed @('init','-b','main') | Out-Null
    Invoke-FixtureGit $seed @('config','user.name','baseline-sync-test') | Out-Null
    Invoke-FixtureGit $seed @('config','user.email','baseline-sync@example.invalid') | Out-Null
    Invoke-FixtureGit $seed @('config','core.autocrlf','false') | Out-Null
    New-Item -ItemType Directory -Path (Join-Path $seed 'scripts') | Out-Null
    foreach ($script in @('ai-task-preflight.ps1','ai-task-finish-contract.ps1',
        'app-ui-task-push-scope.ps1','git-path-resolution.ps1','direct-network.ps1')) {
        Copy-Item -LiteralPath (Join-Path $PSScriptRoot $script) -Destination (Join-Path $seed "scripts\$script")
    }
    Set-Content -LiteralPath (Join-Path $seed 'README.md') -Value 'initial'
    Invoke-FixtureGit $seed @('add','.') | Out-Null
    Invoke-FixtureGit $seed @('commit','-m','seed') | Out-Null
    Invoke-FixtureGit $testRoot @('init','--bare',$remote) | Out-Null
    Invoke-FixtureGit $seed @('remote','add','origin',$remote) | Out-Null
    Invoke-FixtureGit $seed @('push','origin','main') | Out-Null
    Invoke-FixtureGit $testRoot @('clone','-b','main',$remote,$baseline) | Out-Null
    Invoke-FixtureGit $baseline @('config','core.autocrlf','false') | Out-Null

    # A real checkout collision must stop before worktree creation, preserving both copies.
    $collision = Join-Path $baseline 'collision.txt'
    Set-Content -LiteralPath $collision -Value 'unknown local work'
    $before = (Get-FileHash -LiteralPath $collision).Hash
    $oldHead = Invoke-FixtureGit $baseline @('rev-parse','HEAD')
    Set-Content -LiteralPath (Join-Path $seed 'collision.txt') -Value 'upstream work'
    Invoke-FixtureGit $seed @('add','collision.txt') | Out-Null
    Invoke-FixtureGit $seed @('commit','-m','introduce collision') | Out-Null
    Invoke-FixtureGit $seed @('push','origin','main') | Out-Null
    $failure = Preflight -Create
    Check ($failure.Code -ne 0) 'Failed main sync must stop preflight with nonzero exit.'
    Check ($failure.Text.Contains('LOCAL_MAIN_RECOVERY_REQUIRED=true')) 'Failure must name local recovery.'
    Check ($failure.Text.Contains('EDIT_ROOT=BLOCKED_MAIN_BASELINE_SYNC_FAILED')) 'Failure must withhold an edit root.'
    Check (-not $failure.Text.Contains('WORKTREE_CREATED=true')) 'Failure must not issue a task worktree.'
    Check ((Invoke-FixtureGit $baseline @('rev-parse','HEAD')) -eq $oldHead) 'Collision must preserve HEAD.'
    Check ((Get-FileHash -LiteralPath $collision).Hash -eq $before) 'Collision must preserve unknown local bytes.'
    Check (@((Invoke-FixtureGit $baseline @('worktree','list','--porcelain')) -split "`n" | Where-Object {$_ -like 'worktree *'}).Count -eq 1) 'Failure created a worktree.'
    Write-Host 'PASS failed sync stops before task creation and preserves local files'

    $gitRoot = Split-Path -Parent (Split-Path -Parent (Get-Command git.exe).Source)
    $bash = Join-Path $gitRoot 'bin\bash.exe'
    if (Test-Path -LiteralPath $bash) {
        $shellScript = (Join-Path $PSScriptRoot 'ai-task-preflight.sh').Replace('\','/')
        $shellRunner = Join-Path $testRoot 'run-shell.sh'
        [IO.File]::WriteAllText($shellRunner, 'set -eu' + "`n" +
            'cd -- "$1"' + "`n" + 'test "$(git rev-parse HEAD)" = "$4"' + "`n" +
            'exec bash "$2" --create-worktree --worktree-parent "$3" --skip-auto-cleanup' + "`n")
        $previous = $ErrorActionPreference
        $ErrorActionPreference = 'Continue'
        try {
            $shellOutput = & $bash --noprofile --norc $shellRunner.Replace('\','/') `
                $baseline.Replace('\','/') $shellScript $tasks.Replace('\','/') $oldHead 2>&1
            $shellCode = $LASTEXITCODE
        } finally { $ErrorActionPreference = $previous }
        Check ($shellCode -ne 0) "Shell sync failure must return nonzero. $($shellOutput -join ' ')"
        Check (($shellOutput -join "`n").Contains('EDIT_ROOT=BLOCKED_MAIN_BASELINE_SYNC_FAILED')) "Shell failure must withhold an edit root. $($shellOutput -join ' ')"
        Check ((Get-FileHash -LiteralPath $collision).Hash -eq $before) 'Shell failure changed unknown local bytes.'
        Check (@((Invoke-FixtureGit $baseline @('worktree','list','--porcelain')) -split "`n" | Where-Object {$_ -like 'worktree *'}).Count -eq 1) 'Shell failure created a worktree.'
        Write-Host 'PASS Git Bash sync failure stops and preserves local files'
    } else { Write-Host 'SKIP Git Bash executable unavailable' }

    # Remove only the exact fixture file; ordinary unrelated untracked files must survive sync.
    Remove-Item -LiteralPath $collision
    $unrelated = Join-Path $baseline 'unrelated.txt'
    Set-Content -LiteralPath $unrelated -Value 'keep me'
    $success = Preflight
    Check ($success.Code -eq 0) "Clean sync failed: $($success.Text)"
    Check ((Invoke-FixtureGit $baseline @('rev-parse','HEAD')) -eq (Invoke-FixtureGit $seed @('rev-parse','HEAD'))) 'Main did not fast-forward.'
    Check ((Get-Content -LiteralPath $unrelated -Raw).Trim() -eq 'keep me') 'Unrelated file was changed.'
    Write-Host 'PASS clean sync preserves unrelated untracked files'

    # Build the long path in Git's index so Windows PowerShell filesystem limits cannot hide the test.
    $longPath = ((1..9 | ForEach-Object {'long-path-segment-1234567890'}) -join '/') + '/evidence.txt'
    $blob = ('long path payload' | & git -C $seed hash-object -w --stdin).Trim()
    Check ($LASTEXITCODE -eq 0) 'Unable to create fixture blob.'
    Invoke-FixtureGit $seed @('update-index','--add','--cacheinfo',"100644,$blob,$longPath") | Out-Null
    Invoke-FixtureGit $seed @('commit','-m','long path') | Out-Null
    Invoke-FixtureGit $seed @('push','origin','main') | Out-Null
    Invoke-FixtureGit $baseline @('config','core.longpaths','false') | Out-Null
    $longResult = Preflight
    Check ($longResult.Code -eq 0) "Long-path sync failed: $($longResult.Text)"
    Check ((Invoke-FixtureGit $baseline @('-c','core.longpaths=true','hash-object','--',$longPath)) -eq $blob) 'Long-path checkout bytes differ.'
    Check ((Invoke-FixtureGit $baseline @('config','core.longpaths')) -eq 'false') 'Sync must not alter the user Git configuration.'
    # Status itself needs long-path support; the production operation did not persist configuration.
    Invoke-FixtureGit $baseline @('config','core.longpaths','true') | Out-Null
    Write-Host 'PASS long-path sync succeeds with core.longpaths=false and leaves configuration unchanged'

    Set-Content -LiteralPath (Join-Path $baseline 'README.md') -Value 'unrelated tracked edit'
    $dirty = Preflight
    Check ($dirty.Text.Contains('LOCAL_MAIN_RECOVERY_REQUIRED=true')) 'Existing dirty main must remain an explicit recovery item.'
    Check ((Get-Content -LiteralPath (Join-Path $baseline 'README.md') -Raw).Trim() -eq 'unrelated tracked edit') 'Tracked work was overwritten.'
    Write-Host 'PASS existing tracked work is preserved and recovery is surfaced'
} finally {
    $absolute = [IO.Path]::GetFullPath($testRoot)
    $tempPrefix = [IO.Path]::GetFullPath([IO.Path]::GetTempPath()).TrimEnd('\') + '\'
    if (-not $absolute.StartsWith($tempPrefix, [StringComparison]::OrdinalIgnoreCase) -or
        (Split-Path -Leaf $absolute) -notlike 'elon-main-sync-test-*') { throw 'Unsafe fixture cleanup path.' }
    # Git removes its long-path tracked fixture using long-path support before PowerShell cleanup.
    if (Test-Path -LiteralPath (Join-Path $baseline '.git')) {
        & git -C $baseline -c core.longpaths=true clean -fd *> $null
        & git -C $baseline -c core.longpaths=true rm -r -f --ignore-unmatch . *> $null
    }
    if (Test-Path -LiteralPath $absolute) { Remove-Item -LiteralPath $absolute -Recurse -Force }
}
