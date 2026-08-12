#requires -Version 5.1

[CmdletBinding()]
param(
    [string]$Adb = "D:\Android\sdk\platform-tools\adb.exe",
    [Parameter(Mandatory = $true)][string]$DeviceSerial,
    [string]$ExpectedHardwareSerial = "",
    [ValidateRange(10, 180)][int]$ReadyTimeoutSec = 60,
    [ValidateRange(1, 10)][int]$PollIntervalSec = 1,
    [ValidateRange(1, 9999)][int]$ExpectedAdapterVersion = 77
)

$ErrorActionPreference = "Stop"
. (Join-Path $PSScriptRoot "chatgpt-web-smoke-runtime.ps1")

$runtime = New-ChatGptWebSmokeRuntime -Adb $Adb -DeviceSerial $DeviceSerial `
    -ExpectedHardwareSerial $ExpectedHardwareSerial -PollIntervalSec $PollIntervalSec
Assert-ChatGptWebSmokeTrustedDevice -Runtime $runtime
$script:modelDiscoveryStage = "initial"

function Test-SelectableModelLeaf {
    param([Parameter(Mandatory = $true)]$Option)

    if ($Option.opens_submenu -eq $true) { return $false }
    $kind = [string]$Option.kind
    return $Option.selected -eq $true -or $kind -in @("menuitemradio", "option")
}

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

    $dispatched = Invoke-ChatGptWebSmokeReadyAction -Runtime $runtime -Action $Action `
        -Arguments $Arguments -TimeoutSec $ReadyTimeoutSec
    $requestId = [string]$dispatched.command_receipt.request_id
    if (-not $requestId) { throw "Missing command receipt for $Action" }
    return Wait-CommandReceipt -RequestId $requestId -ExpectedAction $ExpectedAction
}

function Get-CachedComposerModels {
    param(
        [switch]$RequireLeafChoices,
        [ValidateRange(1, 180)][int]$TimeoutSec = $ReadyTimeoutSec
    )

    $deadline = [DateTimeOffset]::UtcNow.AddSeconds($TimeoutSec)
    do {
        $navigation = Invoke-ChatGptWebSmokeAction -Runtime $runtime `
            -Action "chatgpt_get_navigation" -Arguments @{ section = "model" }
        $options = @($navigation.composer_sections.model | Where-Object { $null -ne $_ })
        $leafCount = @($options | Where-Object { Test-SelectableModelLeaf -Option $_ }).Count
        if ($options.Count -gt 0 -and (-not $RequireLeafChoices -or $leafCount -ge 2)) {
            return $options
        }
        Start-Sleep -Seconds $runtime.poll_interval_sec
    } while ([DateTimeOffset]::UtcNow -lt $deadline)
    throw "Timed out waiting for cached ChatGPT model options."
}

function Get-ComposerModels {
    $deadline = [DateTimeOffset]::UtcNow.AddSeconds($ReadyTimeoutSec)
    do {
        try {
            Invoke-ReceiptAction -Action "chatgpt_list_composer_options" `
                -ExpectedAction "list_model_options" -Arguments @{ section = "model" } | Out-Null
            return @(Get-CachedComposerModels)
        } catch {
            # The authenticated composer can become ready before the model entry finishes hydrating.
        }
        Start-Sleep -Seconds $runtime.poll_interval_sec
    } while ([DateTimeOffset]::UtcNow -lt $deadline)
    throw "Timed out waiting for ChatGPT model options during $script:modelDiscoveryStage."
}

