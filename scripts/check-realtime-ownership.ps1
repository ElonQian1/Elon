param(
    [string]$OwnershipPath = "docs\realtime-channel-ownership.md"
)

$ErrorActionPreference = "Stop"

function Stop-RealtimeOwnershipGuard {
    param([string]$Message)
    Write-Error $Message
    exit 1
}

function Get-RepoRoot {
    $root = (& git rev-parse --show-toplevel).Trim()
    if ($LASTEXITCODE -ne 0 -or [string]::IsNullOrWhiteSpace($root)) {
        Stop-RealtimeOwnershipGuard "Current directory is not inside a git repository."
    }
    return $root
}

function Read-TextFile {
    param([string]$Path)
    if (-not (Test-Path -LiteralPath $Path -PathType Leaf)) {
        Stop-RealtimeOwnershipGuard "Required file is missing: $Path"
    }
    return [System.IO.File]::ReadAllText($Path, [System.Text.Encoding]::UTF8)
}

function Assert-Contains {
    param(
        [string]$Label,
        [string]$Text,
        [string]$Needle
    )
    if (-not $Text.Contains($Needle)) {
        Stop-RealtimeOwnershipGuard "$Label is missing required entry: $Needle"
    }
}

function Assert-Source-Contains {
    param(
        [string]$SourcePath,
        [string]$Needle
    )
    $source = Read-TextFile $SourcePath
    Assert-Contains -Label $SourcePath -Text $source -Needle $Needle
}

function Assert-Not-Contains {
    param(
        [string]$Label,
        [string]$Text,
        [string]$Needle
    )
    if ($Text.Contains($Needle)) {
        Stop-RealtimeOwnershipGuard "$Label contains forbidden stale entry: $Needle"
    }
}

$repoRoot = Get-RepoRoot
Set-Location $repoRoot

$ownershipFullPath = Join-Path $repoRoot $OwnershipPath
$ownership = Read-TextFile $ownershipFullPath

$channelLabels = @(
    "app_notify",
    "global_app",
    "homecli_agent",
    "peer_relay",
    "project_ws",
    "voice_realtime_chat",
    "voice_transcribe",
    "voice_virtual_mic"
)

$channelVariants = @(
    "RealtimeChannel::AppNotify",
    "RealtimeChannel::GlobalApp",
    "RealtimeChannel::HomecliAgent",
    "RealtimeChannel::PeerRelay",
    "RealtimeChannel::ProjectWs",
    "RealtimeChannel::VoiceRealtimeChat",
    "RealtimeChannel::VoiceTranscribe",
    "RealtimeChannel::VoiceVirtualMic"
)

$sharedCloseReasons = @(
    "peer_closed",
    "read_error",
    "reader_ended",
    "pong_write_failed",
    "write_failed",
    "client_control_close"
)

$homecliCloseReasons = @(
    "reader_shutdown",
    "reader_closed",
    "reader_timeout",
    "reader_error",
    "writer_closed"
)

$peerRelayCloseReasons = @(
    "request_channel_closed",
    "peer_closed",
    "peer_read_error",
    "peer_reader_ended",
    "peer_write_error"
)
$allCloseReasons = @($sharedCloseReasons + $homecliCloseReasons + $peerRelayCloseReasons | Select-Object -Unique)

$ownerFiles = @(
    "server/src/app_update.rs",
    "server/src/global_ws.rs",
    "server/src/project_ws_session.rs",
    "server/src/voice_ws_transcribe.rs",
    "server/src/voice_ws_transcribe_impl.rs",
    "server/src/voice_ws_realtime_chat.rs",
    "server/src/voice_ws_realtime_chat_impl.rs",
    "server/src/voice_ws_virtual_mic.rs",
    "server/src/homecli_agent/agent_session.rs",
    "server/src/peer_relay.rs",
    "server/src/ws_transport.rs",
    "server/src/ws_client_transport.rs",
    "server/src/realtime_metrics.rs",
    "server/src/realtime_diagnostics_catalog.snapshot.json",
    "server/src/router/admin_routes.rs",
    "server/src/store/realtime_close_events.rs",
    "server/src/assets/admin.html",
    "scripts/test-admin-realtime-health.js",
    "scripts/check-realtime-diagnostics-snapshot.ps1",
    "docs/realtime-operations-runbook.md"
)

