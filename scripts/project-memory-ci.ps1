[CmdletBinding()]
param(
    [string]$ProjectRoot = "",
    [string]$NodeAdminUrl = $env:ELON_NODE_ADMIN_URL,
    [ValidateSet("advisory", "fail_on_drift", "strict")]
    [string]$FailurePolicy = "strict",
    [ValidateRange(1, 200)]
    [int]$Limit = 200,
    [switch]$IncludeCapabilities,
    [switch]$JsonOnly
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
            $health = Invoke-RestMethod -Uri "$candidate/api/health" -Method Get -TimeoutSec 1
            if ($null -ne $health) { return $candidate }
        } catch {
            continue
        }
    }
    throw "No local Yilong node admin API was found on ports 7799-7819. Start the node or pass -NodeAdminUrl."
}

if ([string]::IsNullOrWhiteSpace($ProjectRoot)) {
    $ProjectRoot = (& git rev-parse --show-toplevel).Trim()
    if ($LASTEXITCODE -ne 0 -or [string]::IsNullOrWhiteSpace($ProjectRoot)) {
        throw "Current directory is not inside a Git repository; pass -ProjectRoot explicitly."
    }
}
$ProjectRoot = [System.IO.Path]::GetFullPath($ProjectRoot).TrimEnd("\", "/")
$adminUrl = Resolve-NodeAdminUrl $NodeAdminUrl
$body = @{
    project_root = $ProjectRoot
    offset = 0
    limit = $Limit
    failure_policy = $FailurePolicy
    include_capabilities = [bool]$IncludeCapabilities
} | ConvertTo-Json -Depth 6

$envelope = Invoke-RestMethod `
    -Uri "$adminUrl/api/project-docs/native-context/health" `
    -Method Post `
    -ContentType "application/json; charset=utf-8" `
    -Body $body `
    -TimeoutSec 30
if (-not $envelope.ok -or $null -eq $envelope.result) {
    throw "Project memory health check failed: $($envelope.error)"
}

$result = $envelope.result
$exitCode = [int]$result.policy_outcome.recommended_exit_code
if ($JsonOnly) {
    $result | ConvertTo-Json -Depth 16 -Compress
} else {
    Write-Host "PROJECT_MEMORY_CI_SCHEMA=$($result.schema)"
    Write-Host "PROJECT_MEMORY_CI_POLICY=$FailurePolicy"
    Write-Host "PROJECT_MEMORY_CI_STATUS=$($result.policy_outcome.status)"
    Write-Host "PROJECT_MEMORY_CI_CHECKED=$($result.checked_count)"
    Write-Host "PROJECT_MEMORY_CI_CURRENT=$($result.current_count)"
    Write-Host "PROJECT_MEMORY_CI_ISSUES=$($result.issue_count)"
    Write-Host "PROJECT_MEMORY_CI_EXIT_CODE=$exitCode"
    if (@($result.policy_outcome.reasons).Count -gt 0) {
        Write-Host "PROJECT_MEMORY_CI_REASONS=$(@($result.policy_outcome.reasons) -join ',')"
    }
    $result | ConvertTo-Json -Depth 16
}
exit $exitCode
