#requires -Version 5.1

[CmdletBinding()]
param(
    [string]$Adb = "D:\Android\sdk\platform-tools\adb.exe",
    [Parameter(Mandatory = $true)][string]$DeviceSerial,
    [string]$ExpectedHardwareSerial = "",
    [ValidateRange(10, 300)][int]$ReadyTimeoutSec = 90,
    [ValidateRange(30, 600)][int]$TotalTimeoutSec = 180,
    [ValidateRange(1, 10)][int]$PollIntervalSec = 2,
    [ValidateRange(1, 12)][int]$MaxFeaturePages = 8,
    [ValidateRange(1, 9999)][int]$ExpectedAdapterVersion = 79
)

$ErrorActionPreference = "Stop"
. (Join-Path $PSScriptRoot "chatgpt-web-smoke-runtime.ps1")
. (Join-Path $PSScriptRoot "chatgpt-web-feature-audit-policy.ps1")

$runtime = New-ChatGptWebSmokeRuntime -Adb $Adb -DeviceSerial $DeviceSerial `
    -ExpectedHardwareSerial $ExpectedHardwareSerial -PollIntervalSec $PollIntervalSec
Assert-ChatGptWebSmokeTrustedDevice -Runtime $runtime
$scriptDeadline = [DateTimeOffset]::UtcNow.AddSeconds($TotalTimeoutSec)

$safeKinds = @("library", "tasks", "apps", "projects", "gpts")
$results = [System.Collections.Generic.List[object]]::new()

function Get-RemainingSeconds {
    param(
        [Parameter(Mandatory = $true)][DateTimeOffset]$Deadline,
        [ValidateRange(1, 60)][int]$Minimum = 1,
        [ValidateRange(1, 300)][int]$Maximum = 300
    )

    $remaining = [int][Math]::Ceiling(($Deadline - [DateTimeOffset]::UtcNow).TotalSeconds)
    if ($remaining -lt $Minimum) { return 0 }
    return [Math]::Min($remaining, $Maximum)
}

function Get-StepDeadline {
    param([ValidateRange(1, 300)][int]$TimeoutSec)

    $stepDeadline = [DateTimeOffset]::UtcNow.AddSeconds($TimeoutSec)
    if ($stepDeadline -lt $scriptDeadline) { return $stepDeadline }
    return $scriptDeadline
}

function Wait-FeatureList {
    $deadline = Get-StepDeadline -TimeoutSec $ReadyTimeoutSec
    do {
        $remaining = Get-RemainingSeconds -Deadline $deadline -Minimum 10 -Maximum 30
        if ($remaining -eq 0) { break }
        Wait-ChatGptWebSmokeAuthenticatedReady -Runtime $runtime `
            -TimeoutSec $remaining -InitialWaitSec ([Math]::Min(5, $remaining)) | Out-Null
        Invoke-ChatGptWebSmokeAction -Runtime $runtime -Action "chatgpt_list_features" | Out-Null
        $navigation = Invoke-ChatGptWebSmokeAction -Runtime $runtime -Action "chatgpt_get_navigation"
        $features = @($navigation.features | Where-Object { $null -ne $_ })
        if (
            $navigation.control_ok -eq $true -and
            $features.Count -gt 0
        ) {
            return $navigation
        }
        Start-Sleep -Seconds $runtime.poll_interval_sec
    } while ([DateTimeOffset]::UtcNow -lt $deadline)
    throw "Timed out waiting for ChatGPT feature navigation."
}

function Wait-CommandAndPage {
    param(
        [Parameter(Mandatory = $true)][string]$RequestId,
        [Parameter(Mandatory = $true)][string]$PageKind,
        [Parameter(Mandatory = $true)][string]$Description
    )

    $deadline = Get-StepDeadline -TimeoutSec $ReadyTimeoutSec
    $remaining = Get-RemainingSeconds -Deadline $deadline -Minimum 10 `
        -Maximum $ReadyTimeoutSec
    if ($remaining -eq 0) { throw "Timed out waiting for $Description before bridge recovery." }
    Wait-ChatGptWebSmokeAuthenticatedReady -Runtime $runtime `
        -TimeoutSec $remaining -InitialWaitSec ([Math]::Min(5, $remaining)) | Out-Null
    $remaining = Get-RemainingSeconds -Deadline $deadline
    if ($remaining -eq 0) { throw "Timed out waiting for $Description after bridge recovery." }
    return Wait-ChatGptWebSmokeState -Runtime $runtime -TimeoutSec $remaining `
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

function Get-ObservedPath {
    param($State)

    $uri = $null
    if ([Uri]::TryCreate([string]$State.conversation.url, [UriKind]::Absolute, [ref]$uri)) {
        return $uri.AbsolutePath
    }
    return ""
}

function Wait-FeatureMatrix {
    param([Parameter(Mandatory = $true)][string]$Kind)

    $deadline = Get-StepDeadline -TimeoutSec $ReadyTimeoutSec
    $last = $null
    do {
        $last = Invoke-ChatGptWebSmokeAction -Runtime $runtime `
            -Action "chatgpt_get_capability_matrix"
        if (
            $last.control_ok -eq $true -and
            $last.ready_for_mcp -eq $true -and
            [string]$last.manifest.page_kind -eq "feature" -and
            [string]$last.manifest.compatibility -eq "healthy"
        ) {
            return $last
        }
        Start-Sleep -Seconds $runtime.poll_interval_sec
    } while ([DateTimeOffset]::UtcNow -lt $deadline)
    throw "Timed out waiting for current ChatGPT feature manifest: kind=$Kind compatibility=$($last.manifest.compatibility)."
}

