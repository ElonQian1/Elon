#requires -Version 5.1

[CmdletBinding()]
param(
    [string]$Adb = "D:\Android\sdk\platform-tools\adb.exe",
    [string]$DeviceSerial = "",
    [string]$ExpectedHardwareSerial = "",
    [int]$ReadyTimeoutSec = 90,
    [int]$ReplyTimeoutSec = 90,
    [int]$PollIntervalSec = 3,
    [ValidateRange(1, 9999)][int]$ExpectedAdapterVersion = 54,
    [switch]$SendProbe,
    [string]$ProbeMarker = ""
)

$ErrorActionPreference = "Stop"

$invokeMcp = Join-Path $PSScriptRoot "invoke-apk-mcp.ps1"
$evidenceHelper = Join-Path $PSScriptRoot "chatgpt-web-smoke-evidence.ps1"
$runtimeHelper = Join-Path $PSScriptRoot "chatgpt-web-smoke-runtime.ps1"
foreach ($helper in @($invokeMcp, $evidenceHelper, $runtimeHelper)) {
    if (-not (Test-Path -LiteralPath $helper -PathType Leaf)) {
        throw "Missing ChatGPT Web smoke helper: $helper"
    }
}
. $evidenceHelper
. $runtimeHelper
if (-not (Test-Path -LiteralPath $Adb -PathType Leaf)) {
    throw "adb not found: $Adb"
}
if ($ReadyTimeoutSec -lt 5 -or $ReplyTimeoutSec -lt 10 -or $PollIntervalSec -lt 1) {
    throw "Timeouts are too small for a reliable ChatGPT Web smoke run."
}
if (-not $SendProbe -and $ProbeMarker) {
    throw "ProbeMarker requires -SendProbe because the default smoke is read-only."
}

$checks = [System.Collections.Generic.List[object]]::new()

function Add-Check {
    param(
        [Parameter(Mandatory = $true)][string]$Name,
        [Parameter(Mandatory = $true)][bool]$Passed,
        [string]$Detail = ""
    )

    $checks.Add([pscustomobject]@{
        name = $Name
        passed = $Passed
        detail = $Detail
    })
    $status = if ($Passed) { "OK" } else { "FAIL" }
    Write-Output "$status`t$Name`t$Detail"
}

function Invoke-ApkMcp {
    param(
        [Parameter(Mandatory = $true)][string]$Tool,
        [hashtable]$Arguments = @{},
        [switch]$EnsureMainActivity
    )

    $params = @{
        Adb = $Adb
        DeviceSerial = $DeviceSerial
        Tool = $Tool
        Arguments = ($Arguments | ConvertTo-Json -Depth 20 -Compress)
    }
    if ($EnsureMainActivity) {
        $params.EnsureMainActivity = $true
        $params.OpenAppOnFailure = $true
    }
    $response = & $invokeMcp @params
    if ($response.result.isError) {
        throw "APK MCP tool failed: $Tool"
    }
    $structured = $response.result.structuredContent
    if ($null -eq $structured) {
        throw "APK MCP tool returned no structured content: $Tool"
    }
    return $structured
}

function Invoke-UiAction {
    param(
        [Parameter(Mandatory = $true)][string]$Action,
        [hashtable]$Arguments = @{},
        [switch]$EnsureMainActivity
    )

    $payload = @{} + $Arguments
    $payload.action = $Action
    Invoke-ApkMcp -Tool "ui_control" -Arguments $payload -EnsureMainActivity:$EnsureMainActivity
}

function Wait-ChatGptState {
    param(
        [Parameter(Mandatory = $true)][scriptblock]$Predicate,
        [Parameter(Mandatory = $true)][int]$TimeoutSec,
        [Parameter(Mandatory = $true)][string]$Description
    )

    $deadline = [DateTimeOffset]::UtcNow.AddSeconds($TimeoutSec)
    $last = $null
    do {
        $last = Invoke-ApkMcp -Tool "ui_state"
        if (& $Predicate $last) { return $last }
        Start-Sleep -Seconds $PollIntervalSec
    } while ([DateTimeOffset]::UtcNow -lt $deadline)
    throw "Timed out waiting for $Description. Last bridge=$($last.bridge_state), surface=$($last.surface)."
}

function Invoke-Adb {
    $serialArgs = if ($DeviceSerial.Trim()) { @("-s", $DeviceSerial.Trim()) } else { @() }
    & $Adb @serialArgs @args
}

function Get-TopResumedActivity {
    $line = @(Invoke-Adb shell dumpsys activity activities) |
        Where-Object { $_ -match 'topResumedActivity=' } |
        Select-Object -First 1
    if ($null -eq $line) { return "" }
    return ([string]$line).Trim()
}