function Wait-SelectedModel {
    param([Parameter(Mandatory = $true)][string]$ExpectedLabel)

    try {
        return Wait-ChatGptWebSmokeState -Runtime $runtime -TimeoutSec 5 `
            -Description "requested ChatGPT model summary" -Predicate {
            param($state)
            $state.adapter_current -eq $true -and
                [string]$state.conversation.current_model -eq $ExpectedLabel
        }
    } catch {
        # The official page can expose the selected menu item before its compact model label updates.
    }

    $models = @(Get-SelectableModels)
    $selected = $models |
        Where-Object {
            $_.selected -eq $true -and
                [string]$_.label -eq $ExpectedLabel
        } |
        Select-Object -First 1
    if ($null -eq $selected) {
        $current = Invoke-ChatGptWebSmokeMcp -Runtime $runtime -Tool "ui_state"
        throw "ChatGPT model menu did not mark the requested model as selected; expected=$ExpectedLabel current=$([string]$current.conversation.current_model)."
    }
    return Invoke-ChatGptWebSmokeMcp -Runtime $runtime -Tool "ui_state"
}

function Wait-ViewMode {
    param([Parameter(Mandatory = $true)][string]$ExpectedMode)

    return Wait-ChatGptWebSmokeState -Runtime $runtime -TimeoutSec $ReadyTimeoutSec `
        -Description "requested ChatGPT view mode" -Predicate {
            param($state)
            [string]$state.view_mode -eq $ExpectedMode -and
                $state.adapter_current -eq $true
        }
}

function Get-ConversationPathFromUrl {
    param([string]$Url)

    if ([string]::IsNullOrWhiteSpace($Url)) { return "" }
    try {
        $uri = [Uri]$Url
        if ($uri.Host -in @("chatgpt.com", "www.chatgpt.com") -and $uri.AbsolutePath -match '^/c/') {
            return $uri.AbsolutePath
        }
    } catch { }
    return ""
}

function Wait-ConversationPath {
    param([Parameter(Mandatory = $true)][string]$Path)

    $expectedPath = $Path
    return Wait-ChatGptWebSmokeState -Runtime $runtime -TimeoutSec $ReadyTimeoutSec `
        -Description "restored ChatGPT conversation" -Predicate {
            param($state)
            $state.bridge_state -eq "ready" -and
                [string]$state.conversation.url -like "*$expectedPath*"
        }.GetNewClosure()
}

function Wait-BlankConversation {
    return Wait-ChatGptWebSmokeState -Runtime $runtime -TimeoutSec $ReadyTimeoutSec `
        -Description "temporary blank ChatGPT conversation" -Predicate {
            param($state)
            $state.bridge_state -eq "ready" -and
                $state.authenticated -eq $true -and
                $state.composer_ready -eq $true -and
                [int]$state.conversation.message_count -eq 0 -and
                $state.streaming -eq $false
        }
}

function Get-SelectableModels {
    $root = @(Get-ComposerModels)
    $rootLeaves = @($root | Where-Object { Test-SelectableModelLeaf -Option $_ })
    if ($rootLeaves.Count -ge 2) { return $rootLeaves }

    $parents = @(
        $root |
            Where-Object { $_.opens_submenu -eq $true } |
            Sort-Object `
                @{ Expression = {
                    if ([string]$_.label -match '(?i)model|模型') { 0 }
                    elseif ([string]$_.label -match '(?i)reasoning|thinking|effort|思考|推理|强度') { 1 }
                    else { 2 }
                } }, `
                @{ Expression = { [string]$_.label } }
    )
    foreach ($parent in $parents) {
        Invoke-ReceiptAction -Action "chatgpt_select_composer_option" `
            -ExpectedAction "select_model_option" -Arguments @{
                section = "model"
                option_id = [string]$parent.id
            } | Out-Null
        try {
            $children = @(
                Get-CachedComposerModels -RequireLeafChoices `
                    -TimeoutSec ([Math]::Min($ReadyTimeoutSec, 15))
            )
        } catch {
            continue
        }
        $childLeaves = @($children | Where-Object { Test-SelectableModelLeaf -Option $_ })
        if ($childLeaves.Count -ge 2) { return $childLeaves }
    }
    throw "At least two selectable model choices are required for reversible acceptance."
}

function Find-ModelByLabel {
    param(
        [Parameter(Mandatory = $true)][object[]]$Models,
        [Parameter(Mandatory = $true)][string]$Label
    )

    return $Models | Where-Object { [string]$_.label -eq $Label } | Select-Object -First 1
}

