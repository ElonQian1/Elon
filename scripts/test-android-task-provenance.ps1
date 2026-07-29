param()

$ErrorActionPreference = "Stop"
. (Join-Path $PSScriptRoot "android-task-provenance.ps1")

$testRoot = Join-Path ([System.IO.Path]::GetTempPath()) `
    ("elon-android-provenance-test-" + [Guid]::NewGuid().ToString("N"))
[System.IO.Directory]::CreateDirectory($testRoot) | Out-Null

function Invoke-TestGit {
    param([string[]]$GitArgs)
    & git -C $testRoot @GitArgs *> $null
    if ($LASTEXITCODE -ne 0) {
        throw "git $($GitArgs -join ' ') failed"
    }
}

try {
    Invoke-TestGit @("init", "-b", "main")
    Invoke-TestGit @("config", "user.email", "android-provenance@example.invalid")
    Invoke-TestGit @("config", "user.name", "android-provenance-test")

    Set-Content -LiteralPath (Join-Path $testRoot "state.txt") -Value "base" -Encoding UTF8
    Invoke-TestGit @("add", "state.txt")
    Invoke-TestGit @("commit", "-m", "base")
    $base = (& git -C $testRoot rev-parse HEAD).Trim()

    Add-Content -LiteralPath (Join-Path $testRoot "state.txt") -Value "task"
    Invoke-TestGit @("add", "state.txt")
    Invoke-TestGit @("commit", "-m", "task")
    $task = (& git -C $testRoot rev-parse HEAD).Trim()

    Add-Content -LiteralPath (Join-Path $testRoot "state.txt") -Value "newer main"
    Invoke-TestGit @("add", "state.txt")
    Invoke-TestGit @("commit", "-m", "newer main")
    $newer = (& git -C $testRoot rev-parse HEAD).Trim()

    $exact = Get-AndroidTaskPublicationProvenance -RepoPath $testRoot `
        -TaskHead $task -OriginMain $newer -PublishedSha $task
    if (-not $exact.TaskPushed -or -not $exact.PublishedContainsTask -or -not $exact.PublishedExactTask) {
        throw "Exact task APK provenance was rejected."
    }

    $newerRelease = Get-AndroidTaskPublicationProvenance -RepoPath $testRoot `
        -TaskHead $task -OriginMain $newer -PublishedSha $newer
    if (-not $newerRelease.TaskPushed -or -not $newerRelease.PublishedContainsTask) {
        throw "Newer APK containing the task commit was rejected."
    }

    $staleRelease = Get-AndroidTaskPublicationProvenance -RepoPath $testRoot `
        -TaskHead $task -OriginMain $newer -PublishedSha $base
    if ($staleRelease.PublishedContainsTask) {
        throw "Stale APK predating the task commit was accepted."
    }

    Write-Host "PASS Android task publication provenance"
} finally {
    $resolved = [System.IO.Path]::GetFullPath($testRoot)
    $tempRoot = [System.IO.Path]::GetFullPath([System.IO.Path]::GetTempPath())
    if ($resolved.StartsWith($tempRoot, [StringComparison]::OrdinalIgnoreCase) -and
        (Split-Path -Leaf $resolved).StartsWith("elon-android-provenance-test-")) {
        Remove-Item -LiteralPath $resolved -Recurse -Force
    }
}
