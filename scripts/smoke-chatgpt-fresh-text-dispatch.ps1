#requires -Version 7.0
[CmdletBinding()]
param(
    [string]$Adb = 'D:/Android/sdk/platform-tools/adb.exe',
    [Parameter(Mandatory)][string]$DeviceSerial,
    [Parameter(Mandatory)][string]$ExpectedHardwareSerial,
    [ValidateRange(30,180)][int]$TimeoutSec = 90,
    [switch]$OnlyStop,
    [switch]$UseDefault,
    [switch]$FirstOnly,
    [switch]$StopThenFollowup
)
$ErrorActionPreference = 'Stop'
. (Join-Path $PSScriptRoot 'chatgpt-web-smoke-runtime.ps1')
. (Join-Path $PSScriptRoot 'invoke-android-semantic-acceptance.ps1')
$runtime = New-ChatGptWebSmokeRuntime -Adb $Adb -DeviceSerial $DeviceSerial `
    -ExpectedHardwareSerial $ExpectedHardwareSerial -PollIntervalSec 1
$runtime.mcp_bootstrapped = $true
$fixtureFile = Join-Path (Split-Path -Parent $PSScriptRoot) '.ai-tmp/fresh-text-fixture.json'
$originPath = ''; $originDraft = ''; $opened = $false; $restored = $false; $awakeRestored = $false
$report = [ordered]@{ schema = 'elon.fresh_text_ui.v1'; passed = $false; stage = 'opening';
    seed_sends = 0; candidate_clicks = 0; cases = @(); restored = $false; awake_restored = $false }

function Trial([string]$Mode) {
    $result = Invoke-ChatGptWebSmokeAction -Runtime $runtime -Action chatgpt_private_protocol_probe `
        -Arguments @{ mode = "fresh_text_trial_$Mode" }
    $id = [string]$result.command_receipt.request_id
    if (-not $id) { throw 'trial_receipt_missing' }
    $until = [DateTimeOffset]::UtcNow.AddSeconds(12)
    do {
        $state = Invoke-ChatGptWebSmokeMcp -Runtime $runtime -Tool ui_state
        $receipt = @($state.command_requests | Where-Object { $_.request_id -eq $id }) | Select-Object -Last 1
        if ($receipt.result.detail) {
            try { $value = [string]$receipt.result.detail | ConvertFrom-Json } catch { throw 'trial_receipt_invalid' }
            if ($value.schema -ne 'elon.fresh_text_trial.v1') { throw 'trial_schema_invalid' }
            return $value
        }
        Start-Sleep -Milliseconds 200
    } while ([DateTimeOffset]::UtcNow -lt $until)
    throw 'trial_receipt_timeout'
}

