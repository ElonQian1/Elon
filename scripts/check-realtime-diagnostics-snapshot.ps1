param(
    [string]$MetricsPath = "server\src\realtime_metrics.rs",
    [string]$CatalogPath = "server\src\realtime_metrics\catalog.rs",
    [string]$TestsPath = "server\src\realtime_metrics_tests.rs",
    [string]$SnapshotPath = "server\src\realtime_diagnostics_catalog.snapshot.json",
    [int]$CargoLockTimeoutSeconds = 120,
    [switch]$SkipCargoTest
)

$ErrorActionPreference = "Stop"

function Stop-RealtimeDiagnosticsSnapshotGuard {
    param([string]$Message)
    Write-Error $Message
    exit 1
}

function Get-RepoRoot {
    $root = (& git rev-parse --show-toplevel).Trim()
    if ($LASTEXITCODE -ne 0 -or [string]::IsNullOrWhiteSpace($root)) {
        Stop-RealtimeDiagnosticsSnapshotGuard "Current directory is not inside a git repository."
    }
    return $root
}

function Read-TextFile {
    param([string]$Path)
    if (-not (Test-Path -LiteralPath $Path -PathType Leaf)) {
        Stop-RealtimeDiagnosticsSnapshotGuard "Required file is missing: $Path"
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
        Stop-RealtimeDiagnosticsSnapshotGuard "$Label is missing required entry: $Needle"
    }
}

function Assert-HasJsonProperties {
    param(
        [string]$Label,
        [object]$Value,
        [string[]]$Properties
    )

    $actual = @($Value.PSObject.Properties.Name)
    foreach ($property in $Properties) {
        if ($actual -notcontains $property) {
            Stop-RealtimeDiagnosticsSnapshotGuard "$Label is missing JSON property: $property"
        }
    }
}

$repoRoot = Get-RepoRoot
Set-Location $repoRoot

$metricsFullPath = Join-Path $repoRoot $MetricsPath
$catalogFullPath = Join-Path $repoRoot $CatalogPath
$testsFullPath = Join-Path $repoRoot $TestsPath
$snapshotFullPath = Join-Path $repoRoot $SnapshotPath

$metricsSource = Read-TextFile $metricsFullPath
$catalogSource = Read-TextFile $catalogFullPath
$testsSource = Read-TextFile $testsFullPath
$snapshotSource = Read-TextFile $snapshotFullPath

Assert-Contains -Label "realtime_metrics.rs" -Text $metricsSource -Needle "mod catalog"
Assert-Contains -Label "realtime_metrics.rs" -Text $metricsSource -Needle "pub use catalog::realtime_diagnostics_catalog"
Assert-Contains -Label "realtime_metrics/catalog.rs" -Text $catalogSource -Needle "pub fn realtime_diagnostics_catalog"
Assert-Contains -Label "realtime_metrics/catalog.rs" -Text $catalogSource -Needle "Diagnostics catalog changes must update server/src/realtime_diagnostics_catalog.snapshot.json and the snapshot test."
Assert-Contains -Label "realtime_metrics_tests.rs" -Text $testsSource -Needle "realtime_diagnostics_catalog_matches_snapshot"
Assert-Contains -Label "realtime_metrics_tests.rs" -Text $testsSource -Needle 'include_str!("realtime_diagnostics_catalog.snapshot.json")'

try {
    $snapshot = $snapshotSource | ConvertFrom-Json
} catch {
    Stop-RealtimeDiagnosticsSnapshotGuard "Diagnostics snapshot is not valid JSON: $($_.Exception.Message)"
}

Assert-HasJsonProperties -Label "diagnostics snapshot root" -Value $snapshot -Properties @(
    "version",
    "channels",
    "close_reasons",
    "change_rules"
)

$channels = @($snapshot.channels)
$closeReasons = @($snapshot.close_reasons)
$changeRules = @($snapshot.change_rules)

if ($channels.Count -eq 0) {
    Stop-RealtimeDiagnosticsSnapshotGuard "Diagnostics snapshot has no channels."
}
if ($closeReasons.Count -eq 0) {
    Stop-RealtimeDiagnosticsSnapshotGuard "Diagnostics snapshot has no close reasons."
}
if ($changeRules.Count -eq 0) {
    Stop-RealtimeDiagnosticsSnapshotGuard "Diagnostics snapshot has no change rules."
}

foreach ($channel in $channels) {
    Assert-HasJsonProperties -Label "diagnostics snapshot channel" -Value $channel -Properties @(
        "id",
        "business_boundary",
        "entry_modules",
        "close_reason_source",
        "metric_variant",
        "sync_targets"
    )
}

foreach ($reason in $closeReasons) {
    Assert-HasJsonProperties -Label "diagnostics snapshot close reason" -Value $reason -Properties @(
        "id",
        "source",
        "category",
        "alert_bucket",
        "meaning",
        "first_check"
    )
}

if ($changeRules -notcontains "Diagnostics catalog changes must update server/src/realtime_diagnostics_catalog.snapshot.json and the snapshot test.") {
    Stop-RealtimeDiagnosticsSnapshotGuard "Diagnostics snapshot change rules do not mention snapshot updates."
}

if (-not $SkipCargoTest) {
    & powershell -NoProfile -ExecutionPolicy Bypass -File (Join-Path $repoRoot "scripts\cargo-dev.ps1") -LockTimeoutSeconds $CargoLockTimeoutSeconds test --manifest-path server\Cargo.toml realtime_diagnostics_catalog_matches_snapshot --quiet
    if ($LASTEXITCODE -ne 0) {
        Stop-RealtimeDiagnosticsSnapshotGuard "realtime_diagnostics_catalog_matches_snapshot failed."
    }
}

Write-Host "REALTIME_DIAGNOSTICS_SNAPSHOT_GUARD=passed channels=$($channels.Count) reasons=$($closeReasons.Count) rules=$($changeRules.Count)"
