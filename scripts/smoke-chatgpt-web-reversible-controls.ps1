#requires -Version 5.1

[CmdletBinding()]
param(
    [string]$Adb = "D:\Android\sdk\platform-tools\adb.exe",
    [Parameter(Mandatory = $true)][string]$DeviceSerial,
    [string]$ExpectedHardwareSerial = "",
    [ValidateRange(10, 180)][int]$ReadyTimeoutSec = 60,
    [ValidateRange(1, 10)][int]$PollIntervalSec = 1
)

$ErrorActionPreference = "Stop"
. (Join-Path $PSScriptRoot "chatgpt-web-smoke-runtime.ps1")

$runtime = New-ChatGptWebSmokeRuntime -Adb $Adb -DeviceSerial $DeviceSerial `
    -ExpectedHardwareSerial $ExpectedHardwareSerial -PollIntervalSec $PollIntervalSec
Assert-ChatGptWebSmokeTrustedDevice -Runtime $runtime

function Wait-CommandReceipt {
    param(
        [Parameter(Mandatory = $true)][string]$RequestId,
        [Parameter(Mandatory = $true)][string]$ExpectedAction
    )

    $deadline = [DateTimeOffset]::UtcNow.AddSeconds($ReadyTimeoutSec)
    do {
        $state = Invoke-ChatGptWebSmokeMcp -Runtime $runtime -Tool "ui_state"
        $receipt = @($state.command_requests) |
            Where-Object { [string]$_.request_id -eq $RequestId } |
            Select-Object -Last 1
        if ($null -ne $receipt -and [string]$receipt.status -eq "failed") {
            throw "ChatGPT command failed: $ExpectedAction"
        }
        if (
            $null -ne $receipt -and
            [string]$receipt.status -eq "succeeded" -and
            [string]$receipt.expected_web_action -eq $ExpectedAction -and
            $receipt.result.ok -eq $true
        ) {
            return $receipt
        }
        Start-Sleep -Seconds $runtime.poll_interval_sec
    } while ([DateTimeOffset]::UtcNow -lt $deadline)
    throw "Timed out waiting for ChatGPT command: $ExpectedAction"
}

function Invoke-ReceiptAction {
    param(
        [Parameter(Mandatory = $true)][string]$Action,
        [Parameter(Mandatory = $true)][string]$ExpectedAction,
        [hashtable]$Arguments = @{}
    )

    $dispatched = Invoke-ChatGptWebSmokeAction -Runtime $runtime -Action $Action `
        -Arguments $Arguments
    $requestId = [string]$dispatched.command_receipt.request_id
    if (-not $requestId) { throw "Missing command receipt for $Action" }
    return Wait-CommandReceipt -RequestId $requestId -ExpectedAction $ExpectedAction
}

function Get-ComposerModels {
    Invoke-ReceiptAction -Action "chatgpt_list_composer_options" `
        -ExpectedAction "list_model_options" -Arguments @{ section = "model" } | Out-Null
    $deadline = [DateTimeOffset]::UtcNow.AddSeconds($ReadyTimeoutSec)
    do {
        $navigation = Invoke-ChatGptWebSmokeAction -Runtime $runtime `
            -Action "chatgpt_get_navigation" -Arguments @{ section = "model" }
        $options = @($navigation.composer_sections.model | Where-Object { $null -ne $_ })
        if ($options.Count -gt 0) { return $options }
        Start-Sleep -Seconds $runtime.poll_interval_sec
    } while ([DateTimeOffset]::UtcNow -lt $deadline)
    throw "Timed out waiting for ChatGPT model options."
}

function Get-ManifestControls {
    Invoke-ReceiptAction -Action "chatgpt_refresh_controls" `
        -ExpectedAction "snapshot_ui_manifest" | Out-Null
    $controls = [System.Collections.Generic.List[object]]::new()
    $offset = 0
    do {
        $page = Invoke-ChatGptWebSmokeAction -Runtime $runtime `
            -Action "chatgpt_find_controls" -Arguments @{ offset = $offset; limit = 50 }
        @($page.controls | Where-Object { $null -ne $_ }).ForEach({ $controls.Add($_) })
        $offset = if ($null -eq $page.next_offset) { 0 } else { [int]$page.next_offset }
    } while ($page.has_more -eq $true -and $offset -gt 0)
    return @($controls)
}

function Get-ControlMatch {
    param([Parameter(Mandatory = $true)]$Reference)

    $controls = @(Get-ManifestControls)
    $byId = $controls |
        Where-Object { [string]$_.control_id -eq [string]$Reference.control_id } |
        Select-Object -First 1
    if ($null -ne $byId) { return $byId }
    return $controls |
        Where-Object {
            [string]$_.label -eq [string]$Reference.label -and
            [string]$_.role -eq [string]$Reference.role -and
            [string]$_.semantic -eq [string]$Reference.semantic -and
            [string]$_.context_id -eq [string]$Reference.context_id
        } |
        Select-Object -First 1
}

