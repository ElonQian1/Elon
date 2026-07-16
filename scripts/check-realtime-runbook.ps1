param(
    [string]$RunbookPath = "docs\realtime-operations-runbook.md"
)

$ErrorActionPreference = "Stop"

function Stop-RealtimeRunbookGuard {
    param([string]$Message)
    Write-Error $Message
    exit 1
}

function Get-RepoRoot {
    $root = (& git rev-parse --show-toplevel).Trim()
    if ($LASTEXITCODE -ne 0 -or [string]::IsNullOrWhiteSpace($root)) {
        Stop-RealtimeRunbookGuard "Current directory is not inside a git repository."
    }
    return $root
}

function Read-TextFile {
    param([string]$Path)
    if (-not (Test-Path -LiteralPath $Path -PathType Leaf)) {
        Stop-RealtimeRunbookGuard "Required file is missing: $Path"
    }
    return [System.IO.File]::ReadAllText($Path, [System.Text.Encoding]::UTF8)
}

function Assert-Contains {
    param(
        [string]$Text,
        [string]$Needle
    )
    if (-not $Text.Contains($Needle)) {
        Stop-RealtimeRunbookGuard "Realtime runbook is missing required entry: $Needle"
    }
}

$repoRoot = Get-RepoRoot
Set-Location $repoRoot

$runbookFullPath = Join-Path $repoRoot $RunbookPath
$runbook = Read-TextFile $runbookFullPath

$requiredEntries = @(
    "/api/admin/realtime/close-metrics",
    "/api/admin/realtime/diagnostics",
    "windows.last_1h",
    "windows.last_24h",
    "windows.all_time",
    "windows.process",
    "metrics",
    "alerts",
    "channels",
    "close_reasons",
    "alert_bucket",
    "realtime_close_read_error_alert_threshold_1h",
    "realtime_close_write_failure_alert_threshold_1h",
    "realtime_close_timeout_alert_threshold_1h",
    "app_notify",
    "global_app",
    "project_ws",
    "voice_transcribe",
    "voice_realtime_chat",
    "voice_virtual_mic",
    "homecli_agent",
    "peer_relay",
    "peer_closed",
    "client_control_close",
    "reader_ended",
    "peer_reader_ended",
    "read_error",
    "peer_read_error",
    "write_failed",
    "peer_write_error",
    "pong_write_failed",
    "reader_timeout",
    "writer_closed",
    "ws_transport.rs",
    "ws_client_transport.rs",
    "realtime_metrics.rs",
    "store/realtime_close_events.rs",
    "cargo test --manifest-path server\Cargo.toml ws_transport --quiet",
    "cargo test --manifest-path server\Cargo.toml realtime_metrics --quiet",
    "cargo test --manifest-path server\Cargo.toml realtime_close_events --quiet"
)

foreach ($entry in $requiredEntries) {
    Assert-Contains -Text $runbook -Needle $entry
}

Write-Host "REALTIME_RUNBOOK_GUARD=passed entries=$($requiredEntries.Count)"