function Restore-ModelByLabel {
    param([Parameter(Mandatory = $true)][string]$Label)

    foreach ($attempt in 1..2) {
        try {
            $restoreModels = @(Get-SelectableModels)
            $restoreModel = Find-ModelByLabel -Models $restoreModels -Label $Label
            if ($null -eq $restoreModel) { throw "Original model is not in the current menu." }
            Invoke-ReceiptAction -Action "chatgpt_select_composer_option" `
                -ExpectedAction "select_model_option" -Arguments @{
                    section = "model"
                    option_id = [string]$restoreModel.id
                } | Out-Null
            Wait-SelectedModel -ExpectedLabel $Label | Out-Null
            return $true
        } catch {
            if ($attempt -lt 2) { Start-Sleep -Seconds $runtime.poll_interval_sec }
        }
    }
    return $false
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
            $state.adapter_current -eq $true -and
            $state.authenticated -eq $true -and
            $state.composer_ready -eq $true
    }
Assert-ChatGptWebSmokeAdapterVersion -State $origin `
    -ExpectedAdapterVersion $ExpectedAdapterVersion
$originViewMode = [string]$origin.view_mode
$originConversationPath = Get-ConversationPathFromUrl -Url ([string]$origin.conversation.url)

$modelRestored = $false
$modelViewRestored = $false
$modelConversationRestored = $false
$temporaryConversationUsed = $false
try {
    if ($originViewMode -ne "web") {
        Invoke-ChatGptWebSmokeAction -Runtime $runtime -Action "chatgpt_select_view" `
            -Arguments @{ view_mode = "web" } | Out-Null
        Wait-ViewMode -ExpectedMode "web" | Out-Null
    }

    if ($originConversationPath) {
        Invoke-ChatGptWebSmokeAction -Runtime $runtime -Action "chatgpt_new_conversation" | Out-Null
        Wait-BlankConversation | Out-Null
        $temporaryConversationUsed = $true
    }

    $modelOrigin = Invoke-ChatGptWebSmokeMcp -Runtime $runtime -Tool "ui_state"
    $originalModelLabel = [string]$modelOrigin.conversation.current_model
    $script:modelDiscoveryStage = "discover_choices"
    $models = @(Get-SelectableModels)
    $originalModel = $null
    if (-not [string]::IsNullOrWhiteSpace($originalModelLabel)) {
        $originalModel = Find-ModelByLabel -Models $models -Label $originalModelLabel
    }
    if ($null -eq $originalModel) {
        $originalModel = $models | Where-Object { $_.selected -eq $true } | Select-Object -First 1
        $originalModelLabel = [string]$originalModel.label
    }
    $alternateModel = $models |
        Where-Object { [string]$_.label -ne $originalModelLabel } |
        Select-Object -First 1
    if ($null -eq $originalModel -or $null -eq $alternateModel) {
        throw "At least two observable model choices are required for reversible acceptance."
    }
    try {
        $script:modelDiscoveryStage = "verify_alternate"
        Invoke-ReceiptAction -Action "chatgpt_select_composer_option" `
            -ExpectedAction "select_model_option" -Arguments @{
                section = "model"
                option_id = [string]$alternateModel.id
            } | Out-Null
        Wait-SelectedModel -ExpectedLabel ([string]$alternateModel.label) | Out-Null
    } finally {
        $script:modelDiscoveryStage = "restore_original"
        $modelRestored = Restore-ModelByLabel -Label $originalModelLabel
    }
} finally {
    if ($temporaryConversationUsed) {
        Invoke-ChatGptWebSmokeAction -Runtime $runtime -Action "chatgpt_open_conversation" `
            -Arguments @{ conversation_path = $originConversationPath } | Out-Null
        Wait-ConversationPath -Path $originConversationPath | Out-Null
    }
    $modelConversationRestored = $true
    Invoke-ChatGptWebSmokeAction -Runtime $runtime -Action "chatgpt_select_view" `
        -Arguments @{ view_mode = $originViewMode } | Out-Null
    Wait-ViewMode -ExpectedMode $originViewMode | Out-Null
    $modelViewRestored = $true
}
if (-not $modelRestored) { throw "Original ChatGPT model was not restored." }

$expandable = @(Get-ManifestControls) |
    Where-Object {
        $_.expandable -eq $true -and
        $null -ne $_.expanded -and
        [string]$_.region -eq "composer" -and
        [string]$_.semantic -in @("model", "attachment")
    } |
    Sort-Object @{ Expression = { if ([string]$_.semantic -eq "model") { 0 } else { 1 } } } |
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
        original_conversation_restored = $modelConversationRestored
    }
    disclosure_control = [ordered]@{
        idempotent_request_passed = $true
        original_state_restored = $disclosureRestored
    }
    original_view_mode_restored = $modelViewRestored
} | ConvertTo-Json -Depth 10
Write-Output "CHATGPT_WEB_REVERSIBLE_CONTROL_SMOKE_STATUS=passed"
