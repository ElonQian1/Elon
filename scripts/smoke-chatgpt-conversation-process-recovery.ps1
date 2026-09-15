#requires -Version 7.0
[CmdletBinding()]
param(
    [string]$Adb = 'D:/Android/sdk/platform-tools/adb.exe',
    [Parameter(Mandatory)][string]$DeviceSerial,
    [Parameter(Mandatory)][string]$ExpectedHardwareSerial,
    [Parameter(Mandatory)][ValidateRange(1,9999)][int]$ExpectedAdapterVersion,
    [ValidateRange(15,120)][int]$RestoreTimeoutSeconds = 90
)
$ErrorActionPreference = 'Stop'
. (Join-Path $PSScriptRoot 'chatgpt-web-smoke-runtime.ps1')
. (Join-Path $PSScriptRoot 'chatgpt-fresh-trial-smoke.ps1')
. (Join-Path $PSScriptRoot 'chatgpt-fresh-project-smoke.ps1')
. (Join-Path $PSScriptRoot 'chatgpt-conversation-recovery-evidence.ps1')
$r = New-ChatGptWebSmokeRuntime -Adb $Adb -DeviceSerial $DeviceSerial -ExpectedHardwareSerial $ExpectedHardwareSerial
Assert-ChatGptWebSmokeTrustedDevice -Runtime $r
$common = [IO.Path]::GetFullPath((& git -C (Split-Path -Parent $PSScriptRoot) rev-parse --git-common-dir).Trim())
$ledgers = Join-Path $common 'ai-acceptance-fixtures'
if (Test-Path -LiteralPath (Join-Path $ledgers 'fresh-text-pending.json')) { throw 'pending_write_ledger_requires_resolution' }
$file = Get-ChildItem -LiteralPath $ledgers -Filter 'fresh-project-fallback-resolved-*.json' |
    Sort-Object LastWriteTime -Descending | Select-Object -First 1
if (!$file) { throw 'owned_project_fixture_missing' }
$fixture = Get-Content -LiteralPath $file.FullName -Raw | ConvertFrom-Json
if (!(Test-ChatGptFreshProjectPath $fixture.expected_path $fixture.project_id) -or
    $fixture.readback_completed -cne $true -or $fixture.replay_allowed -cne $false) { throw 'owned_fixture_ledger_invalid' }
$origin = ''; $restored = $false; $changed = $false
$report = [ordered]@{schema='elon.conversation_process_recovery.v2';passed=$false;
    adapter=$ExpectedAdapterVersion;send_clicks=0;process_stopped=$false;process_recreated=$false;
    automatic_route_restored=$false;native_body_equal=$false;web_body_equal=$false}
