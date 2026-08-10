#requires -Version 5.1

[CmdletBinding()]
param(
    [string]$Adb = "D:\Android\sdk\platform-tools\adb.exe",
    [Parameter(Mandatory = $true)][string]$DeviceSerial,
    [ValidateRange(10, 300)][int]$ReadyTimeoutSec = 90,
    [ValidateRange(1, 10)][int]$PollIntervalSec = 2,
    [ValidateRange(1, 12)][int]$MaxFeaturePages = 8
)

$ErrorActionPreference = "Stop"
. (Join-Path $PSScriptRoot "chatgpt-web-smoke-runtime.ps1")
. (Join-Path $PSScriptRoot "chatgpt-web-feature-audit-policy.ps1")

$runtime = New-ChatGptWebSmokeRuntime -Adb $Adb -DeviceSerial $DeviceSerial `
    -PollIntervalSec $PollIntervalSec
Assert-ChatGptWebSmokeUsbDevice -Runtime $runtime

$safeKinds = @("library", "tasks", "apps", "projects", "gpts")
$results = [System.Collections.Generic.List[object]]::new()

function Wait-FeatureList {
    Invoke-ChatGptWebSmokeAction -Runtime $runtime -Action "chatgpt_list_features" | Out-Null
    return Wait-ChatGptWebSmokeState -Runtime $runtime -TimeoutSec $ReadyTimeoutSec `
        -Description "ChatGPT feature navigation" -Predicate {
            param($state)
            $state.bridge_state -eq "ready" -and @($state.features).Count -gt 0
        }
}

function Wait-CommandAndPage {
    param(
        [Parameter(Mandatory = $true)][string]$RequestId,
        [Parameter(Mandatory = $true)][string]$PageKind,
        [Parameter(Mandatory = $true)][string]$Description
    )

    return Wait-ChatGptWebSmokeState -Runtime $runtime -TimeoutSec $ReadyTimeoutSec `
        -Description $Description -Predicate {
            param($state)
            $receipt = @($state.command_requests) |
                Where-Object { [string]$_.request_id -eq $RequestId } |
                Select-Object -Last 1
            $receipt.status -eq "succeeded" -and
                $receipt.result.ok -eq $true -and
                [string]$state.page_kind -eq $PageKind -and
                $state.bridge_state -eq "ready"
        }
}

function Return-Home {
    $response = Invoke-ChatGptWebSmokeAction -Runtime $runtime -Action "chatgpt_new_conversation"
    $requestId = [string]$response.command_receipt.request_id
    if (-not $requestId) { throw "New conversation command returned no request receipt." }
    Wait-CommandAndPage -RequestId $requestId -PageKind "home" `
        -Description "ChatGPT empty conversation home" | Out-Null
}

Invoke-ChatGptWebSmokeAction -Runtime $runtime -Action "open_chatgpt_web" `
    -EnsureMainActivity | Out-Null
Invoke-ChatGptWebSmokeAction -Runtime $runtime -Action "chatgpt_select_view" `
    -Arguments @{ view_mode = "official" } | Out-Null
Wait-ChatGptWebSmokeState -Runtime $runtime -TimeoutSec $ReadyTimeoutSec `
    -Description "authenticated ChatGPT Web" -Predicate {
        param($state)
        $state.surface -eq "chatgpt_web" -and
            $state.bridge_state -eq "ready" -and
            $state.authenticated -eq $true
    } | Out-Null

Return-Home
$initial = Wait-FeatureList
$availableKinds = @(
    $initial.features |
        Where-Object { [string]$_.kind -in $safeKinds } |
        ForEach-Object { [string]$_.kind } |
        Sort-Object -Unique |
        Select-Object -First $MaxFeaturePages
)
if ($availableKinds.Count -eq 0) {
    throw "No safe ChatGPT feature pages are visible for structural audit."
}

foreach ($kind in $availableKinds) {
    $navigation = Wait-FeatureList
    $feature = @($navigation.features) |
        Where-Object { [string]$_.kind -eq $kind -and $_.selected -ne $true } |
        Select-Object -First 1
    if ($null -eq $feature) {
        $results.Add([pscustomobject]@{
            kind = $kind
            passed = $false
            reasons = @("feature_not_selectable")
        })
        continue
    }

    $selected = Invoke-ChatGptWebSmokeAction -Runtime $runtime -Action "chatgpt_select_feature" `
        -Arguments @{ feature_id = [string]$feature.id }
    $requestId = [string]$selected.command_receipt.request_id
    if (-not $requestId) { throw "Feature selection returned no request receipt for kind=$kind." }
    Wait-CommandAndPage -RequestId $requestId -PageKind "feature" `
        -Description "ChatGPT feature page kind=$kind" | Out-Null

    $matrix = Invoke-ChatGptWebSmokeAction -Runtime $runtime `
        -Action "chatgpt_get_capability_matrix"
    $audit = Test-ChatGptWebFeatureMatrix -Matrix $matrix
    $results.Add([pscustomobject]@{
        kind = $kind
        passed = [bool]$audit.passed
        reasons = @($audit.reasons)
        control_count = [int]$audit.control_count
        native_control_count = [int]$audit.native_control_count
        generic_control_count = [int]$audit.generic_control_count
        unexpected_fallback_count = [int]$audit.unexpected_fallback_count
    })
    Write-Output "CHATGPT_FEATURE_PAGE kind=$kind passed=$($audit.passed) controls=$($audit.control_count) generic=$($audit.generic_control_count) unexpected_fallback=$($audit.unexpected_fallback_count)"
    Return-Home
}

$failed = @($results | Where-Object { $_.passed -ne $true })
[ordered]@{
    schema = "elon.chatgpt_web.feature_page_smoke.v1"
    passed = $failed.Count -eq 0
    device_serial = $DeviceSerial
    audited_kinds = @($availableKinds)
    results = @($results)
} | ConvertTo-Json -Depth 12

if ($failed.Count -gt 0) {
    Write-Output "CHATGPT_FEATURE_PAGE_SMOKE_STATUS=failed failed_count=$($failed.Count)"
    exit 1
}
Write-Output "CHATGPT_FEATURE_PAGE_SMOKE_STATUS=passed audited_count=$($results.Count)"