function Wait-ChatGptActivityForeground {
    $deadline = [DateTimeOffset]::UtcNow.AddSeconds(10)
    do {
        $top = Get-TopResumedActivity
        if ($top -match 'com\.elon\.app/\.chatgptweb\.ChatGptWebTestActivity\b') {
            return $top
        }
        Start-Sleep -Milliseconds 250
    } while ([DateTimeOffset]::UtcNow -lt $deadline)
    return $top
}

function Get-VisibleUiXml {
    $remotePath = "/sdcard/elon-chatgpt-web-smoke.xml"
    Invoke-Adb shell uiautomator dump $remotePath | Out-Null
    if ($LASTEXITCODE -ne 0) { throw "UIAutomator dump failed." }
    $xml = (Invoke-Adb shell cat $remotePath) -join "`n"
    if ($LASTEXITCODE -ne 0) { throw "Unable to read UIAutomator dump." }
    return $xml
}

function Get-VisibleNativeSelectors {
    param([string]$UiXml = "")

    if ([string]::IsNullOrWhiteSpace($UiXml)) { $UiXml = Get-VisibleUiXml }
    return @(Get-ChatGptNativeSelectorsFromXml -UiXml $UiXml)
}

function Wait-VisibleNativeSelectors {
    param(
        [Parameter(Mandatory = $true)][string[]]$RequiredPrefixes,
        [Parameter(Mandatory = $true)][int]$TimeoutSec
    )

    $deadline = [DateTimeOffset]::UtcNow.AddSeconds($TimeoutSec)
    $last = @()
    do {
        $last = @(Get-VisibleNativeSelectors)
        $missing = @($RequiredPrefixes | Where-Object {
            $prefix = $_
            @($last | Where-Object { $_.StartsWith($prefix) }).Count -eq 0
        })
        if ($missing.Count -eq 0) { return $last }
        Start-Sleep -Milliseconds 500
    } while ([DateTimeOffset]::UtcNow -lt $deadline)
    throw "Timed out waiting for stable native selectors. Visible=$($last.Count), missing=$($missing.Count)."
}

function Wait-CommandResult {
    param(
        [Parameter(Mandatory = $true)][string]$Action,
        [Parameter(Mandatory = $true)][long]$AfterMs,
        [Parameter(Mandatory = $true)][int]$TimeoutSec
    )

    $deadline = [DateTimeOffset]::UtcNow.AddSeconds($TimeoutSec)
    $last = $null
    do {
        $last = Invoke-ApkMcp -Tool "ui_state"
        $command = $last.last_command
        if (
            $null -ne $command -and
            $command.action -eq $Action -and
            [long]$command.observed_at_ms -gt $AfterMs
        ) {
            return $last
        }
        Start-Sleep -Seconds $PollIntervalSec
    } while ([DateTimeOffset]::UtcNow -lt $deadline)
    throw "Timed out waiting for $Action command result. Last action=$($last.last_command.action)."
}

function Wait-NavigationReady {
    param(
        [Parameter(Mandatory = $true)][long]$AfterMs,
        [Parameter(Mandatory = $true)][int]$TimeoutSec
    )

    $deadline = [DateTimeOffset]::UtcNow.AddSeconds($TimeoutSec)
    $last = $null
    do {
        $last = Invoke-ApkMcp -Tool "ui_state"
        $command = $last.last_command
        $fresh = $null -ne $command -and [long]$command.observed_at_ms -gt $AfterMs
        $collected = $command.action -eq "collect_navigation"
        $navigation = Invoke-UiAction -Action "chatgpt_get_navigation"
        $features = @($navigation.features | Where-Object { $null -ne $_ })
        $cachedSnapshot = $navigation.control_ok -eq $true -and $features.Count -gt 0
        $matrix = Invoke-UiAction -Action "chatgpt_get_capability_matrix"
        $overlayOpen = [int]$matrix.observed_semantics.close -gt 0
        if ($fresh -and ($collected -or $cachedSnapshot) -and $overlayOpen) {
            return [pscustomobject]@{
                command_state = $last
                navigation = $navigation
            }
        }
        Start-Sleep -Seconds $PollIntervalSec
    } while ([DateTimeOffset]::UtcNow -lt $deadline)
    throw "Timed out waiting for ChatGPT navigation readiness. Last action=$($last.last_command.action)."
}

function Wait-AccountMenuReady {
    param([Parameter(Mandatory = $true)][int]$TimeoutSec)

    $deadline = [DateTimeOffset]::UtcNow.AddSeconds($TimeoutSec)
    $last = $null
    do {
        $last = Invoke-UiAction -Action "chatgpt_get_capability_matrix"
        $settings = [int]$last.observed_semantics.settings
        $logout = [int]$last.observed_semantics.logout
        if ($settings -gt 0 -and $logout -gt 0) { return $last }
        Start-Sleep -Seconds $PollIntervalSec
    } while ([DateTimeOffset]::UtcNow -lt $deadline)
    throw "Timed out waiting for the ChatGPT account menu semantic manifest."
}

