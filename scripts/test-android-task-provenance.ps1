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

    New-Item -ItemType Directory -Path (Join-Path $testRoot "android") | Out-Null
    New-Item -ItemType Directory -Path (Join-Path $testRoot "docs") | Out-Null
    Set-Content -LiteralPath (Join-Path $testRoot "android/app.txt") -Value "base" -Encoding UTF8
    Set-Content -LiteralPath (Join-Path $testRoot "docs/state.txt") -Value "base" -Encoding UTF8
    Invoke-TestGit @("add", "android/app.txt", "docs/state.txt")
    Invoke-TestGit @("commit", "-m", "base")
    $base = (& git -C $testRoot rev-parse HEAD).Trim()

    Add-Content -LiteralPath (Join-Path $testRoot "android/app.txt") -Value "task"
    Invoke-TestGit @("add", "android/app.txt")
    Invoke-TestGit @("commit", "-m", "task")
    $task = (& git -C $testRoot rev-parse HEAD).Trim()

    Add-Content -LiteralPath (Join-Path $testRoot "docs/state.txt") -Value "newer main"
    Invoke-TestGit @("add", "docs/state.txt")
    Invoke-TestGit @("commit", "-m", "newer main")
    $newer = (& git -C $testRoot rev-parse HEAD).Trim()

    $exact = Get-AndroidTaskPublicationProvenance -RepoPath $testRoot `
        -TaskHead $task -OriginMain $newer -PublishedSha $task
    if (-not $exact.TaskPushed -or -not $exact.PublishedContainsTask -or
        -not $exact.PublishedCoversTask -or -not $exact.PublishedExactTask) {
        throw "Exact task APK provenance was rejected."
    }

    $newerRelease = Get-AndroidTaskPublicationProvenance -RepoPath $testRoot `
        -TaskHead $task -OriginMain $newer -PublishedSha $newer
    if (-not $newerRelease.TaskPushed -or -not $newerRelease.PublishedContainsTask -or
        -not $newerRelease.PublishedCoversTask) {
        throw "Newer APK containing the task commit was rejected."
    }

    $publishedBeforeDocs = Get-AndroidTaskPublicationProvenance -RepoPath $testRoot `
        -TaskHead $newer -OriginMain $newer -PublishedSha $task
    if ($publishedBeforeDocs.PublishedContainsTask -or -not $publishedBeforeDocs.PublishedCoversTask -or
        $publishedBeforeDocs.CoverageReason -ne "same_android_inputs") {
        throw "A docs-only commit after APK publication was not covered by identical Android inputs."
    }

    $staleRelease = Get-AndroidTaskPublicationProvenance -RepoPath $testRoot `
        -TaskHead $task -OriginMain $newer -PublishedSha $base
    if ($staleRelease.PublishedCoversTask -or
        $staleRelease.ChangedAndroidPaths -notcontains "android/app.txt") {
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