Invoke-ChatGptWebSmokeAction -Runtime $runtime -Action "open_chatgpt_web" `
    -EnsureMainActivity | Out-Null
$origin = Wait-ChatGptWebSmokeState -Runtime $runtime -TimeoutSec $ReadyTimeoutSec `
    -Description "authenticated ChatGPT composer" -Predicate {
        param($state)
        $state.surface -eq "chatgpt_web" -and
            $state.bridge_state -eq "ready" -and
            $state.authenticated -eq $true -and
            $state.composer_ready -eq $true
    }
$originViewMode = [string]$origin.view_mode

$models = @(Get-ComposerModels)
$originalModel = $models | Where-Object { $_.selected -eq $true } | Select-Object -First 1
$alternateModel = $models | Where-Object { $_.selected -ne $true } | Select-Object -First 1
if ($null -eq $originalModel -or $null -eq $alternateModel) {
    throw "At least two observable model choices are required for reversible acceptance."
}
$modelRestored = $false
try {
    Invoke-ReceiptAction -Action "chatgpt_select_composer_option" `
        -ExpectedAction "select_model_option" -Arguments @{
            section = "model"
            option_id = [string]$alternateModel.id
        } | Out-Null
    $changedModels = @(Get-ComposerModels)
    if (-not ($changedModels | Where-Object {
        [string]$_.id -eq [string]$alternateModel.id -and $_.selected -eq $true
    })) {
        throw "Alternate ChatGPT model did not become selected."
    }
} finally {
    Invoke-ReceiptAction -Action "chatgpt_select_composer_option" `
        -ExpectedAction "select_model_option" -Arguments @{
            section = "model"
            option_id = [string]$originalModel.id
        } | Out-Null
    $restoredModels = @(Get-ComposerModels)
    $modelRestored = $null -ne ($restoredModels | Where-Object {
        [string]$_.id -eq [string]$originalModel.id -and $_.selected -eq $true
    } | Select-Object -First 1)
}
if (-not $modelRestored) { throw "Original ChatGPT model was not restored." }

$expandable = @(Get-ManifestControls) |
    Where-Object { $_.expandable -eq $true -and $null -ne $_.expanded } |
    Select-Object -First 1
if ($null -eq $expandable) {
    throw "No expandable ChatGPT control is observable on the current page."
}
$originalExpanded = [bool]$expandable.expanded
$targetExpanded = -not $originalExpanded
$disclosureRestored = $false
try {
    Invoke-ReceiptAction -Action "chatgpt_set_control_expanded" `
        -ExpectedAction "set_ui_control_expanded" -Arguments @{
            control_id = [string]$expandable.control_id
            expanded = $targetExpanded
        } | Out-Null
    $changedControl = Get-ControlMatch -Reference $expandable
    if ($null -eq $changedControl -or [bool]$changedControl.expanded -ne $targetExpanded) {
        throw "ChatGPT disclosure control did not reach the requested state."
    }
    Invoke-ReceiptAction -Action "chatgpt_set_control_expanded" `
        -ExpectedAction "set_ui_control_expanded" -Arguments @{
            control_id = [string]$changedControl.control_id
            expanded = $targetExpanded
        } | Out-Null
    $idempotentControl = Get-ControlMatch -Reference $changedControl
    if ($null -eq $idempotentControl -or [bool]$idempotentControl.expanded -ne $targetExpanded) {
        throw "Repeated disclosure state request was not idempotent."
    }
} finally {
    $restoreTarget = Get-ControlMatch -Reference $expandable
    if ($null -eq $restoreTarget) { throw "Disclosure control disappeared before restoration." }
    Invoke-ReceiptAction -Action "chatgpt_set_control_expanded" `
        -ExpectedAction "set_ui_control_expanded" -Arguments @{
            control_id = [string]$restoreTarget.control_id
            expanded = $originalExpanded
        } | Out-Null
    $restoredControl = Get-ControlMatch -Reference $restoreTarget
    $disclosureRestored = $null -ne $restoredControl -and
        [bool]$restoredControl.expanded -eq $originalExpanded
}
if (-not $disclosureRestored) { throw "Original disclosure state was not restored." }

if ($originViewMode -in @("web", "native")) {
    Invoke-ChatGptWebSmokeAction -Runtime $runtime -Action "chatgpt_select_view" `
        -Arguments @{ view_mode = $originViewMode } | Out-Null
}

[ordered]@{
    schema = "elon.chatgpt_web.reversible_control_smoke.v1"
    passed = $true
    device_serial = $DeviceSerial
    sent_messages = 0
    uploaded_attachments = 0
    model_selection = [ordered]@{
        observed_choices = $models.Count
        changed = $true
        original_state_restored = $modelRestored
    }
    disclosure_control = [ordered]@{
        idempotent_request_passed = $true
        original_state_restored = $disclosureRestored
    }
    original_view_mode_restored = $true
} | ConvertTo-Json -Depth 10
Write-Output "CHATGPT_WEB_REVERSIBLE_CONTROL_SMOKE_STATUS=passed"