function Native-Send([string]$Kind, [bool]$Candidate) {
    $stamp = [DateTimeOffset]::UtcNow.ToUnixTimeMilliseconds()
    $marker = "FRESH_$($Kind.ToUpperInvariant())_$stamp"
    $prompt = "ELON_FRESH_TEXT_ACCEPTANCE_V1 $Kind $stamp. Reply exactly $marker."
    $stop = $Kind -eq 'stop'; $stopClicked = $false
    if ($stop) { $prompt = "ELON_FRESH_TEXT_ACCEPTANCE_V1 stop $stamp. Write a numbered list of 1000 simple English words. Do not summarize." }
    Invoke-AndroidSemanticAcceptance -Runtime $runtime -TestClass CanvasUiAcceptance `
        -Step focus_composer -ResultPrefix CANVAS_UI_RESULT | Out-Null
    $before = if ($Candidate) { Trial $(if ($UseDefault) { 'state' } else { 'start' }) } else { $null }
    if ($Candidate -and -not $UseDefault -and $before.armed -ne $true) { throw "trial_not_armed:$($before.control)" }
    $started = [DateTimeOffset]::UtcNow
    Invoke-AndroidSemanticAcceptance -Runtime $runtime -TestClass ConversationUiAcceptance `
        -Step send_fresh_text_fixture -ResultPrefix CONVERSATION_UI_RESULT `
        -Parameters @{ prompt_b64=[Convert]::ToBase64String([Text.Encoding]::UTF8.GetBytes($prompt)) } | Out-Null
    if ($Candidate) { $report.candidate_clicks++ } else { $report.seed_sends++ }
    $until = $started.AddSeconds($TimeoutSec)
    $firstReplyMs = $null; $matched = $false; $diagnostic = $null
    do {
        $main = Invoke-ChatGptWebSmokeMcp -Runtime $runtime -Tool ui_state -MainState
        $messages = @($main.social_chat.messages)
        $users = @($messages | Where-Object { $_.role -eq 'user' -and $_.content -eq $prompt })
        $matched = @($messages | Where-Object {
            $_.role -eq 'friend' -and (([string]$_.content -replace '\\([_-])', '$1').Contains($marker))
        }).Count -gt 0
        if (-not $Candidate -and $users.Count -eq 1 -and
            [string]$main.social_chat.web_chat_conversation_path -match '^/c/[a-f0-9-]{36}$') {
            @{path=[string]$main.social_chat.web_chat_conversation_path} | ConvertTo-Json -Compress |
                Set-Content -LiteralPath $fixtureFile -Encoding utf8
        }
        if ($users.Count -eq 1 -and $matched -and $null -eq $firstReplyMs) {
            $firstReplyMs = [long]([DateTimeOffset]::UtcNow - $started).TotalMilliseconds
        }
        if ($users.Count -gt 1) { throw 'duplicate_user_message' }
        if ($Candidate) {
            $diagnostic = Trial 'state'
            $report.last_trial = $diagnostic
            if ($diagnostic.phase -eq 'rejected') { throw "fresh_rejected:$($diagnostic.code)" }
            if ($diagnostic.attempts -ne ($before.attempts + 1)) { throw 'fresh_route_not_used' }
            if ($stop) { $report.stop_native_streaming = $main.social_chat.web_chat_streaming -eq $true }
            if ($stop -and -not $stopClicked -and $diagnostic.accepted -and $diagnostic.pending) {
                Invoke-AndroidSemanticAcceptance -Runtime $runtime -TestClass ConversationUiAcceptance `
                    -Step stop_fresh_text_fixture -ResultPrefix CONVERSATION_UI_RESULT | Out-Null
                $stopClicked = $true
                $report.stop_clicked = $true
            }
        }
        if (($matched -or ($stop -and $stopClicked)) -and $users.Count -eq 1 -and $main.social_chat.web_chat_streaming -ne $true -and
            (-not $Candidate -or ($diagnostic.dispatched -and $diagnostic.accepted -and
                $diagnostic.reconciled -and -not $diagnostic.pending))) {
            return [ordered]@{ kind=$Kind; native_button=$true; unique_user=$true; reply_matched=$matched; stop_clicked=$stopClicked;
                fresh_http=$Candidate; reconciled=(!$Candidate -or $diagnostic.reconciled);
                parent_role=$(if ($diagnostic) { $diagnostic.parent_role } else { 'unknown' });
                reply_observed_ms=$firstReplyMs; total_ms=[long]([DateTimeOffset]::UtcNow - $started).TotalMilliseconds }
        }
        Start-Sleep -Seconds 1
    } while ([DateTimeOffset]::UtcNow -lt $until)
    $code = if ($diagnostic) { "$($diagnostic.phase):$($diagnostic.code)" } else { 'seed_reply_missing' }
    throw "fresh_acceptance_timeout:$code"
}

