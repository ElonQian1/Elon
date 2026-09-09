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
$runtime = New-ChatGptWebSmokeRuntime -Adb $Adb -DeviceSerial $DeviceSerial `
    -ExpectedHardwareSerial $ExpectedHardwareSerial -PollIntervalSec 1
$report = [ordered]@{ schema = 'elon.chatgpt.temporary_attachment.v1'; passed = $false
    stage = 'prepare'; restored = $false; awake_restored = $false; private_content_emitted = $false }
$changed = $false; $staged = $false
$fixtureId = 'fixed_ascii_text_v1'
$fixtureName = 'elon-chatgpt-attachment-fixture-v1.txt'
$prompt = 'Read the attached test file. Reply with TEMP-FILE-OK on the first line and copy the file first line on the second line. If unavailable, say so instead of guessing.'

function Native {
    if (-not (Test-WebChatNativeChatSurfaceForeground -Runtime $runtime)) { throw 'foreground_changed' }
    return Get-ChatGptWebNativeChatState -Runtime $runtime
}
function Web { return Invoke-ChatGptWebSmokeMcp -Runtime $runtime -Tool ui_state }
function Act([string]$Action, [hashtable]$Arguments = @{}) {
    $payload = @{} + $Arguments; $payload.action = $Action
    # Never automatically retry an upload, send or privacy-state transaction.
    $raw = @(& $runtime.invoke_mcp -Adb $runtime.adb -DeviceSerial $DeviceSerial -Tool ui_control `
        -Arguments ($payload | ConvertTo-Json -Depth 8 -Compress) -NoBootstrap `
        -HealthTimeoutSec 5 -RequestTimeoutSec 30 -AdbTimeoutSec 8) | Select-Object -Last 1
    if ($raw.result.isError -or $raw.result.structuredContent.control_ok -ne $true) { throw 'action_unconfirmed' }
    return $raw.result.structuredContent
}
function Receipt($Action, [string]$Expected) {
    return Wait-ChatGptCommandReceipt -RequestId $Action.command_receipt.request_id -ExpectedAction $Expected `
        -TimeoutSec 25 -PollIntervalSec 1 -InvokeUiState { Web }
}
function Library {
    $action = Act 'chatgpt_list_library_files' @{ operation = 'refresh'; query = $fixtureName }
    $result = Receipt $action 'list_library_files'
    $page = $result.state.library_files
    if ($page.request_id -ne $action.command_receipt.request_id -or $page.stale -or $page.partial -or $page.has_more) {
        throw 'fixture_library_inventory_incomplete'
    }
    return @($page.items | Where-Object { $_.name -ceq $fixtureName }).Count
}
function Wait-Mode([bool]$Selected, [Nullable[bool]]$StateSettable = $null) {
    $deadline = [DateTimeOffset]::UtcNow.AddSeconds(30)
    do {
        $s = Web
        $control = $s.ui_manifest.controls | Where-Object semantic -eq 'temporary_chat' | Select-Object -First 1
        if ($null -ne $control -and $control.selected -eq $Selected -and $s.composer_ready -eq $true -and
            ($null -eq $StateSettable -or $control.state_settable -eq $StateSettable)) { return $control }
        Start-Sleep -Milliseconds 600
    } while ([DateTimeOffset]::UtcNow -lt $deadline)
    $report.mode_observation = [ordered]@{ selected = $control.selected; state_settable = $control.state_settable; controls_present = $null -ne $control
        composer_ready = $s.composer_ready; route_home = ([uri]$s.conversation.url).AbsolutePath -eq '/' }
    throw 'temporary_mode_not_confirmed'
}

