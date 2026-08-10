#requires -Version 5.1

[CmdletBinding()]
param(
    [string]$Adb = "D:\Android\sdk\platform-tools\adb.exe",
    [string]$DeviceSerial = "",
    [int]$ReadyTimeoutSec = 45,
    [int]$ReplyTimeoutSec = 90,
    [int]$PollIntervalSec = 3,
    [switch]$SendProbe,
    [string]$ProbeMarker = ""
)

$ErrorActionPreference = "Stop"

$invokeMcp = Join-Path $PSScriptRoot "invoke-apk-mcp.ps1"
if (-not (Test-Path -LiteralPath $invokeMcp -PathType Leaf)) {
    throw "Missing APK MCP helper: $invokeMcp"
}
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
        OpenAppOnFailure = $true
    }
    if ($EnsureMainActivity) { $params.EnsureMainActivity = $true }
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
    return ([string]$line).Trim()
}

function Get-VisibleNativeSelectors {
    $remotePath = "/sdcard/elon-chatgpt-web-smoke.xml"
    Invoke-Adb shell uiautomator dump $remotePath | Out-Null
    if ($LASTEXITCODE -ne 0) { throw "UIAutomator dump failed." }
    $xml = (Invoke-Adb shell cat $remotePath) -join "`n"
    if ($LASTEXITCODE -ne 0) { throw "Unable to read UIAutomator dump." }
    return @(
        [regex]::Matches($xml, 'content-desc="([^"]*chatgpt-native:[^"]*)"') |
            ForEach-Object { $_.Groups[1].Value } |
            Sort-Object -Unique
    )
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
        $cachedSnapshot = $command.action -eq "list_navigation" -and
            $command.ok -eq $true -and
            @($last.features).Count -gt 0
        if ($fresh -and ($collected -or $cachedSnapshot)) {
            return $last
        }
        Start-Sleep -Seconds $PollIntervalSec
    } while ([DateTimeOffset]::UtcNow -lt $deadline)
    throw "Timed out waiting for ChatGPT navigation readiness. Last action=$($last.last_command.action)."
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
    param([Parameter(Mandatory = $true)][ValidateSet("model", "tools")][string]$Section)

    $beforeState = Invoke-ApkMcp -Tool "ui_state"
    $afterMs = [long]$beforeState.last_command.observed_at_ms
    Invoke-UiAction -Action "chatgpt_list_composer_options" -Arguments @{ section = $Section } | Out-Null
    return Wait-ComposerOptionsReady -Section $Section -AfterMs $afterMs -TimeoutSec $ReadyTimeoutSec
}

function Get-ForeignComposerLabels {
    param([object[]]$Options)

    $foreignPattern = 'download\s+chatgpt|chatgpt\s+(desktop|mobile)|settings?|personalization|profile|log\s*out|sign\s*out|help|account|下载|桌面版|移动版|设置|个性化|个人资料|退出登录|帮助|账户|帐户|账号'
    return @($Options | Where-Object { [string]$_.label -match $foreignPattern } | ForEach-Object { [string]$_.label })
}

function Normalize-ProbeReply {
    param([AllowEmptyString()][string]$Text)

    return $Text.Replace('\_', '_').Replace('\-', '-').Trim()
}

function Wait-ProbeReply {
    param(
        [Parameter(Mandatory = $true)][string]$Marker,
        [Parameter(Mandatory = $true)][long]$AfterMs,
        [Parameter(Mandatory = $true)][int]$TimeoutSec
    )

    $deadline = [DateTimeOffset]::UtcNow.AddSeconds($TimeoutSec)
    $last = $null
    do {
        $last = Invoke-ApkMcp -Tool "ui_state"
        $messages = @($last.conversation.messages)
        $lastMessage = $messages | Select-Object -Last 1
        if (
            $last.last_command.action -eq "send_prompt" -and
            $last.last_command.ok -eq $true -and
            [long]$last.last_command.observed_at_ms -gt $AfterMs -and
            $last.streaming -eq $false -and
            $messages.Count -ge 2 -and
            [string]$lastMessage.role -eq "assistant" -and
            (Normalize-ProbeReply ([string]$lastMessage.content)) -like "*$Marker*"
        ) {
            return $last
        }
        Start-Sleep -Seconds $PollIntervalSec
    } while ([DateTimeOffset]::UtcNow -lt $deadline)
    throw "Timed out waiting for ChatGPT Web probe reply. Last action=$($last.last_command.action)."
}

$opened = Invoke-UiAction -Action "open_chatgpt_web" -EnsureMainActivity
$state = Wait-ChatGptState -TimeoutSec $ReadyTimeoutSec -Description "ChatGPT Web readiness" -Predicate {
    param($value)
    $value.surface -eq "chatgpt_web" -and
        $value.bridge_state -eq "ready" -and
        $value.activity_bound -eq $true
}
$topResumedActivity = Get-TopResumedActivity

Add-Check "open_chatgpt_web" ($opened.control_ok -eq $true) ([string]$opened.action)
Add-Check "chatgpt_activity_foreground" (
    $topResumedActivity -match 'com\.elon\.app/\.chatgptweb\.ChatGptWebTestActivity\b'
) $topResumedActivity
Add-Check "chatgpt_surface" ($state.surface -eq "chatgpt_web") ([string]$state.surface)
Add-Check "bridge_ready" ($state.bridge_state -eq "ready") ([string]$state.bridge_state)
Add-Check "authenticated" ($state.authenticated -eq $true) ([string]$state.authenticated)
Add-Check "composer_ready" ($state.composer_ready -eq $true) ([string]$state.composer_ready)
Add-Check "adapter_version" ([int]$state.adapter_version -ge 6) ([string]$state.adapter_version)

