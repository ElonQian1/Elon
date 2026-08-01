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
        [Parameter(Mandatory)][int]$ExpectedVersionCode,
        [switch]$AllowAdbVerificationDeferred
    )

    Invoke-ElonApkWorktreeCleanup -RepoRoot $RepoRoot
    . (Join-Path $PSScriptRoot 'apk-adb-autodeploy.ps1')
    try {
        Invoke-ElonApkAdbAutodeploy -ApkPath $ApkPath -ExpectedVersionCode $ExpectedVersionCode | Out-Null
    } catch {
        if (-not $AllowAdbVerificationDeferred) { throw }
        $verificationError = $_
        try {
            $adbPath = Resolve-ElonApkAdbPath
            Invoke-ElonAdbCommand -AdbPath $adbPath -Arguments @('kill-server') -TimeoutSeconds 10 | Out-Null
            Write-Host 'ADB_SERVER_CLEANUP=passed'
        } catch {
            Write-Warning "Unable to stop the deferred ADB verification server: $_"
        }
        Write-Warning "Real-device APK verification deferred after successful server publication: $verificationError"
        Write-Host 'APK_ADB_DEPLOY_STATUS=verification_deferred'
        Write-Host 'VERIFICATION_DEFERRED=real_device_unavailable'
        Write-Host 'REAL_DEVICE_STATUS=offline_or_unavailable'
    }
}
