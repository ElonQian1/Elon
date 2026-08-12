$ErrorActionPreference = "Stop"

$path = Join-Path $PSScriptRoot "smoke-chatgpt-web-audio-lifecycle.ps1"
$source = Get-Content -LiteralPath $path -Raw
$tokens = $null
$errors = $null
[System.Management.Automation.Language.Parser]::ParseFile(
    $path,
    [ref]$tokens,
    [ref]$errors
) | Out-Null
if (@($errors).Count -gt 0) {
    throw "Audio lifecycle smoke has PowerShell parse errors."
}

function Assert-Contains {
    param([Parameter(Mandatory = $true)][string]$Needle)
    if (-not $source.Contains($Needle)) {
        throw "Audio lifecycle smoke contract is missing: $Needle"
    }
}

foreach ($required in @(
    '"Prepare",',
    '"StartDictation",',
    '"VerifyAndCancelDictation",',
    '"StartRealtimeVoice",',
    '"VerifyRealtimeVoice",',
    '"VerifyRestored"',
    '[switch]$UserConfirmedMicrophone',
    '[switch]$UserConfirmedRealtimeVoice',
    '[switch]$UserConfirmedVoiceClosed',
    'audio_lifecycle_checkpoint.v1',
    'audio_lifecycle_smoke.v1',
    'audio_permission.v1',
    'Assert-ChatGptWebSmokeTrustedDevice',
    'Assert-ChatGptWebSmokeAdapterVersion',
    'Wait-ChatGptWebSmokeAuthenticatedReady',
    'Start-ChatGptWebSmokeAwakeLease',
    'Stop-ChatGptWebSmokeAwakeLease',
    'conversation_binding_sha256',
    'device_binding_sha256',
    '-Action "chatgpt_start_dictation"',
    '-ExpectedAction "start_dictation"',
    '-Action "chatgpt_cancel_dictation"',
    '-ExpectedAction "cancel_dictation"',
    'finally {',
    '-AllowNonEmptyDraft',
    '-Action "chatgpt_start_realtime_voice"',
    '-ExpectedAction "invoke_ui_control"',
    '[string]$state.audio.request_state -eq "web_permission_granted"',
    'Run this phase with -UserConfirmedMicrophone',
    'while the user supervises active dictation',
    'Run this phase with -UserConfirmedRealtimeVoice',
    'run this phase with -UserConfirmedVoiceClosed',
    'Move-Item -LiteralPath $temporary -Destination $CheckpointPath -Force',
    'message_count_unchanged = $true',
    'sent_messages = 0',
    'uploaded_attachments = 0',
    'cleared_cookies = $false',
    'cleared_app_data = $false',
    'audio_content_read = $false',
    'private_content_emitted = $false',
    'CHATGPT_WEB_AUDIO_LIFECYCLE_STATUS=$Status',
    'waiting_for_user_microphone_permission',
    'waiting_for_user_realtime_voice_confirmation',
    'waiting_for_user_to_close_realtime_voice',
    '-Status "passed"'
)) {
    Assert-Contains $required
}

foreach ($forbidden in @(
    'send_input',
    'set_input_text',
    'pm clear',
    'removeAllCookies',
    'input tap',
    'KEYCODE_ENTER',
    'keyevent 66',
    '.conversation.title',
    '.messages[',
    'audio_capture_state = "capturing"'
)) {
    if ($source.Contains($forbidden)) {
        throw "Audio lifecycle smoke contains forbidden data or action: $forbidden"
    }
}

if ($source -match '\.input\.text(?!_length)') {
    throw "Audio lifecycle smoke must not read or emit draft text."
}
if ($source -match '(?m)^\s*exit\s+[1-9]') {
    throw "Audio lifecycle smoke must fail through exceptions, not nested exit."
}

$lineCount = @($source -split "`n").Count
if ($lineCount -gt 520) {
    throw "Audio lifecycle smoke exceeded its modular size budget: $lineCount"
}

Write-Output "CHATGPT_WEB_AUDIO_LIFECYCLE_SMOKE_CONTRACT=passed"
