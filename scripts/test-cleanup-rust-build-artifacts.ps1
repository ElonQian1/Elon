$ErrorActionPreference = "Stop"

$script:Assertions = 0
function Assert-True {
    param([bool]$Condition, [string]$Message)
    $script:Assertions++
    if (-not $Condition) { throw "Assertion failed: $Message" }
}

$scriptPath = Join-Path $PSScriptRoot "cleanup-rust-build-artifacts.ps1"
$tempRoot = Join-Path ([System.IO.Path]::GetTempPath()) ("elon-rust-target-cleanup-test-" + [Guid]::NewGuid().ToString("N"))
try {
    $repoRoot = Join-Path $tempRoot "Elon"
    $peerRoot = Join-Path $tempRoot "Elon-peer"
    $orphanRoot = Join-Path $tempRoot "Elon-task-orphan"
    $legacyTarget = Join-Path $tempRoot "legacy-target"
    $ordinaryDirectory = Join-Path $tempRoot "ordinary-directory"
    New-Item -ItemType Directory -Force -Path (Join-Path $repoRoot "target\rust-test-fixture"), (Join-Path $repoRoot "server\target\release"), (Join-Path $orphanRoot "target\rust-check-fixture"), (Join-Path $legacyTarget "release"), $ordinaryDirectory | Out-Null
    Set-Content -LiteralPath (Join-Path $repoRoot "target\rust-test-fixture\artifact.bin") -Value "target"
    Set-Content -LiteralPath (Join-Path $repoRoot "server\target\release\artifact.bin") -Value "server-target"
    Set-Content -LiteralPath (Join-Path $orphanRoot "target\rust-check-fixture\artifact.bin") -Value "orphan-target"
    Set-Content -LiteralPath (Join-Path $legacyTarget "CACHEDIR.TAG") -Value "Signature: 8a477f597d28d172789f06886806bc55"
    Set-Content -LiteralPath (Join-Path $legacyTarget "release\\artifact.bin") -Value "legacy-target"
    Set-Content -LiteralPath (Join-Path $ordinaryDirectory "keep.txt") -Value "must survive"
    & git init --quiet $repoRoot
    if ($LASTEXITCODE -ne 0) { throw "git init fixture failed" }
    & git -C $repoRoot config user.email "cleanup-test@example.invalid"
    & git -C $repoRoot config user.name "Rust cleanup test"
    Set-Content -LiteralPath (Join-Path $repoRoot "fixture.txt") -Value "fixture"
    & git -C $repoRoot add fixture.txt
    & git -C $repoRoot commit --quiet -m "fixture"
    if ($LASTEXITCODE -ne 0) { throw "git commit fixture failed" }
    & git -C $repoRoot worktree add --detach --quiet $peerRoot HEAD
    if ($LASTEXITCODE -ne 0) { throw "git worktree fixture failed" }
    New-Item -ItemType Directory -Force -Path (Join-Path $peerRoot "target") | Out-Null
    Set-Content -LiteralPath (Join-Path $peerRoot "target\keep.txt") -Value "not-cargo"

    $preview = & powershell -NoProfile -ExecutionPolicy Bypass -File $scriptPath -RepoRoot $peerRoot -WorkspaceRoot $tempRoot -AdditionalTarget $legacyTarget 2>&1
    if ($LASTEXITCODE -ne 0) { throw "preview failed: $($preview -join "`n")" }
    $previewText = $preview -join "`n"
    Assert-True ($previewText.Contains("RUST_TARGET_CLEANUP_RESULT=preview")) "preview must report preview mode"
    Assert-True ($previewText.Contains($repoRoot + "\target")) "preview must include the root target"
    Assert-True ($previewText.Contains($orphanRoot + "\target")) "preview must include the orphan target"
    Assert-True (Test-Path -LiteralPath (Join-Path $repoRoot "target")) "preview must not remove root target"

    $rejected = $false
    try {
        & powershell -NoProfile -ExecutionPolicy Bypass -File $scriptPath -RepoRoot $peerRoot -WorkspaceRoot $tempRoot -AdditionalTarget $ordinaryDirectory *> $null
        $rejected = $LASTEXITCODE -ne 0
    } catch {
        $rejected = $true
    }
    Assert-True $rejected "ordinary directories must not be accepted as additional targets"

    $apply = & powershell -NoProfile -ExecutionPolicy Bypass -File $scriptPath -Apply -RepoRoot $peerRoot -WorkspaceRoot $tempRoot -AdditionalTarget $legacyTarget 2>&1
    if ($LASTEXITCODE -ne 0) { throw "apply failed: $($apply -join "`n")" }
    $applyText = $apply -join "`n"
    Assert-True ($applyText.Contains("RUST_TARGET_CLEANUP_RESULT=applied")) "apply must report applied mode"
    Assert-True (-not (Test-Path -LiteralPath (Join-Path $repoRoot "target"))) "root target must be removed"
    Assert-True (-not (Test-Path -LiteralPath (Join-Path $repoRoot "server\target"))) "server target must be removed"
    Assert-True (-not (Test-Path -LiteralPath (Join-Path $orphanRoot "target"))) "orphan target must be removed"
    Assert-True (-not (Test-Path -LiteralPath $legacyTarget)) "explicit legacy target must be removed"
    Assert-True (Test-Path -LiteralPath (Join-Path $peerRoot "target\keep.txt")) "non-Cargo target-named directory must survive"
    Assert-True (Test-Path -LiteralPath (Join-Path $ordinaryDirectory "keep.txt")) "ordinary directory must survive"

    Write-Host "PASS: Rust build artifact cleanup ($script:Assertions assertions)" -ForegroundColor Green
} finally {
    if (Test-Path -LiteralPath $tempRoot) {
        Remove-Item -LiteralPath $tempRoot -Force -Recurse
    }
}