function Wait-AccountMenuClosed {
    param([Parameter(Mandatory = $true)][int]$TimeoutSec)

    $deadline = [DateTimeOffset]::UtcNow.AddSeconds($TimeoutSec)
    do {
        $last = Invoke-ApkMcp -Tool "ui_state"
        $accountControls = @(
            $last.ui_manifest.controls |
                Where-Object {
                    $_.region -eq "overlay" -and $_.semantic -in @("settings", "logout")
                }
        )
        if ($accountControls.Count -eq 0) { return $last }
        Start-Sleep -Milliseconds 250
    } while ([DateTimeOffset]::UtcNow -lt $deadline)
    throw "Timed out waiting for the ChatGPT account menu to close."
}

function Wait-ComposerOptionsReady {
    param(
        [Parameter(Mandatory = $true)][ValidateSet("model", "tools")][string]$Section,
        [Parameter(Mandatory = $true)][long]$AfterMs,
        [Parameter(Mandatory = $true)][int]$TimeoutSec
    )

    $deadline = [DateTimeOffset]::UtcNow.AddSeconds($TimeoutSec)
    $expectedAction = if ($Section -eq "model") { "collect_model_options" } else { "collect_composer_tools" }
    $lastState = $null
    do {
        $lastState = Invoke-ApkMcp -Tool "ui_state"
        $navigation = Invoke-UiAction -Action "chatgpt_get_navigation" -Arguments @{ section = $Section }
        $sectionProperty = $navigation.composer_sections.PSObject.Properties[$Section]
        $options = if ($null -eq $sectionProperty) { @() } else { @($sectionProperty.Value) }
        $command = $lastState.last_command
        $freshCollection = $command.action -eq $expectedAction -and
            $command.ok -eq $true -and
            [long]$command.observed_at_ms -gt $AfterMs
        $cachedSnapshot = $navigation.control_ok -eq $true -and $options.Count -gt 0
        if ($freshCollection -or $cachedSnapshot) {
            return [pscustomobject]@{
                command_state = $lastState
                options = $options
            }
        }
        Start-Sleep -Seconds $PollIntervalSec
    } while ([DateTimeOffset]::UtcNow -lt $deadline)
    throw "Timed out waiting for $Section composer options. Last action=$($lastState.last_command.action)."
}

function Get-ComposerOptions {
    param(
        [Parameter(Mandatory = $true)][ValidateSet("model", "tools")][string]$Section,
        [int]$TimeoutSec = $ReadyTimeoutSec
    )

    $beforeState = Invoke-ApkMcp -Tool "ui_state"
    $afterMs = [long]$beforeState.last_command.observed_at_ms
    Invoke-UiAction -Action "chatgpt_list_composer_options" -Arguments @{ section = $Section } | Out-Null
    return Wait-ComposerOptionsReady -Section $Section -AfterMs $afterMs -TimeoutSec $TimeoutSec
}

function Get-ForeignComposerLabels {
    param([object[]]$Options)

    $foreignPattern = 'download\s+chatgpt|chatgpt\s+(desktop|mobile)|settings?|personalization|profile|log\s*out|sign\s*out|help|account|下载|桌面版|移动版|设置|个性化|个人资料|退出登录|帮助|账户|帐户|账号'
    return @($Options | Where-Object { [string]$_.label -match $foreignPattern } | ForEach-Object { [string]$_.label })
}

function Wait-NewConversationReady {
    param(
        [string]$PreviousUrl,
        [int]$PreviousMessageCount,
        [Parameter(Mandatory = $true)][int]$TimeoutSec
    )

    $deadline = [DateTimeOffset]::UtcNow.AddSeconds($TimeoutSec)
    $last = $null
    do {
        $last = Invoke-ApkMcp -Tool "ui_state"
        $currentUrl = [string]$last.conversation.url
        $currentCount = [int]$last.conversation.message_count
        $isolated = $currentUrl -ne $PreviousUrl -or
            ($PreviousMessageCount -eq 0 -and $currentCount -eq 0)
        if (
            $last.bridge_state -eq "ready" -and
            $last.composer_ready -eq $true -and
            $last.streaming -eq $false -and
            $currentCount -eq 0 -and
            $isolated
        ) {
            return $last
        }
        Start-Sleep -Seconds $PollIntervalSec
    } while ([DateTimeOffset]::UtcNow -lt $deadline)
    throw "Timed out proving an isolated blank ChatGPT conversation."
}
$smokeRuntime = New-ChatGptWebSmokeRuntime -Adb $Adb -DeviceSerial $DeviceSerial `
    -ExpectedHardwareSerial $ExpectedHardwareSerial -PollIntervalSec $PollIntervalSec
Assert-ChatGptWebSmokeTrustedDevice -Runtime $smokeRuntime
Start-ChatGptWebSmokeAwakeLease -Runtime $smokeRuntime | Out-Null
try {
$opened = Invoke-UiAction -Action "open_chatgpt_web" -EnsureMainActivity
$officialView = Invoke-UiAction -Action "chatgpt_select_view" -Arguments @{ view_mode = "official" }
$state = Wait-ChatGptState -TimeoutSec $ReadyTimeoutSec -Description "ChatGPT Web readiness" -Predicate {
    param($value)
    $value.surface -eq "chatgpt_web" -and
        $value.bridge_state -eq "ready" -and
        $value.activity_bound -eq $true
}
Assert-ChatGptWebSmokeAdapterVersion -State $state `
    -ExpectedAdapterVersion $ExpectedAdapterVersion
