param()

$ErrorActionPreference = 'Stop'
$repoRoot = (& git rev-parse --show-toplevel 2>&1).Trim()
if ($LASTEXITCODE -ne 0) { throw 'Run inside the repository.' }

. (Join-Path $repoRoot 'scripts\app-ui-change-scope.ps1')

function Invoke-TestGit {
    param([Parameter(Mandatory = $true)][string]$Root, [Parameter(Mandatory = $true)][string[]]$Arguments)
    & git -C $Root @Arguments 2>&1 | Out-Null
    if ($LASTEXITCODE -ne 0) { throw "git $($Arguments -join ' ') failed in fixture" }
}

function Assert-Equal {
    param($Actual, $Expected, [string]$Message)
    if ($Actual -ne $Expected) { throw "$Message expected=$Expected actual=$Actual" }
}

$tempParent = [System.IO.Path]::GetFullPath([System.IO.Path]::GetTempPath())
$fixtureRoot = Join-Path $tempParent ("elon-app-ui-push-scope-" + [Guid]::NewGuid().ToString('N'))
New-Item -ItemType Directory -Path $fixtureRoot | Out-Null
try {
    Invoke-TestGit -Root $fixtureRoot -Arguments @('init', '-q')
    Invoke-TestGit -Root $fixtureRoot -Arguments @('config', 'user.name', 'App UI Scope Test')
    Invoke-TestGit -Root $fixtureRoot -Arguments @('config', 'user.email', 'app-ui-scope@example.invalid')

    New-Item -ItemType Directory -Path (Join-Path $fixtureRoot 'android\app') -Force | Out-Null
    Set-Content -LiteralPath (Join-Path $fixtureRoot 'README.md') -Value 'base' -Encoding UTF8
    Set-Content -LiteralPath (Join-Path $fixtureRoot 'android\app\obsolete.txt') -Value 'obsolete' -Encoding UTF8
    Invoke-TestGit -Root $fixtureRoot -Arguments @('add', 'README.md', 'android/app/obsolete.txt')
    Invoke-TestGit -Root $fixtureRoot -Arguments @('commit', '-q', '-m', 'base')
    $taskBaseSha = (& git -C $fixtureRoot rev-parse HEAD).Trim()
    $taskMarker = Join-Path (Get-ElonRepositoryPathsFromRoot -RepoRoot $fixtureRoot).GitDir 'elon-task-base.v1'
    [System.IO.File]::WriteAllText($taskMarker, "$taskBaseSha`n", [System.Text.Encoding]::ASCII)

    New-Item -ItemType Directory -Path (Join-Path $fixtureRoot 'server\src') -Force | Out-Null
    Set-Content -LiteralPath (Join-Path $fixtureRoot 'server\src\runtime.rs') -Value 'upstream' -Encoding UTF8
    Invoke-TestGit -Root $fixtureRoot -Arguments @('add', 'server/src/runtime.rs')
    Invoke-TestGit -Root $fixtureRoot -Arguments @('commit', '-q', '-m', 'unrelated upstream server change')
    $upstreamSha = (& git -C $fixtureRoot rev-parse HEAD).Trim()

    New-Item -ItemType Directory -Path (Join-Path $fixtureRoot 'server\src\assets') -Force | Out-Null
    New-Item -ItemType Directory -Path (Join-Path $fixtureRoot 'android\app') -Force | Out-Null
    New-Item -ItemType Directory -Path (Join-Path $fixtureRoot 'docs') -Force | Out-Null
    $unicodeDocName = '{0}{1}{2}{3}.md' -f [char]0x989C, [char]0x8272, [char]0x89C4, [char]0x8303
    $unicodeDocPath = "docs/$unicodeDocName"
    Set-Content -LiteralPath (Join-Path $fixtureRoot 'server\src\assets\web_page.html') -Value '<main>glass</main>' -Encoding UTF8
    Set-Content -LiteralPath (Join-Path $fixtureRoot 'server\src\assets\orbital_mobile_theme.css') -Value '.glass {}' -Encoding UTF8
    Set-Content -LiteralPath (Join-Path $fixtureRoot 'android\app\glass.txt') -Value 'glass' -Encoding UTF8
    Set-Content -LiteralPath (Join-Path $fixtureRoot $unicodeDocPath) -Value 'dark glass' -Encoding UTF8
    Remove-Item -LiteralPath (Join-Path $fixtureRoot 'android\app\obsolete.txt') -Force
    Invoke-TestGit -Root $fixtureRoot -Arguments @('add', '-A')
    Invoke-TestGit -Root $fixtureRoot -Arguments @('commit', '-q', '-m', 'task UI change after rebase')
    $firstTaskHead = (& git -C $fixtureRoot rev-parse HEAD).Trim()
    Invoke-TestGit -Root $fixtureRoot -Arguments @('update-ref', 'refs/remotes/origin/main', $upstreamSha)

    $incorrectScope = Resolve-ElonAppUiChangeScope `
        -RepoRoot $fixtureRoot -BaseSha $taskBaseSha -HeadSha $firstTaskHead
    Assert-Equal $incorrectScope.MobilePwaMode 'full_server' 'Preflight-only scope must reproduce the rebase bug'

    $candidate = New-ElonAppUiPushScopeCandidate `
        -RepoRoot $fixtureRoot -GitArgs @('push', 'origin', 'HEAD:main') -RemoteName 'origin'
    Assert-Equal $candidate.ScopeBaseSha $upstreamSha 'Successful push candidate must start after unrelated upstream history'
    Assert-Equal @($candidate.ChangedPaths | Where-Object { $_ -eq $unicodeDocPath }).Count 1 `
        'Successful push paths must preserve Unicode names without Git quoting'
    Assert-Equal @($candidate.ChangedPaths | Where-Object { $_ -eq 'android/app/obsolete.txt' }).Count 1 `
        'Successful push paths must retain deleted files for scope classification'
    Save-ElonAppUiTaskPushScope -RepoRoot $fixtureRoot -Candidate $candidate

    $taskBase = [PSCustomObject]@{ Sha = $taskBaseSha; Source = 'preflight_marker' }
    $scopeBase = Get-ElonAppUiTaskScopeBaseSha `
        -RepoRoot $fixtureRoot -TaskBase $taskBase -HeadSha $firstTaskHead
    Assert-Equal $scopeBase.Sha $upstreamSha 'Task scope must use the successful push base'
    Assert-Equal $scopeBase.Source 'successful_push_marker' 'Task scope must expose marker provenance'
    $ownedScope = Resolve-ElonAppUiChangeScope `
        -RepoRoot $fixtureRoot -BaseSha $scopeBase.Sha -HeadSha $firstTaskHead `
        -ChangedPaths @($scopeBase.ChangedPaths)
    Assert-Equal $ownedScope.MobilePwaMode 'static_template' 'Unrelated upstream server code must not trigger a full server release'
    Assert-Equal $ownedScope.OtherServerChanges.Count 0 'Task-owned server runtime paths must stay empty'
    Assert-Equal $ownedScope.AndroidChanged $true 'Task-owned Android changes must remain visible'

    Set-Content -LiteralPath (Join-Path $fixtureRoot 'server\src\interleaved.rs') -Value 'other task' -Encoding UTF8
    Invoke-TestGit -Root $fixtureRoot -Arguments @('add', 'server/src/interleaved.rs')
    Invoke-TestGit -Root $fixtureRoot -Arguments @('commit', '-q', '-m', 'interleaved upstream task')
    $interleavedUpstreamSha = (& git -C $fixtureRoot rev-parse HEAD).Trim()
    Invoke-TestGit -Root $fixtureRoot -Arguments @('update-ref', 'refs/remotes/origin/main', $interleavedUpstreamSha)

    Set-Content -LiteralPath (Join-Path $fixtureRoot 'README.md') -Value 'second task commit' -Encoding UTF8
    Invoke-TestGit -Root $fixtureRoot -Arguments @('add', 'README.md')
    Invoke-TestGit -Root $fixtureRoot -Arguments @('commit', '-q', '-m', 'second task commit')
    $secondTaskHead = (& git -C $fixtureRoot rev-parse HEAD).Trim()
    $secondCandidate = New-ElonAppUiPushScopeCandidate `
        -RepoRoot $fixtureRoot -GitArgs @('push', 'origin', 'HEAD:main') -RemoteName 'origin'
    Save-ElonAppUiTaskPushScope -RepoRoot $fixtureRoot -Candidate $secondCandidate
    $secondScopeBase = Get-ElonAppUiTaskScopeBaseSha `
        -RepoRoot $fixtureRoot -TaskBase $taskBase -HeadSha $secondTaskHead
    Assert-Equal $secondScopeBase.Sha $upstreamSha 'Multiple successful task pushes must retain the first task-owned base'
    $secondOwnedScope = Resolve-ElonAppUiChangeScope `
        -RepoRoot $fixtureRoot -BaseSha $secondScopeBase.Sha -HeadSha $secondTaskHead `
        -ChangedPaths @($secondScopeBase.ChangedPaths)
    Assert-Equal $secondOwnedScope.MobilePwaMode 'static_template' 'Interleaved upstream server commits must stay outside task-owned paths'
    Assert-Equal $secondOwnedScope.OtherServerChanges.Count 0 'Interleaved upstream server paths must not enter the task path union'

    Write-Host 'APP_UI_TASK_PUSH_SCOPE_TESTS=passed'
} finally {
    $resolvedFixture = [System.IO.Path]::GetFullPath($fixtureRoot)
    if (
        $resolvedFixture.StartsWith($tempParent, [System.StringComparison]::OrdinalIgnoreCase) -and
        (Split-Path $resolvedFixture -Leaf) -like 'elon-app-ui-push-scope-*'
    ) {
        Remove-Item -LiteralPath $resolvedFixture -Recurse -Force -ErrorAction SilentlyContinue
    }
}