try {
    Start-ChatGptWebSmokeAwakeLease -Runtime $r | Out-Null
    Open-WebChatNativeChatSurface -Runtime $r -ProviderId chatgpt_web -TimeoutSec 30 | Out-Null
    $m = Invoke-ChatGptWebSmokeMcp -Runtime $r -Tool ui_state -MainState
    $w = $m.chatgpt_web_mcp
    $origin = [string]$m.social_chat.web_chat_conversation_path
    if ([string]::IsNullOrWhiteSpace($origin)) { throw 'origin_route_unavailable' }
    if ($m.input.has_text -cne $false -or $w.input.text -cne '' -or $w.streaming -cne $false -or
        $w.dictation_active -cne $false -or $w.private_voice_native_research.phase -cne 'idle') { throw 'origin_not_idle' }
    $t = Invoke-ChatGptFreshTrial -Runtime $r -Mode state
    if ($t.pending -cne $false -or $t.armed -cne $false) { throw 'origin_has_pending_write' }
    $changed = $true
    $a = Invoke-ChatGptWebSmokeAction -Runtime $r -Action open_web_chat_conversation -Arguments @{conversation_path=$fixture.expected_path}
    if ($a.control_ok -cne $true) { throw 'fixture_navigation_failed' }
    $until = [DateTimeOffset]::UtcNow.AddSeconds(30)
    $before = $null; $stable = 0
    do {
        $m = Invoke-ChatGptWebSmokeMcp -Runtime $r -Tool ui_state -MainState
        $w = $m.chatgpt_web_mcp
        $candidate = Get-ChatGptConversationRecoveryEvidence $m $w $fixture.expected_path $ExpectedAdapterVersion
        if ($candidate -and (Test-ChatGptFreshProjectFixtureUsers @($m.social_chat.messages)) -and
            (Test-ChatGptFreshProjectFixtureUsers @($w.conversation.messages))) {
            $stable = if (Test-ChatGptConversationRecoveryMatch $before $candidate) { $stable + 1 } else { 1 }
            $before = $candidate
            if ($stable -ge 3) { break }
        } else { $before = $null; $stable = 0 }
        Start-Sleep -Milliseconds 500
    } while ([DateTimeOffset]::UtcNow -lt $until)
    if (!$before -or $stable -lt 3) {
        $report.baseline_diagnostic = Get-ChatGptConversationRecoveryDiagnostic $m $fixture.expected_path
        $report.baseline_native_count = $report.baseline_diagnostic.native_message_count
        $report.baseline_web_count = $report.baseline_diagnostic.web_message_count
        $report.baseline_window_start = $w.conversation.message_window_start
        $report.baseline_observed_count = $w.conversation.message_count
        $report.baseline_available_count = $w.conversation.available_message_count
        $report.baseline_context_complete = $w.conversation.context_complete
        $report.baseline_export_truncated = $w.conversation.messages_truncated
        $report.baseline_evidence_valid = $null -ne $candidate
        $report.baseline_owned_users = (Test-ChatGptFreshProjectFixtureUsers @($m.social_chat.messages)) -and
            (Test-ChatGptFreshProjectFixtureUsers @($w.conversation.messages))
        throw 'fixture_baseline_unconfirmed'
    }
    $t = Invoke-ChatGptFreshTrial -Runtime $r -Mode state
    if ($t.pending -cne $false -or $t.armed -cne $false) { throw 'fixture_has_pending_write' }
    if (!(Test-WebChatNativeChatSurfaceForeground -Runtime $r)) { throw 'foreground_package_mismatch' }
    $pidBefore = (Invoke-ChatGptWebSmokeAdb -Runtime $r -Arguments @('shell','pidof','com.elon.app')).Trim()
    if ($pidBefore -cnotmatch '^[0-9]+$') { throw 'process_identity_missing' }
    # Force-stop only this idle app, never its data or the independently owned VPN.
    Invoke-ChatGptWebSmokeAdb -Runtime $r -Arguments @('shell','am','force-stop','com.elon.app') | Out-Null
    $processes = Invoke-ChatGptWebSmokeAdb -Runtime $r -Arguments @('shell','ps','-A','-o','NAME')
    if (($processes -split '\r?\n') -ccontains 'com.elon.app') { throw 'process_stop_unconfirmed' }
    $report.process_stopped = $true
    $r.mcp_bootstrapped = $false
    # Open the product surface only. Do not navigate to the expected path to manufacture a pass.
    $started = [DateTimeOffset]::UtcNow
    Open-WebChatNativeChatSurface -Runtime $r -ProviderId chatgpt_web -TimeoutSec 60 | Out-Null
    $pidAfter = (Invoke-ChatGptWebSmokeAdb -Runtime $r -Arguments @('shell','pidof','com.elon.app')).Trim()
    if ($pidAfter -cnotmatch '^[0-9]+$' -or $pidBefore -ceq $pidAfter) { throw 'process_recreation_unconfirmed' }
    $report.process_recreated = $true
    $until = $started.AddSeconds($RestoreTimeoutSeconds)
    do {
        $m = Invoke-ChatGptWebSmokeMcp -Runtime $r -Tool ui_state -MainState
        $w = $m.chatgpt_web_mcp
        $after = Get-ChatGptConversationRecoveryEvidence $m $w $fixture.expected_path $ExpectedAdapterVersion
        if (Test-ChatGptConversationRecoveryMatch $before $after) { break }
        # Once the opened chat surface is lost, waiting for its body mislabels navigation as data loss.
        if ($m.active_surface -and $m.active_surface -cne 'social_ai') { break }
        Start-Sleep -Milliseconds 700
    } while ([DateTimeOffset]::UtcNow -lt $until)
    $report.native_route_equal = $m.social_chat.web_chat_conversation_path -ceq $fixture.expected_path
    $report.web_route_equal = $w.conversation.url -ceq ('https://chatgpt.com' + $fixture.expected_path)
    $report.recovery_evidence_valid = $null -ne $after
    $report.native_body_equal = $after -and $before.native_digest -ceq $after.native_digest
    $report.web_body_equal = $after -and $before.web_digest -ceq $after.web_digest
    $report.native_before_count = $before.native_count
    $report.web_before_count = $before.web_count
    $report.recovery_diagnostic = Get-ChatGptConversationRecoveryDiagnostic $m $fixture.expected_path
    $report.native_after_count = $report.recovery_diagnostic.native_message_count
    $report.web_after_count = $report.recovery_diagnostic.web_message_count
    $report.web_window_start = $w.conversation.message_window_start
    $report.web_observed_count = $w.conversation.message_count
    $report.web_available_count = $w.conversation.available_message_count
    $report.web_context_complete = $w.conversation.context_complete
    $report.web_export_truncated = $w.conversation.messages_truncated
    if (!(Test-ChatGptConversationRecoveryMatch $before $after)) {
        switch ($report.recovery_diagnostic.failure_layer) {
            'native_state' { throw 'restored_native_state_unavailable' }
            'native_surface' { throw 'restored_native_surface_changed' }
            'web_projection' { throw 'restored_web_projection_unavailable' }
            'conversation_route' { throw 'restored_conversation_route_mismatch' }
            default { throw 'restored_conversation_body_mismatch' }
        }
    }
    $t = Invoke-ChatGptFreshTrial -Runtime $r -Mode state
    if ($t.pending -cne $false -or $t.armed -cne $false -or $t.attempts -ne 0) { throw 'unexpected_send_after_restart' }
    $report.automatic_route_restored = $true; $report.native_body_equal = $true; $report.web_body_equal = $true
    $report.native_count = $after.native_count; $report.web_count = $after.web_count
    $report.restore_ms = [long]([DateTimeOffset]::UtcNow - $started).TotalMilliseconds
    $report.passed = $true
} catch {
    $report.error = if ($_.Exception.Message -cmatch '^[a-z_]+$') { $_.Exception.Message } else { 'process_recovery_acceptance_failed' }
    $report.failure_line = $_.InvocationInfo.ScriptLineNumber
} finally {
    try {
        if (!$changed) { $restored = $true }
        elseif ($origin) { $restored = Restore-WebChatNativeConversation -Runtime $r -ProviderId chatgpt_web -ConversationPath $origin -TimeoutSec 30 }
    } catch { $restored = $false }
    $report.restored = $restored
    $report.awake_restored = Stop-ChatGptWebSmokeAwakeLease -Runtime $r
    $report | ConvertTo-Json -Compress
}
if (!$report.passed -or !$restored -or !$report.awake_restored) { throw 'process_recovery_acceptance_incomplete' }