foreach ($label in $channelLabels) {
    Assert-Contains -Label "Realtime ownership doc" -Text $ownership -Needle $label
    Assert-Source-Contains -SourcePath (Join-Path $repoRoot "server\src\realtime_metrics.rs") -Needle "`"$label`""
}

foreach ($variant in $channelVariants) {
    Assert-Contains -Label "Realtime ownership doc" -Text $ownership -Needle $variant
    Assert-Source-Contains -SourcePath (Join-Path $repoRoot "server\src\realtime_metrics.rs") -Needle ($variant -replace "RealtimeChannel::", "")
}

foreach ($reason in $allCloseReasons) {
    Assert-Contains -Label "Realtime ownership doc" -Text $ownership -Needle $reason
}

foreach ($reason in $sharedCloseReasons) {
    Assert-Source-Contains -SourcePath (Join-Path $repoRoot "server\src\ws_transport.rs") -Needle "`"$reason`""
}

foreach ($reason in $homecliCloseReasons) {
    Assert-Source-Contains -SourcePath (Join-Path $repoRoot "server\src\homecli_agent\agent_session.rs") -Needle "`"$reason`""
}

foreach ($reason in $peerRelayCloseReasons) {
    Assert-Source-Contains -SourcePath (Join-Path $repoRoot "server\src\peer_relay.rs") -Needle "`"$reason`""
}

foreach ($file in $ownerFiles) {
    Assert-Contains -Label "Realtime ownership doc" -Text $ownership -Needle $file
    if (-not (Test-Path -LiteralPath (Join-Path $repoRoot $file) -PathType Leaf)) {
        Stop-RealtimeOwnershipGuard "Realtime ownership doc references missing file: $file"
    }
}

$recordingFiles = @(
    "server\src\app_update.rs",
    "server\src\global_ws.rs",
    "server\src\project_ws_session.rs",
    "server\src\voice_ws_transcribe_impl.rs",
    "server\src\voice_ws_realtime_chat_impl.rs",
    "server\src\voice_ws_virtual_mic.rs",
    "server\src\homecli_agent\agent_session.rs",
    "server\src\peer_relay.rs"
)

foreach ($file in $recordingFiles) {
    Assert-Source-Contains -SourcePath (Join-Path $repoRoot $file) -Needle "record_close_with_store"
}

Assert-Contains -Label "Realtime ownership doc" -Text $ownership -Needle "Change Rules"
Assert-Contains -Label "Realtime ownership doc" -Text $ownership -Needle "channel"
Assert-Contains -Label "Realtime ownership doc" -Text $ownership -Needle "close reason"
Assert-Contains -Label "Realtime ownership doc" -Text $ownership -Needle "WS"
Assert-Contains -Label "Realtime ownership doc" -Text $ownership -Needle "/api/admin/realtime/diagnostics"
Assert-Contains -Label "Realtime ownership doc" -Text $ownership -Needle "realtime_diagnostics_catalog()"

$metricsSource = Read-TextFile (Join-Path $repoRoot "server\src\realtime_metrics.rs")
Assert-Contains -Label "realtime_metrics.rs" -Text $metricsSource -Needle "pub fn realtime_diagnostics_catalog"
Assert-Contains -Label "realtime_metrics.rs" -Text $metricsSource -Needle "pub async fn admin_diagnostics"
Assert-Contains -Label "realtime_metrics.rs" -Text $metricsSource -Needle "alert_bucket"
Assert-Contains -Label "realtime_metrics.rs" -Text $metricsSource -Needle "first_check"
Assert-Contains -Label "realtime_metrics.rs" -Text $metricsSource -Needle "RealtimeDiagnosticsCatalog"
Assert-Contains -Label "realtime_metrics.rs" -Text $metricsSource -Needle "realtime_diagnostics_catalog_matches_snapshot"
Assert-Contains -Label "realtime_metrics.rs" -Text $metricsSource -Needle "realtime_diagnostics_catalog.snapshot.json"