$matrix = Invoke-UiAction -Action "chatgpt_get_capability_matrix"
$blockingGaps = @($matrix.blocking_gaps)
$unknownCapabilities = @($matrix.unknown_capabilities)
$unknownSemantics = @($matrix.unknown_semantics)
Add-Check "capability_matrix_ready" ($matrix.ready_for_mcp -eq $true) ([string]$matrix.schema)
Add-Check "blocking_gaps" ($blockingGaps.Count -eq 0) ($blockingGaps -join ",")
Add-Check "unknown_capabilities" ($unknownCapabilities.Count -eq 0) ($unknownCapabilities -join ",")
Add-Check "unknown_semantics" ($unknownSemantics.Count -eq 0) ($unknownSemantics -join ",")

$requiredSelectors = @(
    "chatgpt-native:conversation-list:",
    "chatgpt-native:feature-list:",
    "chatgpt-native:composer-input:",
    "chatgpt-native:composer-model:",
    "chatgpt-native:composer-tools:",
    "chatgpt-native:dictation:",
    "chatgpt-native:send:"
)
$visibleSelectors = Get-VisibleNativeSelectors
foreach ($prefix in $requiredSelectors) {
    $match = @($visibleSelectors | Where-Object { $_.StartsWith($prefix) })
    Add-Check "selector_$($prefix.Split(':')[1])" ($match.Count -gt 0) ($match -join ",")
}

$beforeFeaturesState = Invoke-ApkMcp -Tool "ui_state"
$beforeFeatures = [long]$beforeFeaturesState.last_command.observed_at_ms
Invoke-UiAction -Action "chatgpt_list_features" | Out-Null
$featuresState = Wait-NavigationReady -AfterMs $beforeFeatures -TimeoutSec $ReadyTimeoutSec
Add-Check "composer_contamination_setup" ($featuresState.last_command.ok -eq $true) "official sidebar opened"

$modelResult = Get-ComposerOptions -Section "model"
$modelOptions = @($modelResult.options)
$modelLabels = @($modelOptions | ForEach-Object { [string]$_.label })
$foreignModelLabels = Get-ForeignComposerLabels -Options $modelOptions
Add-Check "composer_model_options" ($modelOptions.Count -gt 0) ($modelLabels -join ",")
Add-Check "composer_model_scope" ($foreignModelLabels.Count -eq 0) ($foreignModelLabels -join ",")

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
    Add-Check "new_conversation" ($newState.last_command.ok -eq $true) ([string]$newState.last_command.detail)

    $prompt = "Reply only with: $ProbeMarker"
    Invoke-UiAction -Action "set_input_text" -Arguments @{ text = $prompt } | Out-Null
    $beforeSendState = Invoke-ApkMcp -Tool "ui_state"
    $beforeSend = [long]$beforeSendState.last_command.observed_at_ms
    Invoke-UiAction -Action "send_input" | Out-Null
    $replyState = Wait-ProbeReply -Marker $ProbeMarker -AfterMs $beforeSend -TimeoutSec $ReplyTimeoutSec
    $lastMessage = @($replyState.conversation.messages) | Select-Object -Last 1
    $normalizedReply = Normalize-ProbeReply ([string]$lastMessage.content)
    Add-Check "probe_reply" ($normalizedReply -like "*$ProbeMarker*") $ProbeMarker
    $probe = [ordered]@{
        marker = $ProbeMarker
        conversation_url = [string]$replyState.conversation.url
        model = [string]$replyState.conversation.current_model
        message_count = [int]$replyState.conversation.message_count
    }
}

$beforeListState = Invoke-ApkMcp -Tool "ui_state"
$beforeList = [long]$beforeListState.last_command.observed_at_ms
Invoke-UiAction -Action "chatgpt_list_conversations" | Out-Null
$listState = Wait-CommandResult -Action "list_conversations" -AfterMs $beforeList -TimeoutSec $ReadyTimeoutSec
Add-Check "conversation_list" ($listState.last_command.ok -eq $true) ([string]$listState.last_command.detail)
$conversationPage = Invoke-UiAction -Action "chatgpt_get_conversations" -Arguments @{ offset = 0; limit = 10 }
Add-Check "conversation_query" ($conversationPage.control_ok -eq $true) "returned=$(@($conversationPage.conversations).Count)"

$failed = @($checks | Where-Object { -not $_.passed })
$summary = [ordered]@{
    schema = "elon.chatgpt_web.apk_smoke.v1"
    passed = $failed.Count -eq 0
    mode = if ($SendProbe) { "send_probe" } else { "read_only" }
    device_serial = $DeviceSerial
    adapter_version = [int]$state.adapter_version
    authenticated = [bool]$state.authenticated
    composer_ready = [bool]$state.composer_ready
    visible_native_selector_count = $visibleSelectors.Count
    manifest = $matrix.manifest
    adaptation_review = $matrix.adaptation_review
    conversation_count = [int]$conversationPage.match_count
    probe = $probe
    checks = $checks
}
$summary | ConvertTo-Json -Depth 30

if ($failed.Count -gt 0) {
    Write-Output "CHATGPT_WEB_SMOKE_STATUS=failed failed_count=$($failed.Count)"
    exit 1
}

Write-Output "CHATGPT_WEB_SMOKE_STATUS=passed mode=$($summary.mode)"
