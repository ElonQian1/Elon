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
$report = [ordered]@{ schema = 'elon.chatgpt.media_batch.v1'; passed = $false
    stage = 'prepare'; restored = $false; awake_restored = $false; private_content_emitted = $false }
$changed = $false
$fixtureId = 'fixed_media_batch_v1'
# The expected document lines and image facts must not occur in the prompt.
$prompt = 'Read all three attached test files. Reply in English: quote the exact first line from each document, then describe the shapes in the image, including their counts and colors. If an attachment is unavailable, say so instead of guessing.'

function Native {
    if (-not (Test-WebChatNativeChatSurfaceForeground -Runtime $runtime)) { throw 'foreground_changed' }
    return Get-ChatGptWebNativeChatState -Runtime $runtime
}
function Web { return Invoke-ChatGptWebSmokeMcp -Runtime $runtime -Tool ui_state }
function Act([string]$Action, [hashtable]$Arguments = @{}) {
    $payload = @{} + $Arguments; $payload.action = $Action
    # A timed-out write is unconfirmed, never an invitation to send it twice.
    $raw = @(& $runtime.invoke_mcp -Adb $runtime.adb -DeviceSerial $DeviceSerial -Tool ui_control `
        -Arguments ($payload | ConvertTo-Json -Depth 8 -Compress) -NoBootstrap `
        -HealthTimeoutSec 5 -RequestTimeoutSec 30 -AdbTimeoutSec 8) | Select-Object -Last 1
    if ($raw.result.isError -or $raw.result.structuredContent.control_ok -ne $true) { throw 'action_unconfirmed' }
    return $raw.result.structuredContent
}
function Stage-Bundle {
    Act 'stage_chatgpt_web_acceptance_attachment' @{fixture_id=$fixtureId} | Out-Null
    $state = Native
    if ($state.chatgpt_web_acceptance_attachment.fixture_staged -ne $true -or
        [int]$state.chatgpt_web_acceptance_attachment.composer_pending_count -ne 3) { throw 'media_bundle_not_staged' }
    return $state
}
function Check-Reply([string]$Text) {
    $plain = $Text.Replace('\_', '_').Replace('**', '').Replace('`', '')
    return @{
        text_read = $plain.Contains('ELON_CHATGPT_ATTACHMENT_FIXTURE_V1=ready')
        pdf_read = $plain.Contains('ELON_PRIVATE_PDF_FIXTURE_V1=ready')
        image_read = $plain -match '(?is)\b(three|3)\s+blue\s+squares?\b' -and
            $plain -match '(?is)\b(one|1|a)\s+red\s+circle\b'
    }
}