try {
    Assert-ChatGptWebSmokeTrustedDevice -Runtime $runtime
    if ((Get-ChatGptWebSmokeUserReadiness -Runtime $runtime).ready -ne $true) { throw 'device_locked' }
    $origin = Native; $web = Web
    if ($origin.active_surface -ne 'social_ai' -or $origin.social_chat.web_chat_provider_id -ne 'chatgpt_web' -or
        $origin.social_chat.interaction_mode -ne 'chat' -or $web.authenticated -ne $true) { throw 'surface_not_ready' }
    if (@($origin.social_chat.messages).Count -gt 0 -or $origin.input.text -or $web.streaming -or $web.dictation_active -or
        [int]$web.input.official_draft_length -gt 0 -or [int]$origin.social_chat.web_chat_pending_attachment_count -gt 0 -or
        [int]$origin.chatgpt_web_acceptance_attachment.composer_pending_count -gt 0) {
        throw 'blank_idle_chat_required'
    }
    Start-ChatGptWebSmokeAwakeLease -Runtime $runtime | Out-Null
    $control = Wait-Mode $false
    $report.stage = 'library_before'
    $report.library_before_count = Library
    $report.stage = 'enter_temporary'
    $changed = $true
    $result = Receipt (Act 'chatgpt_set_control_selected' @{control_id=$control.control_id; selected=$true}) 'set_ui_control_selected'
    $report.mode_receipt = $result.receipt.result.detail
    # This toggle's current receipt does not distinguish runtime from compatibility.
    $report.mode_transport = 'unattributed'
    Wait-Mode $true | Out-Null
    $report.stage = 'stage_fixture'
    Act 'stage_chatgpt_web_acceptance_attachment' @{ fixture_id = $fixtureId } | Out-Null
    $staged = $true
    $s = Native
    if ($s.chatgpt_web_acceptance_attachment.fixture_staged -ne $true -or
        [int]$s.chatgpt_web_acceptance_attachment.composer_pending_count -ne 1) { throw 'fixture_not_staged' }
    Act 'set_input_text' @{text=$prompt} | Out-Null
    $report.stage = 'send'
    $since = [DateTimeOffset]::UtcNow.ToUnixTimeMilliseconds()
    Act 'send_input' | Out-Null
    $deadline = [DateTimeOffset]::UtcNow.AddSeconds(110)
    $report.stage = 'reply'
    do {
        $s = Native
        $upload = $s.chatgpt_web_mcp.last_attachment_upload
        if ($s.social_chat.web_chat_attachment_phase -eq 'failed') {
            $report.upload_receipt = $upload.detail
            throw 'attachment_failed'
        }
        $reply = @($s.social_chat.messages | Where-Object role -eq 'friend' | ForEach-Object content) -join "`n"
        if ($s.social_chat.web_chat_attachment_phase -eq 'completed' -and -not $s.social_chat.web_chat_streaming -and
            $reply.Replace('\_', '_').Contains('ELON_CHATGPT_ATTACHMENT_FIXTURE_V1=ready') -and $reply.Contains('TEMP-FILE-OK')) {
            break
        }
        Start-Sleep -Seconds 1
    } while ([DateTimeOffset]::UtcNow -lt $deadline)
    $report.private_upload = $upload.ok -eq $true -and $upload.detail -eq 'private_attachment_associated' -and
        [long]$upload.observed_at_ms -ge $since
    $report.file_reply = $reply.Replace('\_', '_').Contains('ELON_CHATGPT_ATTACHMENT_FIXTURE_V1=ready') -and
        $reply.Contains('TEMP-FILE-OK')
    $report.user_rows = @($s.social_chat.messages | Where-Object role -eq 'user').Count
    $report.attachment_phase = $s.social_chat.web_chat_attachment_phase
    $report.send_receipt = $s.social_chat.web_chat_last_send_command.detail
    $report.private_send = $report.send_receipt -eq 'official_runtime_v1:accepted'
    $postMode = Wait-Mode $true $false
    $report.temporary_selected = $postMode.selected
    $report.temporary_readonly = $postMode.state_settable -eq $false
    if (-not $report.private_upload -or -not $report.private_send -or -not $report.file_reply -or $report.user_rows -ne 1 -or
        -not $report.temporary_readonly -or
        $s.social_chat.web_chat_streaming -or $s.input.text) { throw 'temporary_attachment_unconfirmed' }
    $report.stage = 'library_after'
    $report.library_after_count = Library
    $report.no_added_fixture_in_library = $report.library_after_count -eq $report.library_before_count
    if (-not $report.no_added_fixture_in_library) { throw 'temporary_library_count_changed' }
    $report.passed = $true
    $report.stage = 'complete'
} catch {
    $report.error = ConvertTo-ChatGptWebSmokeSafeDiagnostic -Value $_.Exception.Message
} finally {
    try {
        if ($changed) {
            $s = Native; $web = Web
            if ($s.social_chat.web_chat_streaming -or $s.social_chat.web_chat_attachment_phase -in @('uploading','sending')) {
                throw 'active_transaction_preserved'
            }
            if ($staged -and $s.chatgpt_web_acceptance_attachment.fixture_staged -eq $true) {
                Act 'remove_chatgpt_web_acceptance_attachment' @{fixture_id=$fixtureId} | Out-Null
            }
            if ($s.input.text -ceq $prompt) { Act 'set_input_text' @{text=''} | Out-Null }
            elseif ($s.input.text) { throw 'changed_draft_preserved' }
            Act 'start_new_web_chat_conversation' | Out-Null
            Wait-Mode $false | Out-Null
            $s = Native; $web = Web
            $report.restored = @($s.social_chat.messages).Count -eq 0 -and -not $s.input.text -and
                [int]$web.input.official_draft_length -eq 0 -and -not $web.streaming -and
                $s.chatgpt_web_acceptance_attachment.fixture_staged -eq $false -and
                [int]$s.chatgpt_web_acceptance_attachment.composer_pending_count -eq 0
        } else { $report.restored = $true }
    } catch { $report.cleanup_error = ConvertTo-ChatGptWebSmokeSafeDiagnostic -Value $_.Exception.Message }
    try { $report.awake_restored = Stop-ChatGptWebSmokeAwakeLease -Runtime $runtime }
    catch { $report.awake_restored = $false }
    $report.passed = $report.passed -and $report.restored -and $report.awake_restored
    $report | ConvertTo-Json -Depth 4 -Compress
}
if (-not $report.passed) { exit 1 }