$diagnosticsSnapshotSource = Read-TextFile (Join-Path $repoRoot "server\src\realtime_diagnostics_catalog.snapshot.json")
Assert-Contains -Label "realtime_diagnostics_catalog.snapshot.json" -Text $diagnosticsSnapshotSource -Needle '"channels"'
Assert-Contains -Label "realtime_diagnostics_catalog.snapshot.json" -Text $diagnosticsSnapshotSource -Needle '"close_reasons"'
Assert-Contains -Label "realtime_diagnostics_catalog.snapshot.json" -Text $diagnosticsSnapshotSource -Needle '"change_rules"'
Assert-Contains -Label "realtime_diagnostics_catalog.snapshot.json" -Text $diagnosticsSnapshotSource -Needle '"alert_bucket"'
Assert-Contains -Label "realtime_diagnostics_catalog.snapshot.json" -Text $diagnosticsSnapshotSource -Needle '"first_check"'

$adminRoutesSource = Read-TextFile (Join-Path $repoRoot "server\src\router\admin_routes.rs")
Assert-Contains -Label "admin_routes.rs" -Text $adminRoutesSource -Needle "/api/admin/realtime/diagnostics"
Assert-Contains -Label "admin_routes.rs" -Text $adminRoutesSource -Needle "realtime_metrics::admin_diagnostics"

$adminHtmlSource = Read-TextFile (Join-Path $repoRoot "server\src\assets\admin.html")
Assert-Contains -Label "admin.html" -Text $adminHtmlSource -Needle "/api/admin/realtime/diagnostics"
Assert-Contains -Label "admin.html" -Text $adminHtmlSource -Needle "realtimeDiagnosticsMaps"
Assert-Contains -Label "admin.html" -Text $adminHtmlSource -Needle "alert_bucket"
Assert-Contains -Label "admin.html" -Text $adminHtmlSource -Needle "first_check"

$adminSmokeSource = Read-TextFile (Join-Path $repoRoot "scripts\test-admin-realtime-health.js")
Assert-Contains -Label "test-admin-realtime-health.js" -Text $adminSmokeSource -Needle "/api/admin/realtime/diagnostics"
Assert-Contains -Label "test-admin-realtime-health.js" -Text $adminSmokeSource -Needle "first_check"

$realtimeStoreSource = Read-TextFile (Join-Path $repoRoot "server\src\store\realtime_close_events.rs")
Assert-Contains -Label "realtime_close_events.rs" -Text $realtimeStoreSource -Needle "realtime_diagnostics_catalog"
Assert-Contains -Label "realtime_close_events.rs" -Text $realtimeStoreSource -Needle "realtime_alert_detail"
Assert-Contains -Label "realtime_close_events.rs" -Text $realtimeStoreSource -Needle "realtime_first_check_for_bucket"
Assert-Contains -Label "realtime_close_events.rs" -Text $realtimeStoreSource -Needle "count_realtime_close_bucket_since"
Assert-Contains -Label "realtime_close_events.rs" -Text $realtimeStoreSource -Needle "realtime_close_reasons_for_bucket"
Assert-Not-Contains -Label "realtime_close_events.rs" -Text $realtimeStoreSource -Needle "count_realtime_close_read_errors_since"
Assert-Not-Contains -Label "realtime_close_events.rs" -Text $realtimeStoreSource -Needle "count_realtime_close_write_failures_since"
Assert-Not-Contains -Label "realtime_close_events.rs" -Text $realtimeStoreSource -Needle "count_realtime_close_timeouts_since"

$realtimeStoreTestsSource = Read-TextFile (Join-Path $repoRoot "server\src\store\realtime_close_events_tests.rs")
Assert-Contains -Label "realtime_close_events_tests.rs" -Text $realtimeStoreTestsSource -Needle "realtime_close_alert_details_include_diagnostics_first_check"
Assert-Contains -Label "realtime_close_events_tests.rs" -Text $realtimeStoreTestsSource -Needle "realtime_close_alert_counts_follow_diagnostics_buckets"

Write-Host "REALTIME_OWNERSHIP_GUARD=passed channels=$($channelLabels.Count) reasons=$($allCloseReasons.Count) files=$($ownerFiles.Count)"
