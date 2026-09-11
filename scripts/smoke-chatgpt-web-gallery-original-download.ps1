#requires -Version 7.0
[CmdletBinding()]
param(
    [Parameter(Mandatory)][string]$DeviceSerial,
    [Parameter(Mandatory)][string]$ExpectedHardwareSerial,
    [string]$Adb = 'D:/Android/sdk/platform-tools/adb.exe'
)
$ErrorActionPreference = 'Stop'
. (Join-Path $PSScriptRoot 'chatgpt-web-smoke-runtime.ps1')
. (Join-Path $PSScriptRoot 'chatgpt-web-smoke-evidence.ps1')
. (Join-Path $PSScriptRoot 'invoke-android-semantic-acceptance.ps1')
$r = New-ChatGptWebSmokeRuntime -Adb $Adb -DeviceSerial $DeviceSerial -ExpectedHardwareSerial $ExpectedHardwareSerial
$report = [ordered]@{schema='elon.chatgpt.gallery_original_ui.v1';passed=$false;stage='prepare'
    restored=$false;awake_restored=$false;content_exported=$false;download_attempts=0}
$origin=$null; $gallery=$false; $preview=$false; $download=$false; $probe=$false; $sidebar=$false
function Native { Get-ChatGptWebNativeChatState -Runtime $r }
function Web {
    if (-not (Test-WebChatNativeChatSurfaceForeground -Runtime $r)) {throw 'foreground_changed'}
    Invoke-ChatGptWebSmokeMcp -Runtime $r -Tool ui_state
}
function Act([string]$Action, [hashtable]$Arguments=@{}) {
    $result=Invoke-ChatGptWebSmokeAction -Runtime $r -Action $Action -Arguments $Arguments
    if ($result.control_ok -eq $false) {throw 'native_action_rejected'}
    $result
}
function Command([string]$Action, [hashtable]$Arguments, [string]$Expected) {
    $sent=Act $Action $Arguments
    Wait-ChatGptCommandReceipt -InvokeUiState { Web } -RequestId $sent.command_receipt.request_id `
        -ExpectedAction $Expected -TimeoutSec 25 -PollIntervalSec 1
}
function Ui([string]$Step) {
    $report.ui_step=$Step
    Invoke-AndroidSemanticAcceptance -Runtime $r -TestClass LibraryUiAcceptance -Step $Step -ResultPrefix LIBRARY_UI_RESULT
}
try {
    Assert-ChatGptWebSmokeTrustedDevice -Runtime $r
    if (-not (Get-ChatGptWebSmokeUserReadiness -Runtime $r).ready) {throw 'device_locked'}
    $origin=Native; $before=Web
    if ($origin.active_surface -ne 'social_ai' -or $origin.social_chat.web_chat_provider_id -ne 'chatgpt_web' -or
        -not $before.authenticated -or -not $before.adapter_current -or [int]$before.adapter_version -lt 336 -or
        -not $origin.social_chat.web_chat_conversation_path) {throw 'surface_not_ready'}
    if ($origin.input.text -or $before.streaming -or $before.dictation_active -or
        [int]$before.input.official_draft_length -gt 0 -or $before.file_download.can_cancel -or
        [int]$origin.social_chat.web_chat_pending_attachment_count -gt 0) {throw 'existing_work_in_progress'}
    Start-ChatGptWebSmokeAwakeLease -Runtime $r | Out-Null
    $report.adapter=$before.adapter_version
    $report.stage='native_gallery'
    Act 'open_chat_side_menu' | Out-Null
    $sidebar=$true
    Ui 'gallery' | Out-Null
    $gallery=$true
    $page=Ui 'gallery_wait'
    if ($page.empty -or $page.failed -or (-not $page.ready -and -not $page.partial)) {throw 'gallery_not_ready'}
    Ui 'gallery_preview' | Out-Null
    $preview=$true
    Command 'chatgpt_private_protocol_probe' @{mode='start'} 'private_protocol_probe' | Out-Null
    $probe=$true
    $idsBefore=@((Web).command_requests | ForEach-Object request_id)
    $report.stage='native_original_download'
    $report.download_attempts=1; $download=$true
    $report.storage=Ui 'gallery_download_verified'
    $w=Web
    $receipts=@($w.command_requests | Where-Object {
        $_.request_id -notin $idsBefore -and $_.expected_web_action -eq 'download_conversation_file'
    })
    if ($receipts.Count -ne 1 -or $receipts[0].status -ne 'succeeded' -or
        $receipts[0].result.detail -notin @('download_saved','download_queued') -or -not $report.storage.saved) {
        throw 'download_save_unconfirmed'
    }
    $report.download_detail=$receipts[0].result.detail
    $report.stage='complete'; $report.passed=$true
} catch {
    $message=$_.Exception.Message
    $report.error=if ($message -cmatch '^[a-z_]+$') {$message}
        elseif ($message -match '^Semantic UI acceptance failed: ([a-z_]+)$') {$Matches[1]} else {'gallery_acceptance_failed'}
    if ($report.download_attempts -gt 0) {
        try {
            $failed=@((Web).command_requests | Where-Object {
                $_.request_id -notin $idsBefore -and $_.expected_web_action -eq 'download_conversation_file'
            }) | Select-Object -Last 1
            $report.download_status=$failed.status
            $report.download_detail=if ($failed.result.detail -cmatch '^[a-z_]+$') {$failed.result.detail} else {'other'}
        } catch {$report.download_receipt_unavailable=$true}
    }
} finally {
    if ($probe) {
        try {
            $stopped=Command 'chatgpt_private_protocol_probe' @{mode='stop'} 'private_protocol_probe'
            $evidence=$stopped.receipt.result.detail | ConvertFrom-Json
            $report.protocol=@($evidence.records | Select-Object method,path,status,responseState,responseFields)
            $report.protocol_dropped=$evidence.dropped
        } catch {$report.protocol_unavailable=$true}
    }
    try {
        if ($download) {Ui 'close_download' | Out-Null}
        if ($preview) {Ui 'gallery_close_preview' | Out-Null}
        if ($gallery) {Ui 'gallery_close' | Out-Null}
        if ($sidebar) {Act 'close_chat_side_menu' | Out-Null}
        if ($origin) {
            $s=Native; $w=Web
            $report.restored=$s.social_chat.web_chat_conversation_path -eq $origin.social_chat.web_chat_conversation_path -and
                $s.input.text -ceq $origin.input.text -and @($s.social_chat.messages).Count -eq @($origin.social_chat.messages).Count -and
                [int]$w.input.official_draft_length -eq 0
        }
    } catch {$report.restored=$false}
    try {$report.awake_restored=Stop-ChatGptWebSmokeAwakeLease -Runtime $r} catch {$report.awake_restored=$false}
    $report.passed=$report.passed -and $report.restored -and $report.awake_restored
    $report | ConvertTo-Json -Depth 6 -Compress
}
if (-not $report.passed) {exit 1}