$topResumedActivity = Wait-ChatGptActivityForeground
$officialUiXml = Get-VisibleUiXml

Add-Check "open_chatgpt_web" ($opened.control_ok -eq $true) ([string]$opened.action)
Add-Check "official_view_selected" ($officialView.control_ok -eq $true) ([string]$officialView.view_mode)
Add-Check "chatgpt_target_bound" (
    $opened.target_activity_bound -eq $true -or $opened.surface -eq "chatgpt_web"
) ([string]$opened.target_surface)
Add-Check "chatgpt_activity_foreground" (
    $topResumedActivity -match 'com\.elon\.app/\.chatgptweb\.ChatGptWebTestActivity\b'
) $topResumedActivity
Add-Check "chatgpt_surface" ($state.surface -eq "chatgpt_web") ([string]$state.surface)
Add-Check "official_fullscreen_mode" ($state.view_mode -eq "web") ([string]$state.view_mode)
foreach ($chromeId in @("chatGptWebToolbar", "chatGptWebStatus", "chatGptModeToggle")) {
    Add-Check "official_fullscreen_chrome_$chromeId" (
        -not (Test-ChatGptResourceVisible -UiXml $officialUiXml -ResourceId $chromeId)
    ) $chromeId
}
Add-Check "bridge_ready" ($state.bridge_state -eq "ready") ([string]$state.bridge_state)
Add-Check "authenticated" ($state.authenticated -eq $true) ([string]$state.authenticated)
Add-Check "composer_ready" ($state.composer_ready -eq $true) ([string]$state.composer_ready)
Add-Check "adapter_version" (
    [int]$state.adapter_version -eq $ExpectedAdapterVersion
) ([string]$state.adapter_version)

$matrix = Invoke-UiAction -Action "chatgpt_get_capability_matrix"
$blockingGaps = @($matrix.blocking_gaps)
$unknownCapabilities = @($matrix.unknown_capabilities)
$unknownSemantics = @($matrix.unknown_semantics)
$adaptationRequired = $matrix.adaptation_review.required -eq $true
$adaptationReasons = @($matrix.adaptation_review.reasons)
$featureBaseline = $matrix.feature_baseline
$baselineSummary = $featureBaseline.summary
$baselineStatusTotal = [int]$baselineSummary.complete +
    [int]$baselineSummary.partial +
    [int]$baselineSummary.fallback_only
