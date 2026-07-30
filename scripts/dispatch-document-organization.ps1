[CmdletBinding()]
param(
    [string]$NodeAdminUrl = $env:ELON_NODE_ADMIN_URL,
    [string]$SignalPath = "",
    [switch]$KeepSignal
)

$ErrorActionPreference = "Stop"

function Resolve-NodeAdminUrl {
    param([string]$RequestedUrl)
    if (-not [string]::IsNullOrWhiteSpace($RequestedUrl)) {
        return $RequestedUrl.TrimEnd("/")
    }
    foreach ($port in 7799..7819) {
        $candidate = "http://127.0.0.1:$port"
        try {
            $health = Invoke-RestMethod -Uri "$candidate/health" -Method Get -TimeoutSec 1
            if ($null -ne $health) { return $candidate }
        } catch {
            continue
        }
    }
    throw "No local Yilong node admin API was found on ports 7799-7819."
}

$repoRoot = (& git rev-parse --show-toplevel).Trim()
if ($LASTEXITCODE -ne 0 -or [string]::IsNullOrWhiteSpace($repoRoot)) {
    throw "Current directory is not inside a git repository."
}
Set-Location $repoRoot

if ([string]::IsNullOrWhiteSpace($SignalPath)) {
    $gitSignalPath = (& git rev-parse --git-path "elon/document-organization-trigger.json").Trim()
    if ([System.IO.Path]::IsPathRooted($gitSignalPath)) {
        $SignalPath = [System.IO.Path]::GetFullPath($gitSignalPath)
    } else {
        $SignalPath = [System.IO.Path]::GetFullPath((Join-Path $repoRoot $gitSignalPath))
    }
}
if (-not (Test-Path -LiteralPath $SignalPath -PathType Leaf)) {
    Write-Host "DOCUMENT_AUTOMATION_DISPATCH=not_required"
    exit 0
}

$signal = [System.IO.File]::ReadAllText($SignalPath, [System.Text.Encoding]::UTF8) | ConvertFrom-Json
if ([int]$signal.version -ne 1) {
    throw "Unsupported document organization signal version '$($signal.version)'."
}
$expectedRoot = [System.IO.Path]::GetFullPath($repoRoot).TrimEnd("\", "/")
$signalRoot = [System.IO.Path]::GetFullPath([string]$signal.workspace_path).TrimEnd("\", "/")
if (-not $expectedRoot.Equals($signalRoot, [System.StringComparison]::OrdinalIgnoreCase)) {
    throw "Document organization signal belongs to a different workspace."
}

$commitSha = [string]$signal.commit_sha
if ([string]::IsNullOrWhiteSpace($commitSha)) {
    $commitSha = (& git rev-parse HEAD).Trim()
    $signal | Add-Member -NotePropertyName commit_sha -NotePropertyValue $commitSha -Force
    $temporarySignal = "$SignalPath.$([guid]::NewGuid().ToString('N')).tmp"
    [System.IO.File]::WriteAllText(
        $temporarySignal,
        ($signal | ConvertTo-Json -Depth 8),
        [System.Text.UTF8Encoding]::new($false)
    )
    Move-Item -LiteralPath $temporarySignal -Destination $SignalPath -Force
}
$adminUrl = Resolve-NodeAdminUrl $NodeAdminUrl
$body = @{
    project_root = $expectedRoot
    commit_sha = $commitSha
    severity = [string]$signal.severity
    paths = @($signal.paths)
    reasons = @($signal.reasons)
} | ConvertTo-Json -Depth 8
$response = Invoke-RestMethod `
    -Uri "$adminUrl/api/project-docs/organization/automatic-trigger" `
    -Method Post `
    -ContentType "application/json; charset=utf-8" `
    -Body $body `
    -TimeoutSec 10
if (-not $response.ok -or $null -eq $response.trigger) {
    throw "The local node did not acknowledge the document organization trigger."
}
if (-not $KeepSignal) {
    Remove-Item -LiteralPath $SignalPath -Force
}
Write-Host "DOCUMENT_AUTOMATION_DISPATCH=queued"
Write-Host "DOCUMENT_AUTOMATION_TRIGGER_ID=$($response.trigger.trigger_id)"
Write-Host "DOCUMENT_AUTOMATION_OPERATION_ID=$($response.trigger.operation_id)"