try {
    Assert-ChatGptWebSmokeTrustedDevice -Runtime $runtime
    if ((Get-ChatGptWebSmokeUserReadiness -Runtime $runtime).ready -ne $true) { throw 'device_locked' }
    $origin = Native; $web = Web
    if ($origin.social_chat.web_chat_provider_id -ne 'chatgpt_web' -or
        $origin.social_chat.interaction_mode -ne 'chat' -or $web.authenticated -ne $true -or
        $web.composer_ready -ne $true) { throw 'surface_not_ready' }
    $page = [uri]$web.conversation.url
    if ($page.Scheme -ne 'https' -or $page.Host -ne 'chatgpt.com' -or $page.AbsolutePath -ne '/') {
        throw 'ordinary_new_chat_required'
    }
    $temporary = $web.ui_manifest.controls | Where-Object semantic -eq 'temporary_chat' | Select-Object -First 1
    if ($null -eq $temporary -or $temporary.selected -ne $false) { throw 'ordinary_chat_required' }
    if (@($origin.social_chat.messages).Count -gt 0 -or $origin.input.text -or $web.streaming -or $web.dictation_active -or
        [int]$web.input.official_draft_length -gt 0 -or [int]$origin.social_chat.web_chat_pending_attachment_count -gt 0 -or
        [int]$origin.chatgpt_web_acceptance_attachment.composer_pending_count -gt 0) { throw 'blank_idle_chat_required' }
    if ($fixtureId -notin $origin.chatgpt_web_acceptance_attachment.supported_fixture_ids) { throw 'media_fixture_not_supported' }
    Start-ChatGptWebSmokeAwakeLease -Runtime $runtime | Out-Null
    $changed = $true
    $report.stage = 'stage_remove'
    $beforeUpload = $origin.chatgpt_web_mcp.last_attachment_upload.observed_at_ms
    $s = Stage-Bundle
    $report.no_early_upload_receipt = $s.chatgpt_web_mcp.last_attachment_upload.observed_at_ms -eq $beforeUpload
    Act 'remove_chatgpt_web_acceptance_attachment' @{fixture_id=$fixtureId} | Out-Null
    $s = Native
    $report.local_remove = $s.chatgpt_web_acceptance_attachment.fixture_staged -eq $false -and
        [int]$s.chatgpt_web_acceptance_attachment.composer_pending_count -eq 0
    if (-not $report.local_remove -or -not $report.no_early_upload_receipt) { throw 'local_bundle_lifecycle_failed' }
    Stage-Bundle | Out-Null
    Act 'set_input_text' @{text=$prompt} | Out-Null
    $report.stage = 'send'
    $since = [DateTimeOffset]::UtcNow.ToUnixTimeMilliseconds()
    $report.synthetic_remote_artifacts_may_remain = $true
    Act 'send_input' | Out-Null
    $deadline = [DateTimeOffset]::UtcNow.AddSeconds(120)
    $report.stage = 'reply'
    do {
        $s = Native
        $upload = $s.chatgpt_web_mcp.last_attachment_upload
        if ($s.social_chat.web_chat_attachment_phase -eq 'failed') {
            $report.upload_receipt = ConvertTo-ChatGptWebSmokeSafeDiagnostic -Value $upload.detail
            throw 'attachment_failed'
        }
        $reply = @($s.social_chat.messages | Where-Object role -eq 'friend' | ForEach-Object content) -join "`n"
        $facts = Check-Reply $reply
        if ($s.social_chat.web_chat_attachment_phase -eq 'completed' -and -not $s.social_chat.web_chat_streaming -and
            $facts.text_read -and $facts.pdf_read -and $facts.image_read) { break }
        Start-Sleep -Seconds 1
    } while ([DateTimeOffset]::UtcNow -lt $deadline)
    $report.elapsed_ms = [DateTimeOffset]::UtcNow.ToUnixTimeMilliseconds() - $since
    $report.private_upload = $upload.ok -eq $true -and $upload.detail -eq 'private_attachment_associated' -and
        [long]$upload.observed_at_ms -ge $since
    $report.text_read = $facts.text_read; $report.pdf_read = $facts.pdf_read; $report.image_read = $facts.image_read
    $report.user_rows = @($s.social_chat.messages | Where-Object role -eq 'user').Count
    $report.attachment_phase = $s.social_chat.web_chat_attachment_phase
    $report.private_send = $s.social_chat.web_chat_last_send_command.detail -eq 'official_runtime_v1:accepted'
    if (-not $report.private_upload -or -not $report.private_send -or -not $facts.text_read -or -not $facts.pdf_read -or
        -not $facts.image_read -or $report.user_rows -ne 1 -or $s.social_chat.web_chat_streaming -or $s.input.text) {
        throw 'mixed_media_reply_unconfirmed'
    }
    $report.passed = $true; $report.stage = 'complete'
} catch {
    $report.error = ConvertTo-ChatGptWebSmokeSafeDiagnostic -Value $_.Exception.Message
} finally {
    try {
        if ($changed) {
            $s = Native
            if ($s.social_chat.web_chat_streaming -or $s.social_chat.web_chat_attachment_phase -in @('uploading','sending')) {
                throw 'active_transaction_preserved'
            }
            if ($s.input.text -and $s.input.text -cne $prompt) { throw 'changed_draft_preserved' }
            if ($s.chatgpt_web_acceptance_attachment.fixture_staged -eq $true) {
                Act 'remove_chatgpt_web_acceptance_attachment' @{fixture_id=$fixtureId} | Out-Null
            }
            if ($s.input.text -ceq $prompt) { Act 'set_input_text' @{text=''} | Out-Null }
            # Only local pinned bytes are removed. Never delete a user's Library records.
            Act 'start_new_web_chat_conversation' | Out-Null
            $deadline = [DateTimeOffset]::UtcNow.AddSeconds(20)
            do {
                $s = Native; $web = Web
                $report.restored = @($s.social_chat.messages).Count -eq 0 -and -not $s.input.text -and
                    [int]$web.input.official_draft_length -eq 0 -and -not $web.streaming -and $web.composer_ready -eq $true -and
                    $s.chatgpt_web_acceptance_attachment.fixture_staged -eq $false -and
                    [int]$s.chatgpt_web_acceptance_attachment.composer_pending_count -eq 0
                if ($report.restored) { break }
                Start-Sleep -Seconds 1
            } while ([DateTimeOffset]::UtcNow -lt $deadline)
        } else { $report.restored = $true }
    } catch { $report.cleanup_error = ConvertTo-ChatGptWebSmokeSafeDiagnostic -Value $_.Exception.Message }
    try { $report.awake_restored = Stop-ChatGptWebSmokeAwakeLease -Runtime $runtime }
    catch { $report.awake_restored = $false }
    $report.passed = $report.passed -and $report.restored -and $report.awake_restored
    $report | ConvertTo-Json -Depth 4 -Compress
}
if (-not $report.passed) { exit 1 }