Add-Check "capability_matrix_ready" ($matrix.ready_for_mcp -eq $true) ([string]$matrix.schema)
Add-Check "capability_matrix_app_version" (
    [int]$matrix.app.version_code -gt 0 -and
        -not [string]::IsNullOrWhiteSpace([string]$matrix.app.version_name)
) "v$($matrix.app.version_name) build=$($matrix.app.version_code)"
Add-Check "feature_baseline_schema" (
    $featureBaseline.schema -eq "elon.chatgpt_web.feature_baseline.v4"
) ([string]$featureBaseline.schema)
$currentEvidenceInput = [string]$featureBaseline.device_verification_input_sha256
$verifiedEvidenceInput = [string]$featureBaseline.device_verification_verified_input_sha256
$evidenceProvenance = $featureBaseline.device_verification_provenance
Add-Check "feature_device_evidence_current" (
    $featureBaseline.device_verification_current -eq $true -and
        [int]$featureBaseline.device_verification_adapter_version -eq [int]$state.adapter_version -and
        $currentEvidenceInput -match '^[0-9a-f]{64}$' -and
        $currentEvidenceInput -eq $verifiedEvidenceInput
) "evidence_adapter=$($featureBaseline.device_verification_adapter_version),runtime_adapter=$($state.adapter_version),input=$($currentEvidenceInput.Substring(0, [Math]::Min(12, $currentEvidenceInput.Length)))"
Add-Check "feature_device_evidence_provenance" (
    $evidenceProvenance.schema -eq "elon.chatgpt_web.device_evidence.v1" -and
        [int]$evidenceProvenance.verified_apk_version_code -gt 0 -and
        -not [string]::IsNullOrWhiteSpace([string]$evidenceProvenance.verified_apk_version_name) -and
        [string]$evidenceProvenance.verified_source_commit -match '^[0-9a-f]{40}$'
) "v$($evidenceProvenance.verified_apk_version_name) build=$($evidenceProvenance.verified_apk_version_code) source=$(([string]$evidenceProvenance.verified_source_commit).Substring(0, [Math]::Min(12, ([string]$evidenceProvenance.verified_source_commit).Length)))"
Add-Check "feature_baseline_complete" (
    [int]$featureBaseline.feature_count -gt 0 -and
        $baselineStatusTotal -eq [int]$featureBaseline.feature_count -and
        [int]$baselineSummary.remaining -eq @($featureBaseline.remaining_feature_ids).Count
) "features=$($featureBaseline.feature_count),remaining=$($baselineSummary.remaining)"
$codeSummary = $featureBaseline.code_summary
$verificationSummary = $featureBaseline.verification_summary
Add-Check "feature_code_status_complete" (
    [int]$codeSummary.implemented +
        [int]$codeSummary.partial +
        [int]$codeSummary.official_fallback -eq [int]$featureBaseline.feature_count -and
        [int]$codeSummary.remaining -eq @($featureBaseline.remaining_code_feature_ids).Count
) "implemented=$($codeSummary.implemented),remaining=$($codeSummary.remaining)"
Add-Check "feature_verification_status_complete" (
    [int]$verificationSummary.offline_verified +
        [int]$verificationSummary.device_verified +
        [int]$verificationSummary.user_action_required +
        [int]$verificationSummary.deferred +
        [int]$verificationSummary.failed -eq [int]$featureBaseline.feature_count -and
        [int]$verificationSummary.remaining -eq @($featureBaseline.pending_verification_feature_ids).Count
) "device=$($verificationSummary.device_verified),offline=$($verificationSummary.offline_verified),remaining=$($verificationSummary.remaining)"
Add-Check "blocking_gaps" ($blockingGaps.Count -eq 0) ($blockingGaps -join ",")
Add-Check "unknown_capabilities" ($unknownCapabilities.Count -eq 0) ($unknownCapabilities -join ",")
Add-Check "unknown_semantics" ($unknownSemantics.Count -eq 0) ($unknownSemantics -join ",")
Add-Check "adaptation_review" (-not $adaptationRequired) ($adaptationReasons -join ",")

