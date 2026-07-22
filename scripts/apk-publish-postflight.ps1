Set-StrictMode -Version Latest

function Invoke-ElonApkWorktreeCleanup {
    param([Parameter(Mandatory)][string]$RepoRoot)

    $cleanupScript = Join-Path $RepoRoot 'scripts\cleanup-task-worktrees.ps1'
    if (-not (Test-Path -LiteralPath $cleanupScript)) { return }
    try {
        $cleanupOut = & powershell -NoProfile -ExecutionPolicy Bypass -File $cleanupScript -Apply 2>&1
        $cleanupMarker = '^' + (-join ([char]0x5b8c, [char]0x6210, [char]0xff1a, [char]0x6e05, [char]0x7406))
        $removedLine = $cleanupOut | Select-String -Pattern $cleanupMarker | Select-Object -Last 1
        if ($removedLine) {
            Write-Host "   $($removedLine.Line.Trim()) (auto)" -ForegroundColor DarkGray
        }
    } catch {
        Write-Host "   Worktree auto-cleanup failed: $_" -ForegroundColor Yellow
    }
}

function Invoke-ElonApkPublishPostflight {
    param(
        [Parameter(Mandatory)][string]$RepoRoot,
        [Parameter(Mandatory)][string]$ApkPath,
        [Parameter(Mandatory)][int]$ExpectedVersionCode
    )

    Invoke-ElonApkWorktreeCleanup -RepoRoot $RepoRoot
    . (Join-Path $PSScriptRoot 'apk-adb-autodeploy.ps1')
    Invoke-ElonApkAdbAutodeploy -ApkPath $ApkPath -ExpectedVersionCode $ExpectedVersionCode | Out-Null
}
