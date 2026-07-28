$ErrorActionPreference = 'Stop'
. (Join-Path $PSScriptRoot 'apk-release-freshness.ps1')

function Assert-True {
    param([bool]$Condition, [string]$Message)
    if (-not $Condition) { throw "ASSERT FAILED: $Message" }
}

$root = Join-Path ([System.IO.Path]::GetTempPath()) ("elon-apk-freshness-" + [Guid]::NewGuid())
New-Item -ItemType Directory -Path $root -Force | Out-Null
try {
    git -C $root init --quiet
    git -C $root config user.email 'test@elon.local'
    git -C $root config user.name 'Elon Test'
    New-Item -ItemType Directory -Path (Join-Path $root 'android') -Force | Out-Null
    New-Item -ItemType Directory -Path (Join-Path $root 'docs') -Force | Out-Null

    Set-Content -LiteralPath (Join-Path $root 'android/app.txt') -Value 'A' -Encoding UTF8
    git -C $root add android/app.txt
    git -C $root commit --quiet -m 'A'
    $shaA = (git -C $root rev-parse HEAD).Trim()

    Set-Content -LiteralPath (Join-Path $root 'docs/note.txt') -Value 'B' -Encoding UTF8
    git -C $root add docs/note.txt
    git -C $root commit --quiet -m 'B docs only'
    $shaB = (git -C $root rev-parse HEAD).Trim()

    Set-Content -LiteralPath (Join-Path $root 'android/app.txt') -Value 'C' -Encoding UTF8
    git -C $root add android/app.txt
    git -C $root commit --quiet -m 'C android'
    $shaC = (git -C $root rev-parse HEAD).Trim()

    $docsOnly = Get-ElonApkInputCoverage -RepoRoot $root -CandidateSha $shaB -DeployedSha $shaA
    Assert-True $docsOnly.Covered 'docs-only descendants must reuse the deployed APK'
    Assert-True ($docsOnly.Reason -eq 'same_android_inputs') 'docs-only coverage reason'

    $androidChanged = Get-ElonApkInputCoverage -RepoRoot $root -CandidateSha $shaC -DeployedSha $shaA
    Assert-True (-not $androidChanged.Covered) 'Android changes must require a new APK'
    Assert-True ($androidChanged.ChangedPaths -contains 'android/app.txt') 'Android diff evidence'

    $newerAlreadyPublished = Get-ElonApkInputCoverage -RepoRoot $root -CandidateSha $shaA -DeployedSha $shaC
    Assert-True $newerAlreadyPublished.Covered 'a deployed descendant covers an older candidate'

    $skipQueuedB = Get-ElonApkBuildStartDecision -RepoRoot $root -CandidateSha $shaB -CurrentMainSha $shaC
    Assert-True (-not $skipQueuedB.Build) 'an unbuilt queued generation must yield to newer Android main'

    $buildCurrent = Get-ElonApkBuildStartDecision -RepoRoot $root -CandidateSha $shaC -CurrentMainSha $shaC
    Assert-True $buildCurrent.Build 'current main must build'

    $windowsPublisher = Get-Content -LiteralPath (Join-Path $PSScriptRoot 'publish-apk.ps1') -Raw
    $shellPublisher = Get-Content -LiteralPath (Join-Path $PSScriptRoot 'publish-apk.sh') -Raw
    $leaseHelper = Get-Content -LiteralPath (Join-Path $PSScriptRoot 'release-publish-lease.ps1') -Raw
    Assert-True ($windowsPublisher.Contains('Get-ElonApkInputCoverage')) 'Windows publisher must dedupe before claim'
    Assert-True ($windowsPublisher.Contains('Get-ElonApkBuildStartDecision')) 'Windows publisher must skip stale unbuilt generations'
    Assert-True ($windowsPublisher.Contains('Test-GitAncestor -Ancestor $BuildBaseSha -Descendant $freshness.RemoteHead')) 'Windows publisher must preserve a completed A build'
    Assert-True ($shellPublisher.Contains('superseded before build')) 'shell publisher must skip stale unbuilt generations'
    Assert-True ($shellPublisher.Contains('elif is_git_ancestor "$SHA_FULL" "$REMOTE_HEAD_NOW"; then')) 'shell publisher must preserve a completed A build'
    Assert-True (-not $leaseHelper.Contains('14400')) 'publisher heartbeat leases must not survive a power loss for four hours'

    Write-Host 'APK_RELEASE_FRESHNESS_TEST=passed'
} finally {
    if (Test-Path -LiteralPath $root) {
        Get-ChildItem -LiteralPath $root -Force -Recurse -ErrorAction SilentlyContinue |
            ForEach-Object { $_.Attributes = 'Normal' }
        Remove-Item -LiteralPath $root -Recurse -Force
    }
}