$contextEvidence = Get-ChatGptContextPagingEvidence -MessageOffset ([int]$state.conversation.message_window_start) `
    -InvokeUiAction { param($action, $arguments) Invoke-UiAction -Action $action -Arguments $arguments }
$contextFirst = $contextEvidence.first
$contextReplay = $contextEvidence.replay
$contextNext = $contextEvidence.next
Add-Check "context_page" ($contextFirst.control_ok -eq $true) ([string]$contextFirst.schema)
Add-Check "context_schema" ($contextFirst.schema -eq "elon.chatgpt_web.context.v2") ([string]$contextFirst.schema)
Add-Check "context_cursor_roundtrip" (
    $null -ne $contextReplay -and
        $contextReplay.control_ok -eq $true -and
        $contextReplay.context_revision -eq $contextFirst.context_revision -and
        [int]$contextReplay.message_offset -eq [int]$contextFirst.message_offset
) "offset=$($contextFirst.message_offset)"
$nextContextValid = if ($contextFirst.has_more -eq $true) {
    $null -ne $contextNext -and
        $contextNext.control_ok -eq $true -and
        $contextNext.context_revision -eq $contextFirst.context_revision -and
        [int]$contextNext.message_offset -eq [int]$contextFirst.next_message_offset
} else {
    $null -eq $contextNext
}
Add-Check "context_cursor_next" $nextContextValid "has_more=$($contextFirst.has_more)"

$beforeFeaturesState = Invoke-ApkMcp -Tool "ui_state"
$beforeFeatures = [long]$beforeFeaturesState.last_command.observed_at_ms
Invoke-UiAction -Action "chatgpt_list_features" | Out-Null
$featuresState = Wait-NavigationReady -AfterMs $beforeFeatures -TimeoutSec $ReadyTimeoutSec
Add-Check "composer_contamination_setup" (
    $featuresState.command_state.last_command.ok -eq $true
) "official sidebar opened"
$navigationMatrix = Invoke-UiAction -Action "chatgpt_get_capability_matrix"
$navigationAdaptationRequired = $navigationMatrix.adaptation_review.required -eq $true
$navigationAdaptationReasons = @($navigationMatrix.adaptation_review.reasons)
$navigationCloseCount = [int]$navigationMatrix.observed_semantics.close
Add-Check "navigation_overlay_open" ($navigationCloseCount -gt 0) ([string]$navigationCloseCount)
Add-Check "navigation_adaptation_review" (
    -not $navigationAdaptationRequired
) ($navigationAdaptationReasons -join ",")

$profileControls = @(
    $navigationMatrix.control_coverage |
        Where-Object { $_.semantic -eq "profile" -and $_.region -eq "overlay" }
)
Add-Check "account_menu_entry" ($profileControls.Count -gt 0) ([string]$profileControls.Count)
$accountMenuMatrix = $null
if ($profileControls.Count -gt 0) {
    $accountMenuOpen = Invoke-UiAction -Action "chatgpt_invoke_control" -Arguments @{
        control_id = [string]$profileControls[0].control_id
    }
    Add-Check "account_menu_open" ($accountMenuOpen.control_ok -eq $true) ([string]$accountMenuOpen.action)
    $accountMenuMatrix = Wait-AccountMenuReady -TimeoutSec $ReadyTimeoutSec
    $accountMenuReasons = @($accountMenuMatrix.adaptation_review.reasons)
    Add-Check "account_menu_settings" (
        [int]$accountMenuMatrix.observed_semantics.settings -gt 0
    ) ([string]$accountMenuMatrix.observed_semantics.settings)
    Add-Check "account_menu_logout" (
        [int]$accountMenuMatrix.observed_semantics.logout -gt 0
    ) ([string]$accountMenuMatrix.observed_semantics.logout)
    Add-Check "account_menu_generic_controls" (
        [int]$accountMenuMatrix.manifest.generic_control_count -eq 0
    ) ([string]$accountMenuMatrix.manifest.generic_control_count)
    Add-Check "account_menu_adaptation_review" (
        $accountMenuMatrix.adaptation_review.required -ne $true
    ) ($accountMenuReasons -join ",")
    $accountMenuClose = Invoke-UiAction -Action "chatgpt_invoke_control" -Arguments @{
        control_id = [string]$profileControls[0].control_id
    }
    Add-Check "account_menu_close" (
        $accountMenuClose.control_ok -eq $true
    ) ([string]$accountMenuClose.action)
    Wait-AccountMenuClosed -TimeoutSec $ReadyTimeoutSec | Out-Null
}

$composerOptionsOriginPath = ""
try {
    $composerOptionsOriginUri = [Uri][string]$state.conversation.url
    if (
        $composerOptionsOriginUri.Host -in @("chatgpt.com", "www.chatgpt.com") -and
        $composerOptionsOriginUri.AbsolutePath -match '^/c/'
    ) {
        $composerOptionsOriginPath = $composerOptionsOriginUri.AbsolutePath
    }
} catch { }
$temporaryComposerConversation = $false
$composerOptionsOriginRestored = $false
try {
    $modelResult = $null
    $modelOptionFailure = $null
    $initialComposerTimeoutSec = [Math]::Min(20, $ReadyTimeoutSec)
    try {
        $modelResult = Get-ComposerOptions -Section "model" -TimeoutSec $initialComposerTimeoutSec
    } catch {
        $modelOptionFailure = $_
    }
    if ($null -eq $modelResult -and $composerOptionsOriginPath) {
        $beforeComposerNewState = Invoke-ApkMcp -Tool "ui_state"
        $beforeComposerNew = [long]$beforeComposerNewState.last_command.observed_at_ms
        Invoke-UiAction -Action "chatgpt_new_conversation" | Out-Null
        $composerNewState = Wait-CommandResult -Action "new_conversation" `
            -AfterMs $beforeComposerNew -TimeoutSec $ReadyTimeoutSec
        if ($composerNewState.last_command.ok -ne $true) {
            throw "Temporary blank ChatGPT conversation was not accepted."
        }
        Wait-NewConversationReady `
            -PreviousUrl ([string]$beforeComposerNewState.conversation.url) `
            -PreviousMessageCount ([int]$beforeComposerNewState.conversation.message_count) `
            -TimeoutSec $ReadyTimeoutSec | Out-Null
        $temporaryComposerConversation = $true
        $modelResult = Get-ComposerOptions -Section "model"
    }
    if ($null -eq $modelResult) { throw $modelOptionFailure }
    $modelOptions = @($modelResult.options)
    $modelLabels = @($modelOptions | ForEach-Object { [string]$_.label })
    $foreignModelLabels = Get-ForeignComposerLabels -Options $modelOptions
    Add-Check "composer_model_options" ($modelOptions.Count -gt 0) ($modelLabels -join ",")
    Add-Check "composer_model_scope" ($foreignModelLabels.Count -eq 0) ($foreignModelLabels -join ",")
    if ($modelOptions.Count -gt 0) {
        Invoke-Adb shell input keyevent 4 | Out-Null
        Start-Sleep -Milliseconds 500
    }

    $toolsResult = Get-ComposerOptions -Section "tools"
    $toolOptions = @($toolsResult.options)
    $toolLabels = @($toolOptions | ForEach-Object { [string]$_.label })
    $foreignToolLabels = Get-ForeignComposerLabels -Options $toolOptions
    Add-Check "composer_tool_options" ($toolOptions.Count -gt 0) ($toolLabels -join ",")
    Add-Check "composer_tool_scope" ($foreignToolLabels.Count -eq 0) ($foreignToolLabels -join ",")
    if ($toolsResult.command_state.last_command.ok -eq $true -and $toolOptions.Count -gt 0) {
        Invoke-Adb shell input keyevent 4 | Out-Null
        Start-Sleep -Milliseconds 500
    }
} finally {
    if ($temporaryComposerConversation) {
        Invoke-UiAction -Action "chatgpt_open_conversation" `
            -Arguments @{ conversation_path = $composerOptionsOriginPath } | Out-Null
        $expectedComposerOriginPath = $composerOptionsOriginPath
        Wait-ChatGptState -TimeoutSec $ReadyTimeoutSec `
            -Description "restored ChatGPT conversation after composer inspection" -Predicate {
                param($value)
                $value.bridge_state -eq "ready" -and
                    [string]$value.conversation.url -like "*$expectedComposerOriginPath*"
            }.GetNewClosure() | Out-Null
    }
    $composerOptionsOriginRestored = $true
}
Add-Check "composer_options_origin_restored" $composerOptionsOriginRestored `
    "temporary_blank=$temporaryComposerConversation"

