#requires -Version 7.0
[CmdletBinding()]
param(
    [Parameter(Mandatory)][string]$DeviceSerial,
    [Parameter(Mandatory)][string]$ExpectedHardwareSerial,
    [ValidateRange(1,9999)][int]$ExpectedAdapterVersion = 357,
    [string]$Adb = 'D:/Android/sdk/platform-tools/adb.exe'
)
$ErrorActionPreference = 'Stop'
. (Join-Path $PSScriptRoot 'chatgpt-web-smoke-runtime.ps1')
. (Join-Path $PSScriptRoot 'chatgpt-web-smoke-evidence.ps1')
. (Join-Path $PSScriptRoot 'invoke-android-semantic-acceptance.ps1')
$r = New-ChatGptWebSmokeRuntime -Adb $Adb -DeviceSerial $DeviceSerial -ExpectedHardwareSerial $ExpectedHardwareSerial
$report = [ordered]@{schema='elon.instant_model_native.v1';passed=$false;stage='prepare';native_slider=$false;instant_confirmed=$false
    error=$null;cleanup_failed=$false
    model_restored=$false;conversation_unchanged=$false;draft_unchanged=$false;awake_restored=$false;sent_messages=0;content_exported=$false}
$before=$null; $original=$null; $changed=$false
function Web { Invoke-ChatGptWebSmokeMcp -Runtime $r -Tool ui_state }
function Ui([string]$Step, [hashtable]$Parameters=@{}) {
    Invoke-AndroidSemanticAcceptance -Runtime $r -TestClass ConversationUiAcceptance -Step $Step -Parameters $Parameters -ResultPrefix CONVERSATION_UI_RESULT
}
function Command([string]$Action, [hashtable]$Arguments, [string]$Expected) {
    $sent=Invoke-ChatGptWebSmokeAction -Runtime $r -Action $Action -Arguments $Arguments
    $done=Wait-ChatGptCommandReceipt -InvokeUiState { Web } -RequestId $sent.command_receipt.request_id -ExpectedAction $Expected -TimeoutSec 25 -PollIntervalSec 1
    return $done.receipt
}
function Models {
    Command 'chatgpt_list_composer_options' @{section='model'} 'list_model_options' | Out-Null
    $nav=Invoke-ChatGptWebSmokeAction -Runtime $r -Action chatgpt_get_navigation -Arguments @{section='model'}
    $all=@($nav.composer_sections.model)
    if (-not $all.Count -or @($all | Where-Object { -not $_.id.StartsWith('private_model_') }).Count) { throw 'private_catalog_missing' }
    return @($all | Where-Object { -not $_.opens_submenu })
}
function Select-NativeLevel([int]$Index) {
    $visible=Ui 'inspect'
    if (-not $visible.model_menu) { $visible=Ui 'model' }
    if (-not $visible.level_slider) { throw 'native_slider_missing' }
    $since=[long](Web).last_command.observed_at_ms
    Ui 'model_level' @{level=[string]$Index} | Out-Null
    $deadline=[DateTimeOffset]::UtcNow.AddSeconds(25)
    do {
        # Native model gestures dispatch directly; they do not allocate an MCP request ID.
        $receipt=(Web).last_command
        if ($receipt.action -eq 'select_model_option' -and [long]$receipt.observed_at_ms -gt $since) {
            if ($receipt.ok -ne $true) { throw 'native_selection_unconfirmed' }
            return
        }
        Start-Sleep -Milliseconds 500
    } while ([DateTimeOffset]::UtcNow -lt $deadline)
    throw 'native_selection_timeout'
}
function Index-OfLabel([object[]]$Choices, [string]$Label) {
    $indexes=@(0..($Choices.Count-1) | Where-Object { $Choices[$_].label -ceq $Label })
    if ($indexes.Count -ne 1) { throw 'model_label_ambiguous' }
    return [int]$indexes[0]
}
try {
    Assert-ChatGptWebSmokeTrustedDevice -Runtime $r
    if (-not (Get-ChatGptWebSmokeUserReadiness -Runtime $r).ready) { throw 'device_locked' }
    $main=Get-ChatGptWebNativeChatState -Runtime $r
    $before=Web
    if ($main.active_surface -ne 'social_ai' -or $main.social_chat.web_chat_provider_id -ne 'chatgpt_web' -or
        -not $before.authenticated -or $before.streaming -or $before.dictation_active -or $main.input.text -or
        [int]$before.input.official_draft_length -gt 0) { throw 'idle_native_chat_required' }
    Assert-ChatGptWebSmokeAdapterVersion -State $before -ExpectedAdapterVersion $ExpectedAdapterVersion
    Start-ChatGptWebSmokeAwakeLease -Runtime $r | Out-Null
    $choices=Models
    $selected=@($choices | Where-Object selected)
    $instant=@($choices | Where-Object { $_.label -match '^(Instant|\u5373\u65f6)$' })
    if ($selected.Count -ne 1 -or $instant.Count -ne 1 -or $instant[0].selected) { throw 'model_choices_unconfirmed' }
    $original=$selected[0].label
    $report.stage='select_instant'; $changed=$true
    Write-Host 'INSTANT_MODEL_STAGE=select_instant'
    Select-NativeLevel (Index-OfLabel $choices $instant[0].label)
    $report.native_slider=$true
    Start-Sleep -Milliseconds 800
    $current=Models
    if (@($current | Where-Object { $_.selected -and $_.label -ceq $instant[0].label }).Count -ne 1) { throw 'instant_selection_not_observed' }
    $report.instant_confirmed=$true
    $context=Command 'chatgpt_private_protocol_probe' @{mode='model_runtime_context'} 'private_protocol_probe'
    $report.model_context=$context.result.detail
    if ($report.model_context -ne 'model_runtime_context:ready') { throw 'model_context_not_ready' }
    $report.stage='restore'
} catch {
    $message=[string]$_.Exception.Message
    $report.error=if ($message -match '^[a-z_]+$') { $message } else { 'acceptance_failed' }
} finally {
    try {
        if ($changed -and $original) {
            Write-Host 'INSTANT_MODEL_STAGE=restore'
            $current=Models
            $index=Index-OfLabel $current $original
            if (-not $current[$index].selected) { Select-NativeLevel $index }
            Start-Sleep -Milliseconds 800
            $confirmed=Models
            $report.model_restored=@($confirmed | Where-Object { $_.selected -and $_.label -ceq $original }).Count -eq 1
        }
        if ((Ui 'inspect').model_menu) { Ui 'back' | Out-Null }
        Command 'chatgpt_dismiss_composer_options' @{} 'dismiss_composer_menu' | Out-Null
    } catch { $report.cleanup_failed=$true }
    try {
        $after=Web
        $report.conversation_unchanged=$before -and $after.conversation.url -eq $before.conversation.url
        $report.draft_unchanged=$before -and $after.input.official_draft_length -eq $before.input.official_draft_length
        $report.awake_restored=Stop-ChatGptWebSmokeAwakeLease -Runtime $r
    } catch { $report.cleanup_failed=$true }
    $report.passed=$report.native_slider -and $report.instant_confirmed -and $report.model_restored -and
        $report.conversation_unchanged -and $report.draft_unchanged -and $report.awake_restored -and
        -not $report.error -and -not $report.cleanup_failed
    if ($report.passed) { $report.stage='complete' }
    $report | ConvertTo-Json -Depth 5 -Compress
}
if (-not $report.passed) { throw 'instant_model_native_acceptance_failed' }
