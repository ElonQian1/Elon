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
    '"VerifyAndClearDictation",',
    '"StartRealtimeVoice",',
    '"VerifyRealtimeVoice",',
    '"VerifyRestored"',
    '[switch]$UserConfirmedMicrophone',
    '[switch]$UserConfirmedRealtimeVoice',
    '[switch]$UserConfirmedVoiceRoundTrip',
    '[switch]$UserConfirmedVoiceClosed',
    '[ValidateRange(5, 60)][int]$ManualDictationGraceSec = 30',
    'audio_lifecycle_checkpoint.v1',
    'audio_lifecycle_smoke.v1',
    'audio_permission.v1',
    'Assert-ChatGptWebSmokeTrustedDevice',
    'Assert-ChatGptWebSmokeAdapterVersion',
    'Wait-ChatGptWebSmokeAuthenticatedReady',
    'Open-ChatGptWebSmokeSurface -Runtime $runtime',
    'Start-ChatGptWebSmokeAwakeLease',
    'Stop-ChatGptWebSmokeAwakeLease',
    'conversation_binding_sha256',
    'isolated_conversation_path = Get-ChatGptWebSmokeConversationPath',
    'device_binding_sha256',
    'Start-ChatGptWebSmokeIsolatedConversation',
    'Restore-ChatGptWebSmokeOrigin',
    'dictation_request_id = ""',
    'realtime_voice_request_id = ""',
    'voice_round_trip_confirmed = $false',
    '-Action "chatgpt_start_dictation"',
    '$checkpoint.phase = "dictation_active"',
    'active ChatGPT dictation',
    '$manualDeadline = [DateTimeOffset]::UtcNow.AddSeconds(',
    'if ($completionState.dictation_active -ne $true) { break }',
    '-Action "chatgpt_submit_dictation"',
    '-ExpectedAction "submit_dictation"',
    'completed non-empty ChatGPT dictation draft',
    'finally {',
    '-AllowNonEmptyDraft',
    '-Action "chatgpt_start_realtime_voice"',
    '-ExpectedAction "invoke_ui_control"',
    '[string]$state.audio.request_state -eq "web_permission_granted"',
    '[int]$state.input.text_length -gt 0',
    '-Action "set_input_text" -Arguments @{ text = "" }',
    '-ExpectedAction "set_draft"',
    'Run this phase with -UserConfirmedMicrophone',
    'while the user supervises active dictation',
    'Run this phase with -UserConfirmedRealtimeVoice',
    'run this phase with -UserConfirmedVoiceClosed',
    'Move-Item -LiteralPath $temporary -Destination $CheckpointPath -Force',
    '[bool]$MessageCountUnchanged = $true',
    'message_count_unchanged = $MessageCountUnchanged',
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
    'Register-ChatGptWebVerificationCases',
    '-CaseIds @("supervised/dictation_transcription")',
    '-CaseIds @("supervised/realtime_voice_round_trip")',
    'voice_round_trip_confirmed = $true',
    'Promote-FirstTurnConversationBinding -State $activeVoice -Checkpoint $checkpoint',
    '[string]$Checkpoint.phase -ne "realtime_voice_started"',
    '[string]$Checkpoint.isolated_conversation_path',
    '[int]$Checkpoint.message_count -ne 0',
    '[int]$State.conversation.message_count -lt 2',
    '-not $currentPath',
    'voice_message_count_added = $messagesAdded',
    '$messagesAdded -lt 2',
    'Realtime voice did not persist a complete user and assistant round trip.',
    '-MessageCountUnchanged:$false',
    'only after the user spoke and heard a ChatGPT voice reply',
    '-Status "passed"'
)) {
    Assert-Contains $required
}

if ($source.Contains('-Action "open_chatgpt_web"')) {
    throw "Audio lifecycle phases must preserve the isolated conversation instead of reopening the entry route."
}

foreach ($forbidden in @(
    'send_input',
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
if (@([regex]::Matches($source, '-Action "set_input_text"')).Count -ne 1 -or
    -not $source.Contains('-Action "set_input_text" -Arguments @{ text = "" }')) {
    throw "Audio lifecycle may only clear the recognized draft without reading or replacing it."
}
if ($source -match '(?m)^\s*exit\s+[1-9]') {
    throw "Audio lifecycle smoke must fail through exceptions, not nested exit."
}


$dictationRegister = $source.IndexOf('-CaseIds @("supervised/dictation_transcription")')
$dictationRestored = $source.IndexOf('Assert-IdleComposer -State $restored -Checkpoint $checkpoint')
if ($dictationRegister -le $dictationRestored) {
    throw "Dictation evidence must be recorded only after cancel and restoration."
}
$voiceRegister = $source.IndexOf('-CaseIds @("supervised/realtime_voice_round_trip")')
$voiceRestored = $source.LastIndexOf('Assert-IdleComposer -State $restored -Checkpoint $checkpoint')
if ($voiceRegister -le $voiceRestored) {
    throw "Realtime voice evidence must be recorded only after the official voice UI is closed."
}
$originRestore = $source.LastIndexOf('Restore-ChatGptWebSmokeOrigin')
if ($voiceRegister -le $originRestore) {
    throw "Realtime voice evidence must be recorded only after the original conversation is restored."
}

$lineCount = @($source -split "`n").Count
if ($lineCount -gt 520) {
    throw "Audio lifecycle smoke exceeded its modular size budget: $lineCount"
}

Write-Output "CHATGPT_WEB_AUDIO_LIFECYCLE_SMOKE_CONTRACT=passed"