$probe = $null
if ($SendProbe) {
    if (-not $ProbeMarker) {
        $ProbeMarker = "ELON-CHATGPT-WEB-SMOKE-$([DateTimeOffset]::UtcNow.ToUnixTimeSeconds())"
    }
    if ($ProbeMarker -notmatch '^[A-Za-z0-9_-]{8,120}$') {
        throw "ProbeMarker must be 8-120 ASCII letters, digits, underscores, or hyphens."
    }

    $beforeNewState = Invoke-ApkMcp -Tool "ui_state"
    $beforeNew = [long]$beforeNewState.last_command.observed_at_ms
    Invoke-UiAction -Action "chatgpt_new_conversation" | Out-Null
    $newState = Wait-CommandResult -Action "new_conversation" -AfterMs $beforeNew -TimeoutSec $ReadyTimeoutSec
    $newConversationAccepted = $newState.last_command.ok -eq $true
    Add-Check "new_conversation" $newConversationAccepted ([string]$newState.last_command.detail)
    if (-not $newConversationAccepted) {
        throw "ChatGPT Web new conversation failed; the send probe was not dispatched."
    }
    $blankState = Wait-NewConversationReady `
        -PreviousUrl ([string]$beforeNewState.conversation.url) `
        -PreviousMessageCount ([int]$beforeNewState.conversation.message_count) `
        -TimeoutSec $ReadyTimeoutSec
    Add-Check "new_conversation_ready" (
        [int]$blankState.conversation.message_count -eq 0
    ) "isolated blank conversation"

    $prompt = "Reply only with: $ProbeMarker"
    Invoke-UiAction -Action "set_input_text" -Arguments @{ text = $prompt } | Out-Null
    $beforeSendState = Invoke-ApkMcp -Tool "ui_state"
    $beforeSend = [long]$beforeSendState.last_command.observed_at_ms
    $sendDispatch = Invoke-UiAction -Action "send_input"
    $sendRequestId = [string]$sendDispatch.command_receipt.request_id
    if ([string]::IsNullOrWhiteSpace($sendRequestId)) {
        throw "send_input did not return a command receipt request_id."
    }
    $replyState = Wait-ChatGptProbeReply -RequestId $sendRequestId -Marker $ProbeMarker `
        -AfterMs $beforeSend -TimeoutSec $ReplyTimeoutSec -PollIntervalSec $PollIntervalSec `
        -InvokeUiState { Invoke-ApkMcp -Tool "ui_state" }
    $lastMessage = @($replyState.conversation.messages) | Select-Object -Last 1
    $normalizedReply = Normalize-ChatGptProbeReply ([string]$lastMessage.content)
    Add-Check "probe_reply" ($normalizedReply -like "*$ProbeMarker*") $ProbeMarker
    $probe = [ordered]@{
        marker = $ProbeMarker
        conversation_route_observed = -not [string]::IsNullOrWhiteSpace(
            [string]$replyState.conversation.url
        )
        model_observed = -not [string]::IsNullOrWhiteSpace(
            [string]$replyState.conversation.current_model
        )
        message_count = [int]$replyState.conversation.message_count
        private_content_emitted = $false
    }
}

$beforeListState = Invoke-ApkMcp -Tool "ui_state"
$beforeList = [long]$beforeListState.last_command.observed_at_ms
Invoke-UiAction -Action "chatgpt_list_conversations" | Out-Null
$listState = Wait-CommandResult -Action "list_conversations" -AfterMs $beforeList -TimeoutSec $ReadyTimeoutSec
Add-Check "conversation_list" ($listState.last_command.ok -eq $true) ([string]$listState.last_command.detail)
$conversationPage = Invoke-UiAction -Action "chatgpt_get_conversations" -Arguments @{ offset = 0; limit = 10 }
Add-Check "conversation_query" ($conversationPage.control_ok -eq $true) "returned=$(@($conversationPage.conversations).Count)"
$conversationCollection = $conversationPage.collection
Add-Check "conversation_collection_count" (
    [int]$conversationCollection.observed_count -ge [int]$conversationPage.source_count
) "observed=$($conversationCollection.observed_count),source=$($conversationPage.source_count)"
Add-Check "conversation_scroll_restored" (
    $conversationCollection.scroll_restored -eq $true
) "scrolled=$($conversationCollection.scrolled),steps=$($conversationCollection.steps)"
Add-Check "conversation_collection_timeout" (
    $conversationCollection.timed_out -ne $true
) "reached_end=$($conversationCollection.reached_end),truncated=$($conversationCollection.truncated)"

$nativeView = Invoke-UiAction -Action "chatgpt_select_view" -Arguments @{ view_mode = "native" }
Add-Check "native_view_selected" ($nativeView.control_ok -eq $true) ([string]$nativeView.view_mode)
$requiredSelectors = @(
    "chatgpt-native:conversation-list:",
    "chatgpt-native:feature-list:",
    "chatgpt-native:composer-input:",
    "chatgpt-native:composer-model:",
    "chatgpt-native:composer-tools:",
    "chatgpt-native:dictation:",
    "chatgpt-native:send:"
)
$visibleSelectors = Wait-VisibleNativeSelectors -RequiredPrefixes $requiredSelectors `
    -TimeoutSec $ReadyTimeoutSec
foreach ($prefix in $requiredSelectors) {
    $match = @($visibleSelectors | Where-Object { $_.StartsWith($prefix) })
    Add-Check "selector_$($prefix.Split(':')[1])" ($match.Count -gt 0) ($match -join ",")
}
$restoredOfficialView = Invoke-UiAction -Action "chatgpt_select_view" -Arguments @{ view_mode = "official" }
Add-Check "official_view_restored" (
    $restoredOfficialView.control_ok -eq $true
) ([string]$restoredOfficialView.view_mode)

$failed = @($checks | Where-Object { -not $_.passed })
$summary = [ordered]@{
    schema = "elon.chatgpt_web.apk_smoke.v2"
    recorded_at_utc = [DateTimeOffset]::UtcNow.ToString("o")
    passed = $failed.Count -eq 0
    mode = if ($SendProbe) { "send_probe" } else { "read_only" }
    device_serial = $DeviceSerial
    app = $matrix.app
    adapter_version = [int]$state.adapter_version
    authenticated = [bool]$state.authenticated
    composer_ready = [bool]$state.composer_ready
    visible_native_selector_count = $visibleSelectors.Count
    manifest = $matrix.manifest
    feature_baseline = $featureBaseline
    adaptation_review = $matrix.adaptation_review
    conversation_count = [int]$conversationPage.match_count
    conversation_collection = $conversationCollection
    context = [ordered]@{
        schema = [string]$contextFirst.schema
        revision = [string]$contextFirst.context_revision
        message_offset = [int]$contextFirst.message_offset
        has_more = [bool]$contextFirst.has_more
        cursor_roundtrip = $contextReplay.control_ok -eq $true
    }
    probe = $probe
    checks = $checks
}
$summary | ConvertTo-Json -Depth 30

if ($failed.Count -gt 0) {
    Write-Output "CHATGPT_WEB_SMOKE_STATUS=failed failed_count=$($failed.Count)"
    throw "ChatGPT Web smoke failed: $($failed.Count) check(s)."
}

Write-Output "CHATGPT_WEB_SMOKE_STATUS=passed mode=$($summary.mode)"
} finally {
    Stop-ChatGptWebSmokeAwakeLease -Runtime $smokeRuntime | Out-Null
}