Assert-ChatGptWebSmokeTrustedDevice -Runtime $runtime
Start-ChatGptWebSmokeAwakeLease -Runtime $runtime | Out-Null
try {
    $origin = Open-WebChatNativeChatSurface -Runtime $runtime -ProviderId chatgpt_web -TimeoutSec $TimeoutSec
    $opened = $true; $originPath = [string]$origin.social_chat.web_chat_conversation_path
    $originDraft = [string]$origin.input.text
    Trial 'end' | Out-Null
    if (Test-Path -LiteralPath $fixtureFile) {
        $saved = Get-Content -LiteralPath $fixtureFile -Raw | ConvertFrom-Json
        if ($saved.path -notmatch '^/c/[a-f0-9-]{36}$') { throw 'fixture_path_invalid' }
        Invoke-ChatGptWebSmokeAction -Runtime $runtime -Action open_web_chat_conversation `
            -Arguments @{conversation_path=$saved.path} | Out-Null
        Wait-ChatGptWebSmokeState -Runtime $runtime -MainState -TimeoutSec $TimeoutSec -Description 'owned text fixture' -Predicate {
            param($state)
            $state.social_chat.web_chat_conversation_path -eq $saved.path -and
                $state.social_chat.web_chat_composer_ready -eq $true -and $state.social_chat.message_count -gt 0
        } | Out-Null
        $fixture = Invoke-ChatGptWebSmokeMcp -Runtime $runtime -Tool ui_state -MainState
        $users = @($fixture.social_chat.messages | Where-Object { $_.role -eq 'user' })
        if (-not $users.Count -or @($users | Where-Object {
            -not ([string]$_.content).StartsWith('ELON_FRESH_TEXT_ACCEPTANCE_V1 ')
        }).Count) { throw 'fixture_content_not_owned' }
    } else {
        Invoke-ChatGptWebSmokeAction -Runtime $runtime -Action start_new_web_chat_conversation | Out-Null
        Wait-ChatGptWebSmokeState -Runtime $runtime -MainState -TimeoutSec $TimeoutSec -Description 'new native text fixture' -Predicate {
            param($state)
            $state.social_chat.web_chat_composer_ready -eq $true -and $state.social_chat.message_count -eq 0
        } | Out-Null
        $report.stage = 'seed'; Write-Output 'FRESH_TEXT_STAGE=seed'
        $report.cases += Native-Send 'seed' $false
        $fixture = Invoke-ChatGptWebSmokeMcp -Runtime $runtime -Tool ui_state -MainState
        $path = [string]$fixture.social_chat.web_chat_conversation_path
        if ($path -notmatch '^/c/[a-f0-9-]{36}$') { throw 'fixture_route_missing' }
        New-Item -ItemType Directory -Force -Path (Split-Path -Parent $fixtureFile) | Out-Null
        @{path=$path} | ConvertTo-Json -Compress | Set-Content -LiteralPath $fixtureFile -Encoding utf8
    }
    foreach ($kind in $(if ($StopThenFollowup) { @('stop','followup') } elseif ($OnlyStop) { @('stop') } elseif ($FirstOnly) { @('first') } else { @('first','followup') })) {
        $report.stage = $kind; Write-Output "FRESH_TEXT_STAGE=$kind"
        $report.cases += Native-Send $kind $true
    }
    $report.passed = $true; $report.stage = 'complete'
} catch {
    $report.error = if ($_.Exception.Message -match '^[a-z_:]+$') { $_.Exception.Message } else { 'acceptance_failed' }
    if ($_.Exception.Message -match '^Semantic UI acceptance failed: ([a-z_]+)$') { $report.error = $Matches[1] }
    $report.failure_line = $_.InvocationInfo.ScriptLineNumber
} finally {
    if ($opened) {
        try { Trial 'end' | Out-Null } catch {}
        if ($originPath) { $restored = Restore-WebChatNativeConversation -Runtime $runtime -ProviderId chatgpt_web -ConversationPath $originPath }
        if ($restored) {
            Invoke-ChatGptWebSmokeAction -Runtime $runtime -Action set_input_text -Arguments @{text=$originDraft} | Out-Null
            Invoke-ChatGptWebSmokeAction -Runtime $runtime -Action show_conversation_home | Out-Null
        }
    }
    $awakeRestored = Stop-ChatGptWebSmokeAwakeLease -Runtime $runtime
    $report.restored = $restored; $report.awake_restored = [bool]$awakeRestored
    $report | ConvertTo-Json -Depth 5 -Compress
}
if (-not $report.passed -or -not $restored -or -not $awakeRestored) { throw 'fresh_text_acceptance_incomplete' }
