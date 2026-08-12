#requires -Version 5.1

[CmdletBinding()]
param(
    [string]$Adb = "D:\Android\sdk\platform-tools\adb.exe",
    [Parameter(Mandatory = $true)][string]$DeviceSerial,
    [string]$ExpectedHardwareSerial = "",
    [ValidateRange(10, 180)][int]$ReadyTimeoutSec = 90,
    [ValidateRange(1, 10)][int]$PollIntervalSec = 2,
    [ValidateRange(1, 9999)][int]$ExpectedAdapterVersion = 78
)

$ErrorActionPreference = "Stop"
. (Join-Path $PSScriptRoot "chatgpt-web-smoke-runtime.ps1")

$runtime = New-ChatGptWebSmokeRuntime -Adb $Adb -DeviceSerial $DeviceSerial `
    -ExpectedHardwareSerial $ExpectedHardwareSerial -PollIntervalSec $PollIntervalSec
Assert-ChatGptWebSmokeTrustedDevice -Runtime $runtime

function Get-ObservedPath {
    param($State)

    $uri = $null
    if ([Uri]::TryCreate([string]$State.conversation.url, [UriKind]::Absolute, [ref]$uri)) {
        return $uri.AbsolutePath
    }
    return ""
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

function Invoke-ReadOnlyControlQuery {
    param(
        [Parameter(Mandatory = $true)]
        [ValidateSet("chatgpt_find_controls", "chatgpt_get_capability_matrix")]
        [string]$Action,
        [hashtable]$Arguments = @{}
    )

    $deadline = [DateTimeOffset]::UtcNow.AddSeconds($ReadyTimeoutSec)
    do {
        $state = Invoke-ChatGptWebSmokeMcp -Runtime $runtime -Tool "ui_state"
        if (
            $state.surface -eq "chatgpt_web" -and
            $state.bridge_state -eq "ready" -and
            $state.adapter_current -eq $true
        ) {
            try {
                return Invoke-ChatGptWebSmokeAction -Runtime $runtime -Action $Action `
                    -Arguments $Arguments
            } catch {
                if ($_.Exception.Message -notmatch "bridge_not_ready") { throw }
            }
        }
        Start-Sleep -Seconds $runtime.poll_interval_sec
    } while ([DateTimeOffset]::UtcNow -lt $deadline)
    throw "Timed out waiting to run read-only ChatGPT query: $Action"
}