function Restore-Origin {
    param(
        [Parameter(Mandatory = $true)][string]$PageKind,
        [string]$Path
    )

    $deadline = Get-StepDeadline -TimeoutSec ([Math]::Min(45, $ReadyTimeoutSec))
    $nextBackAt = [DateTimeOffset]::MinValue
    $backAttempts = 0
    $last = $null
    do {
        $remaining = Get-RemainingSeconds -Deadline $deadline -Minimum 10 -Maximum 30
        if ($remaining -eq 0) { break }
        $last = Wait-ChatGptWebSmokeAuthenticatedReady -Runtime $runtime `
            -TimeoutSec $remaining -InitialWaitSec ([Math]::Min(5, $remaining))
        $currentPath = Get-ObservedPath -State $last
        $pathMatches = -not $Path -or $currentPath -eq $Path
        if (
            $last.bridge_state -eq "ready" -and
            [string]$last.page_kind -eq $PageKind -and
            $pathMatches
        ) {
            return
        }
        if ([DateTimeOffset]::UtcNow -ge $nextBackAt -and $backAttempts -lt 3) {
            Invoke-ChatGptWebSmokeAdb -Runtime $runtime -Arguments @("shell", "input", "keyevent", "4") `
                -TimeoutSec 10 -Label "restore ChatGPT origin" | Out-Null
            $backAttempts += 1
            $nextBackAt = [DateTimeOffset]::UtcNow.AddSeconds(5)
        }
        Start-Sleep -Seconds $runtime.poll_interval_sec
    } while ([DateTimeOffset]::UtcNow -lt $deadline)
    throw "Timed out restoring the original ChatGPT page. Last page=$($last.page_kind)."
}

Write-Output "CHATGPT_FEATURE_PAGE_PHASE phase=bootstrap"
Open-ChatGptWebSmokeSurface -Runtime $runtime | Out-Null
$initialReadySec = Get-RemainingSeconds -Deadline $scriptDeadline -Minimum 10 `
    -Maximum $ReadyTimeoutSec
if ($initialReadySec -eq 0) { throw "Feature-page smoke exhausted its total budget during bootstrap." }
$origin = Wait-ChatGptWebSmokeAuthenticatedReady -Runtime $runtime `
    -TimeoutSec $initialReadySec -InitialWaitSec ([Math]::Min(5, $initialReadySec))
if ([string]$origin.view_mode -notin @("official", "web")) {
    Invoke-ChatGptWebSmokeAction -Runtime $runtime -Action "chatgpt_select_view" `
        -Arguments @{ view_mode = "official" } | Out-Null
    $officialReadySec = Get-RemainingSeconds -Deadline $scriptDeadline -Minimum 10 `
        -Maximum $ReadyTimeoutSec
    if ($officialReadySec -eq 0) {
        throw "Feature-page smoke exhausted its total budget selecting official view."
    }
    $origin = Wait-ChatGptWebSmokeAuthenticatedReady -Runtime $runtime `
        -TimeoutSec $officialReadySec -InitialWaitSec ([Math]::Min(5, $officialReadySec))
}
Assert-ChatGptWebSmokeAdapterVersion -State $origin `
    -ExpectedAdapterVersion $ExpectedAdapterVersion
$originPageKind = [string]$origin.page_kind
$originPath = Get-ObservedPath -State $origin

$initial = Wait-FeatureList
$availableKinds = @(
    @($initial.features | Where-Object { $null -ne $_ }) |
        Where-Object { [string]$_.kind -in $safeKinds } |
        ForEach-Object { [string]$_.kind } |
        Sort-Object -Unique |
        Select-Object -First $MaxFeaturePages
)
if ($availableKinds.Count -eq 0) {
    throw "No safe ChatGPT feature pages are visible for structural audit."
}

foreach ($kind in $availableKinds) {
    if ((Get-RemainingSeconds -Deadline $scriptDeadline) -eq 0) {
        throw "Feature-page smoke exhausted its total budget before kind=$kind."
    }
    Write-Output "CHATGPT_FEATURE_PAGE_START kind=$kind"
    $navigation = Wait-FeatureList
    $feature = @($navigation.features | Where-Object { $null -ne $_ }) |
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

    $matrix = Wait-FeatureMatrix -Kind $kind
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
    Write-Output "CHATGPT_FEATURE_PAGE_PHASE phase=restore kind=$kind"
    Restore-Origin -PageKind $originPageKind -Path $originPath
}

$failed = @($results | Where-Object { $_.passed -ne $true })
[ordered]@{
    schema = "elon.chatgpt_web.feature_page_smoke.v1"
    passed = $failed.Count -eq 0
    device_serial = $DeviceSerial
    audited_kinds = @($availableKinds)
    origin_restored = $true
    results = @($results)
} | ConvertTo-Json -Depth 12

if ($failed.Count -gt 0) {
    $failedKinds = @($failed | ForEach-Object { [string]$_.kind }) -join ","
    Write-Output "CHATGPT_FEATURE_PAGE_SMOKE_STATUS=failed failed_count=$($failed.Count) failed_kinds=$failedKinds"
    throw "ChatGPT feature-page smoke failed: failed_count=$($failed.Count) failed_kinds=$failedKinds"
}
Register-ChatGptWebVerificationCases -Runtime $runtime `
    -CaseIds @("safe/feature_pages", "safe/feature_pages_individual") `
    -ExpectedAdapterVersion $ExpectedAdapterVersion | Out-Null
Write-Output "CHATGPT_FEATURE_PAGE_SMOKE_STATUS=passed audited_count=$($results.Count)"