function Get-ManifestControls {
    param(
        [string]$Semantic = "",
        [string]$Region = ""
    )

    $controls = [System.Collections.Generic.List[object]]::new()
    $offset = 0
    do {
        $arguments = @{ offset = $offset; limit = 50 }
        if ($Semantic) { $arguments.semantic = $Semantic }
        if ($Region) { $arguments.region = $Region }
        $page = Invoke-ReadOnlyControlQuery `
            -Action "chatgpt_find_controls" -Arguments $arguments
        @($page.controls | Where-Object { $null -ne $_ }).ForEach({ $controls.Add($_) })
        $offset = if ($null -eq $page.next_offset) { 0 } else { [int]$page.next_offset }
    } while ($page.has_more -eq $true -and $offset -gt 0)
    return @($controls)
}

function Wait-FirstControl {
    param(
        [Parameter(Mandatory = $true)][string]$Semantic,
        [Parameter(Mandatory = $true)][string]$Region
    )

    $deadline = [DateTimeOffset]::UtcNow.AddSeconds($ReadyTimeoutSec)
    do {
        $state = Invoke-ChatGptWebSmokeMcp -Runtime $runtime -Tool "ui_state"
        if (
            $state.surface -eq "chatgpt_web" -and
            $state.bridge_state -eq "ready" -and
            $state.adapter_current -eq $true
        ) {
            $control = @(Get-ManifestControls -Semantic $Semantic -Region $Region) |
                Select-Object -First 1
            if ($null -ne $control) { return $control }
        }
        Start-Sleep -Seconds $runtime.poll_interval_sec
    } while ([DateTimeOffset]::UtcNow -lt $deadline)
    throw "Timed out waiting for ChatGPT control semantic=$Semantic region=$Region"
}

function Wait-SettingsStructure {
    $deadline = [DateTimeOffset]::UtcNow.AddSeconds($ReadyTimeoutSec)
    do {
        $state = Invoke-ChatGptWebSmokeMcp -Runtime $runtime -Tool "ui_state"
        if (
            $state.surface -eq "chatgpt_web" -and
            $state.bridge_state -eq "ready" -and
            $state.adapter_current -eq $true
        ) {
            Invoke-ReceiptAction -Action "chatgpt_refresh_controls" `
                -ExpectedAction "snapshot_ui_manifest" | Out-Null
            $controls = @(Get-ManifestControls -Region "overlay")
            $tabs = @($controls | Where-Object { [string]$_.role -eq "tab" })
            $switches = @($controls | Where-Object { [string]$_.role -eq "switch" })
            if ($tabs.Count -gt 0 -and $switches.Count -gt 0) {
                return [pscustomobject]@{
                    matrix = Invoke-ReadOnlyControlQuery `
                        -Action "chatgpt_get_capability_matrix"
                    controls = $controls
                    tabs = $tabs
                    switches = $switches
                }
            }
        }
        Start-Sleep -Seconds $runtime.poll_interval_sec
    } while ([DateTimeOffset]::UtcNow -lt $deadline)
    throw "Timed out waiting for structural ChatGPT settings controls."
}

function Restore-Origin {
    param(
        [Parameter(Mandatory = $true)][string]$PageKind,
        [string]$Path,
        [Parameter(Mandatory = $true)][int]$OverlayControlCount
    )

    $deadline = [DateTimeOffset]::UtcNow.AddSeconds([Math]::Min(60, $ReadyTimeoutSec))
    $restoreStartedAt = [DateTimeOffset]::UtcNow
    $refreshAttempted = $false
    $backAttempts = 0
    do {
        $state = Invoke-ChatGptWebSmokeMcp -Runtime $runtime -Tool "ui_state"
        if (
            $state.surface -eq "chatgpt_web" -and
            $state.bridge_state -eq "ready" -and
            $state.adapter_current -eq $true -and
            $state.authenticated -eq $true
        ) {
            $overlay = Invoke-ReadOnlyControlQuery `
                -Action "chatgpt_find_controls" -Arguments @{ region = "overlay"; offset = 0; limit = 1 }
            $pathMatches = -not $Path -or (Get-ObservedPath -State $state) -eq $Path
            if (
                [string]$state.page_kind -eq $PageKind -and
                $pathMatches -and
                [int]$overlay.match_count -eq $OverlayControlCount
            ) {
                return
            }
            if ($backAttempts -ge 4) { break }
            Invoke-ChatGptWebSmokeAdb -Runtime $runtime `
                -Arguments @("shell", "input", "keyevent", "4") `
                -TimeoutSec 10 -Label "restore ChatGPT settings origin" | Out-Null
            $backAttempts += 1
        } elseif (
            -not $refreshAttempted -and
            $state.surface -eq "chatgpt_web" -and
            $state.bridge_state -eq "connecting" -and
            $state.adapter_current -eq $true -and
            $state.authenticated -eq $true -and
            ([DateTimeOffset]::UtcNow - $restoreStartedAt).TotalSeconds -ge 10
        ) {
            Invoke-ChatGptWebSmokeAction -Runtime $runtime -Action "chatgpt_refresh" | Out-Null
            $refreshAttempted = $true
        }
        Start-Sleep -Seconds $runtime.poll_interval_sec
    } while ([DateTimeOffset]::UtcNow -lt $deadline)
    throw "Timed out restoring the original ChatGPT page after settings audit."
}

Open-ChatGptWebSmokeSurface -Runtime $runtime | Out-Null
$origin = Wait-ChatGptWebSmokeAuthenticatedReady -Runtime $runtime `
    -TimeoutSec $ReadyTimeoutSec -InitialWaitSec ([Math]::Min(5, $ReadyTimeoutSec))
if ([string]$origin.view_mode -notin @("official", "web")) {
    Invoke-ChatGptWebSmokeAction -Runtime $runtime -Action "chatgpt_select_view" `
        -Arguments @{ view_mode = "official" } | Out-Null
    $origin = Wait-ChatGptWebSmokeAuthenticatedReady -Runtime $runtime `
        -TimeoutSec $ReadyTimeoutSec -InitialWaitSec 3
}
Assert-ChatGptWebSmokeAdapterVersion -State $origin `
    -ExpectedAdapterVersion $ExpectedAdapterVersion
$originPageKind = [string]$origin.page_kind
$originPath = Get-ObservedPath -State $origin
$originOverlay = Invoke-ReadOnlyControlQuery `
    -Action "chatgpt_find_controls" -Arguments @{ region = "overlay"; offset = 0; limit = 1 }
$originOverlayCount = [int]$originOverlay.match_count
$originOverlayControls = @(Get-ManifestControls -Region "overlay")
$settingsAlreadyOpen =
    @($originOverlayControls | Where-Object { [string]$_.role -eq "tab" }).Count -gt 0 -and
    @($originOverlayControls | Where-Object { [string]$_.role -eq "switch" }).Count -gt 0
$originRestored = $false
$report = $null

try {
    if (-not $settingsAlreadyOpen) {
        Invoke-ReceiptAction -Action "chatgpt_list_features" `
            -ExpectedAction "list_navigation" | Out-Null
        $profile = Wait-FirstControl -Semantic "profile" -Region "overlay"
        Invoke-ReceiptAction -Action "chatgpt_invoke_control" `
            -ExpectedAction "invoke_ui_control" `
            -Arguments @{ control_id = [string]$profile.control_id } | Out-Null
        Wait-FirstControl -Semantic "logout" -Region "overlay" | Out-Null
        $settings = Wait-FirstControl -Semantic "settings" -Region "overlay"
        Invoke-ReceiptAction -Action "chatgpt_invoke_control" `
            -ExpectedAction "invoke_ui_control" `
            -Arguments @{ control_id = [string]$settings.control_id } | Out-Null
    }

    $structure = Wait-SettingsStructure
    $matrix = $structure.matrix
    $tabs = @($structure.tabs)
    $switches = @($structure.switches)
    $tabContractFailures = @($tabs | Where-Object {
        [string]$_.semantic -ne "selection" -or
        $_.state_settable -ne $true -or
        [string]$_.native_presentation -ne "menu"
    })
    $switchContractFailures = @($switches | Where-Object {
        [string]$_.semantic -ne "toggle" -or
        $_.state_settable -ne $true -or
        [string]$_.native_presentation -ne "menu"
    })
    if ($matrix.control_ok -ne $true -or $matrix.ready_for_mcp -ne $true) {
        throw "Settings capability matrix is not ready for MCP."
    }
    if ([string]$matrix.manifest.compatibility -ne "healthy") {
        throw "Settings manifest compatibility is not healthy."
    }
    if ($matrix.manifest.controls_truncated -eq $true) {
        throw "Settings manifest controls were truncated."
    }
    if ([int]$matrix.manifest.generic_control_count -ne 0) {
        throw "Settings manifest still contains generic controls."
    }
    if ([int]$matrix.manifest.unexpected_official_fallback_control_count -ne 0) {
        throw "Settings manifest contains unexpected official fallback controls."
    }
    if (@($matrix.unknown_semantics).Count -ne 0 -or @($matrix.unknown_capabilities).Count -ne 0) {
        throw "Settings manifest contains unknown semantics or capabilities."
    }
    if ($matrix.adaptation_review.required -eq $true) {
        throw "Settings manifest still requires adaptation review."
    }
    if ($tabContractFailures.Count -ne 0 -or $switchContractFailures.Count -ne 0) {
        throw "Settings form controls are not fully mapped to native MCP semantics."
    }

    $activeTab = @($tabs | Where-Object { $_.selected -eq $true }) | Select-Object -First 1
    if ($null -eq $activeTab) { throw "Settings did not expose an active tab." }
    Invoke-ReceiptAction -Action "chatgpt_set_control_selected" `
        -ExpectedAction "set_ui_control_selected" `
        -Arguments @{
            control_id = [string]$activeTab.control_id
            selected = $true
        } | Out-Null
    $afterControls = @(Get-ManifestControls -Region "overlay")
    $activeTabAfter = @($afterControls | Where-Object {
        [string]$_.role -eq "tab" -and $_.selected -eq $true
    }) | Select-Object -First 1
    if ($null -eq $activeTabAfter) {
        throw "The idempotent settings tab command did not preserve an active tab."
    }

    $report = [ordered]@{
        schema = "elon.chatgpt_web.settings_smoke.v1"
        passed = $true
        adapter_version = [int]$matrix.adapter_version
        tab_count = $tabs.Count
        switch_count = $switches.Count
        generic_control_count = [int]$matrix.manifest.generic_control_count
        unexpected_fallback_count = [int]$matrix.manifest.unexpected_official_fallback_control_count
        idempotent_tab_selection = $true
        settings_already_open = $settingsAlreadyOpen
        changed_settings = $false
        sent_messages = 0
        uploaded_attachments = 0
        cleared_cookies = $false
        cleared_app_data = $false
    }
} finally {
    Restore-Origin -PageKind $originPageKind -Path $originPath `
        -OverlayControlCount $originOverlayCount
    $originRestored = $true
}

if (-not $originRestored) { throw "ChatGPT settings origin was not restored." }
$report | ConvertTo-Json -Depth 6
Write-Output "CHATGPT_SETTINGS_SMOKE_STATUS=passed"
