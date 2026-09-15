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
    [switch]$StopThenFollowup,
    [switch]$NewConversation,
    [switch]$ComposerUnavailable,
    [switch]$BackgroundResume,
    [switch]$ExistingProjectFixture
)
$ErrorActionPreference = 'Stop'
if ($ExistingProjectFixture -and ($NewConversation -or $ComposerUnavailable -or $BackgroundResume -or $OnlyStop -or $StopThenFollowup)) {
    throw 'project_fixture_requires_existing_text_scope'
}
if ($NewConversation -and ($OnlyStop -or $StopThenFollowup)) { throw 'new_conversation_stop_scope_not_supported' }
if ($ComposerUnavailable -and (!$NewConversation -or !$FirstOnly -or !$UseDefault)) { throw 'composer_lease_requires_default_new_first' }
if ($BackgroundResume -and (!$FirstOnly -or !$UseDefault -or $ComposerUnavailable -or $OnlyStop -or $StopThenFollowup)) {
    throw 'background_resume_requires_single_default_send'
}
. (Join-Path $PSScriptRoot 'chatgpt-web-smoke-runtime.ps1')
. (Join-Path $PSScriptRoot 'chatgpt-fresh-trial-smoke.ps1')
. (Join-Path $PSScriptRoot 'chatgpt-fresh-text-smoke-evidence.ps1')
. (Join-Path $PSScriptRoot 'invoke-android-semantic-acceptance.ps1')
. (Join-Path $PSScriptRoot 'chatgpt-composer-dom-smoke.ps1')
. (Join-Path $PSScriptRoot 'chatgpt-fresh-background-smoke.ps1')
. (Join-Path $PSScriptRoot 'chatgpt-fresh-project-smoke.ps1')
$runtime = New-ChatGptWebSmokeRuntime -Adb $Adb -DeviceSerial $DeviceSerial `
    -ExpectedHardwareSerial $ExpectedHardwareSerial -PollIntervalSec 1
$runtime.mcp_bootstrapped = $true
$fixtureFile = Join-Path (Split-Path -Parent $PSScriptRoot) '.ai-tmp/fresh-text-fixture.json'
if ($ExistingProjectFixture) { $fixtureFile = Join-Path (Split-Path -Parent $PSScriptRoot) '.ai-tmp/fresh-project-text-fixture.json' }
$commonGit = [IO.Path]::GetFullPath((& git -C (Split-Path -Parent $PSScriptRoot) rev-parse --git-common-dir).Trim())
$pendingFile = Join-Path $commonGit 'ai-acceptance-fixtures/fresh-text-pending.json'
$resolvedPending = $null; $pendingHash = $null
if (Test-Path -LiteralPath $pendingFile) {
    $resolvedPending = Get-Content -LiteralPath $pendingFile -Raw | ConvertFrom-Json
    $pendingHash = (Get-FileHash -LiteralPath $pendingFile -Algorithm SHA256).Hash
    if ($resolvedPending.readback_completed -isnot [bool] -or !$resolvedPending.readback_completed) {
        throw 'pending_fresh_fixture_requires_readonly_resolution'
    }
}
$originPath = ''; $originDraft = ''; $opened = $false; $restored = $false; $awakeRestored = $false
$awaitingResult = $false; $trialRequested = $false; $ownedNavigation = $false; $expectedPath = ''
$lastPrompt = ''; $lastUserId = ''; $lastObservedPath = ''; $clickAcknowledged = $false
$composerLease = $null
$projectId = ''
$report = [ordered]@{ schema = 'elon.fresh_text_ui.v1'; passed = $false; stage = 'opening';
    new_conversation = [bool]$NewConversation; seed_sends = 0; candidate_clicks = 0; cases = @();
    restored = $false; awake_restored = $false; write_unconfirmed = $false;
    state_probe_timeouts = 0; foreground_changed = $false; send_not_attempted = $false; background_resume = $null }

function Trial([string]$Mode) {
    Invoke-ChatGptFreshTrial -Runtime $runtime -Mode $Mode
}

function Native-Send([string]$Kind, [bool]$Candidate, [bool]$NewFirst = $false) {
    $stamp = [DateTimeOffset]::UtcNow.ToUnixTimeMilliseconds()
    $marker = "FRESH_$($Kind.ToUpperInvariant())_$stamp"
    $prompt = "ELON_FRESH_TEXT_ACCEPTANCE_V1 $Kind $stamp. Reply exactly $marker."
    if ($BackgroundResume) {
        $prompt = "ELON_FRESH_TEXT_ACCEPTANCE_V1 background $stamp. Write 30 numbered short English sentences. End with $marker."
    }
    $stop = $Kind -eq 'stop'; $stopClicked = $false
    if ($stop) { $prompt = "ELON_FRESH_TEXT_ACCEPTANCE_V1 stop $stamp. Write a numbered list of 1000 simple English words. Do not summarize." }
    Invoke-AndroidSemanticAcceptance -Runtime $runtime -TestClass CanvasUiAcceptance `
        -Step focus_composer -ResultPrefix CANVAS_UI_RESULT | Out-Null
    $baseline = Invoke-ChatGptWebSmokeMcp -Runtime $runtime -Tool ui_state
    if ($baseline.input.text -isnot [string] -or $baseline.input.text -cne '' -or
        $baseline.streaming -isnot [bool] -or $baseline.streaming) { throw 'send_baseline_not_idle' }
    if ($NewFirst -and ($baseline.authenticated -ne $true -or @($baseline.conversation.messages).Count -ne 0 -or
        [string]$baseline.conversation.url -cnotmatch '^https://chatgpt.com/?$')) { throw 'new_first_scope_not_empty_personal_chat' }
    $priorReceiptIds = @($baseline.command_requests | ForEach-Object { [string]$_.request_id })
    if ($Candidate -and !$UseDefault) { $script:trialRequested = $true }
    $before = if ($Candidate) { Trial $(if ($UseDefault) { 'state' } else { 'start' }) } else { $null }
    if ($Candidate -and -not $UseDefault -and $before.armed -ne $true) { throw "trial_not_armed:$($before.control)" }
    if ($composerLease) {
        Assert-ChatGptComposerDomUnavailable (Invoke-ChatGptComposerDomLease -Lease $composerLease -Action state)
        if ($baseline.private_send_ready -ne $true) { throw 'composer_lease_private_readiness_missing' }
        $report.composer_unavailable_before_click = $true
        $report.native_composer_ready_before_click = $baseline.composer_ready
    }
    $started = [DateTimeOffset]::UtcNow
    # A lost click acknowledgement must not permit replay, navigation or draft cleanup.
    $script:awaitingResult = $true
    $script:lastPrompt = $prompt; $script:lastUserId = ''
    $script:lastObservedPath = if ($projectId) { $expectedPath } else { '' }; $script:clickAcknowledged = $false
    if ($Candidate) { $report.candidate_clicks++ } else { $report.seed_sends++ }
    Write-Host "FRESH_TEXT_PROGRESS kind=$Kind phase=native_click_requested"
    try {
        Invoke-AndroidSemanticAcceptance -Runtime $runtime -TestClass ConversationUiAcceptance `
            -Step send_fresh_text_fixture -ResultPrefix CONVERSATION_UI_RESULT `
            -Parameters @{ prompt_b64=[Convert]::ToBase64String([Text.Encoding]::UTF8.GetBytes($prompt)) } | Out-Null
    } catch {
        if (Test-ChatGptFreshClickNotDispatched $_.Exception.Message) {
            $script:awaitingResult = $false
            $report.foreground_changed = $true
            $report.send_not_attempted = $true
            if ($Candidate) { $report.candidate_clicks-- } else { $report.seed_sends-- }
        }
        throw
    }
    $script:clickAcknowledged = $true
    Write-Host "FRESH_TEXT_PROGRESS kind=$Kind phase=native_click_acknowledged"
    $until = $started.AddSeconds($TimeoutSec)
    $firstReplyMs = $null; $matched = $false; $diagnostic = $null
    do {
        $main = Invoke-ChatGptWebSmokeMcp -Runtime $runtime -Tool ui_state -MainState
        $web = Invoke-ChatGptWebSmokeMcp -Runtime $runtime -Tool ui_state
        $receipts = @($web.command_requests | Where-Object {
            $_.expected_web_action -ceq 'send_prompt' -and $_.request_id -notin $priorReceiptIds
        })
        if ($receipts.Count -gt 1) { throw 'multiple_send_receipts' }
        $receipt = $receipts | Select-Object -Last 1
        $messages = @($main.social_chat.messages)
        $users = @($messages | Where-Object { $_.role -eq 'user' -and $_.content -eq $prompt })
        $webUsers = @($web.conversation.messages | Where-Object { $_.role -ceq 'user' -and $_.content -ceq $prompt })
        if ($webUsers.Count -eq 1) { $script:lastUserId = [string]$webUsers[0].id }
        if ($NewFirst -and !$script:lastObservedPath) {
            $script:lastObservedPath = Get-ChatGptFreshPendingObservedPath -Web $web -Prompt $prompt -UserMessageId $script:lastUserId
        }
        $answerCount = @($messages | Where-Object {
            $_.role -eq 'friend' -and (([string]$_.content -replace '\\([_-])', '$1').Contains($marker))
        }).Count
        $matched = $answerCount -eq 1
        if ($users.Count -eq 1 -and $matched -and $null -eq $firstReplyMs) {
            $firstReplyMs = [long]([DateTimeOffset]::UtcNow - $started).TotalMilliseconds
        }
        if ($users.Count -gt 1) { throw 'duplicate_user_message' }
        if ($Candidate) {
            try { $diagnostic = Trial 'state' }
            catch {
                if ($_.Exception.Message -cne 'fresh_trial_receipt_timeout') { throw }
                # Only this read-only probe may be retried within the send's
                # original deadline. Never dispatch or arm another write here.
                $report.state_probe_timeouts = 1 + [int]$report.state_probe_timeouts
                continue
            }
            $report.last_trial = $diagnostic
            if ($diagnostic.phase -eq 'rejected') { throw "fresh_rejected:$($diagnostic.code)" }
            if ($diagnostic.attempts -gt ($before.attempts + 1)) { throw 'multiple_fresh_attempts' }
            if ($BackgroundResume -and !$report.background_resume -and
                (Test-ChatGptFreshBackgroundAdmission -Before $before -Current $diagnostic)) {
                $report.background_resume = Invoke-ChatGptFreshBackgroundResume -Runtime $runtime
                continue
            }
            if ($stop) { $report.stop_native_streaming = $main.social_chat.web_chat_streaming -eq $true }
            if ($stop -and -not $stopClicked -and $diagnostic.accepted -and $diagnostic.pending) {
                Invoke-AndroidSemanticAcceptance -Runtime $runtime -TestClass ConversationUiAcceptance `
                    -Step stop_fresh_text_fixture -ResultPrefix CONVERSATION_UI_RESULT | Out-Null
                $stopClicked = $true
                $report.stop_clicked = $true
            }
        }
        $freshConfirmed = $Candidate -and (Test-ChatGptFreshSendEvidence -Before $before -After $diagnostic `
            -Receipt $receipt -UseDefault:$UseDefault)
        $continuity = Test-ChatGptFreshSendContinuity -Before $baseline -After $web -Main $main `
            -Prompt $prompt -NewConversation:$NewFirst -ProjectId $projectId
        $report.last_checks = [ordered]@{ native_user_count=$users.Count; native_answer_count=$answerCount;
            receipt_count=$receipts.Count; fresh_receipt=$freshConfirmed; continuity=$continuity;
            native_streaming=$main.social_chat.web_chat_streaming; web_streaming=$web.streaming }
        if (($matched -or ($stop -and $stopClicked)) -and $users.Count -eq 1 -and
            $main.social_chat.web_chat_streaming -is [bool] -and !$main.social_chat.web_chat_streaming -and
            $web.streaming -is [bool] -and !$web.streaming -and $continuity -and
            (!$Candidate -or $freshConfirmed)) {
            if ($composerLease) {
                $leaseState = Invoke-ChatGptComposerDomLease -Lease $composerLease -Action state
                Assert-ChatGptComposerDomUnavailable $leaseState
                $report.composer_lease = $leaseState
            }
            $script:awaitingResult = $false
            $script:expectedPath = [string]$main.social_chat.web_chat_conversation_path
            New-Item -ItemType Directory -Force -Path (Split-Path -Parent $fixtureFile) | Out-Null
            @{path=$script:expectedPath} | ConvertTo-Json -Compress | Set-Content -LiteralPath $fixtureFile -Encoding utf8
            return [ordered]@{ kind=$Kind; native_button=$true; unique_user=$true; reply_matched=$matched; stop_clicked=$stopClicked;
                fresh_http=$freshConfirmed; reconciled=(!$Candidate -or $diagnostic.reconciled);
                new_first=$NewFirst; conversation_identity_verified=$continuity; receipt_verified=(!$Candidate -or $freshConfirmed);
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
    $origin = Invoke-ChatGptWebSmokeMcp -Runtime $runtime -Tool ui_state -MainState
    if ($origin.active_surface -cne 'social_ai' -or $origin.social_chat.web_chat_provider_id -cne 'chatgpt_web' -or
        $origin.social_chat.interaction_mode -cne 'chat' -or !(Test-WebChatNativeChatSurfaceForeground -Runtime $runtime)) {
        $origin = Open-WebChatNativeChatSurface -Runtime $runtime -ProviderId chatgpt_web -TimeoutSec $TimeoutSec
    }
    $opened = $true; $originPath = [string]$origin.social_chat.web_chat_conversation_path
    $originWeb = Invoke-ChatGptWebSmokeMcp -Runtime $runtime -Tool ui_state
    $originDraft = [string]$originWeb.input.text
    $expectedPath = $originPath
    $preflight = Trial 'state'
    if (!(Test-ChatGptFreshTextIdle $preflight)) { throw 'existing_trial_or_write_pending' }
    if ($resolvedPending) {
        if (!(Test-ChatGptFreshPendingReadback -Pending $resolvedPending -Web $originWeb -Main $origin) -or
            (Get-FileHash -LiteralPath $pendingFile -Algorithm SHA256).Hash -cne $pendingHash) {
            throw 'pending_fresh_fixture_requires_readonly_resolution'
        }
        # Preserve the original handoff before any new fixture can replace it.
        $archive = Join-Path (Split-Path -Parent $pendingFile) ('fresh-text-resolved-' + [Guid]::NewGuid().ToString('N') + '.json')
        Copy-Item -LiteralPath $pendingFile -Destination $archive
        $report.resolved_prior_fixture = $true
    }
    if ($originWeb.input.text -isnot [string] -or $originDraft -cne '' -or
        $origin.input.has_text -isnot [bool] -or $origin.input.has_text -or
        $originWeb.dictation_active -ne $false -or $originWeb.private_voice_native_research.phase -cne 'idle' -or
        $origin.social_chat.web_chat_streaming -isnot [bool] -or $origin.social_chat.web_chat_streaming) {
        throw 'origin_not_idle'
    }
    if ($ExistingProjectFixture) {
        foreach ($candidate in @(Find-ChatGptFreshProjectFixture -Runtime $runtime)) {
            $projectId = [string]$candidate.project_id
            $ownedNavigation = $true; $expectedPath = [string]$candidate.path
            $lastObservedPath = $expectedPath
            Invoke-ChatGptWebSmokeAction -Runtime $runtime -Action open_web_chat_conversation `
                -Arguments @{conversation_path=$expectedPath} | Out-Null
            try {
                Assert-ChatGptFreshProjectFixture -Runtime $runtime -Path $expectedPath -ProjectId $projectId
                $report.project_fixture_verified = $true
                break
            } catch {
                if ($_.Exception.Message -cne 'project_synthetic_fixture_unconfirmed') { throw }
            }
        }
        if (!$report.project_fixture_verified) { throw 'project_synthetic_fixture_unconfirmed' }
    } elseif (!$NewConversation -and (Test-Path -LiteralPath $fixtureFile)) {
        $saved = Get-Content -LiteralPath $fixtureFile -Raw | ConvertFrom-Json
        if ($saved.path -notmatch '^/c/[a-f0-9-]{36}$') { throw 'fixture_path_invalid' }
        $ownedNavigation = $true; $expectedPath = [string]$saved.path
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
        $ownedNavigation = $true; $expectedPath = ''
        Invoke-ChatGptWebSmokeAction -Runtime $runtime -Action start_new_web_chat_conversation | Out-Null
        Wait-ChatGptWebSmokeState -Runtime $runtime -MainState -TimeoutSec $TimeoutSec -Description 'new native text fixture' -Predicate {
            param($state)
            $state.social_chat.web_chat_composer_ready -eq $true -and $state.social_chat.message_count -eq 0 -and
                [string]$state.social_chat.web_chat_conversation_path -eq ''
        } | Out-Null
        if (!$NewConversation) {
            $report.stage = 'seed'; Write-Output 'FRESH_TEXT_STAGE=seed'
            $report.cases += Native-Send 'seed' $false $true
        }
    }
    if ($ComposerUnavailable) {
        $composerLease = Start-ChatGptComposerDomLease -Runtime $runtime
        Wait-ChatGptWebSmokeState -Runtime $runtime -TimeoutSec 20 -Description 'private sender without usable composer' -Predicate {
            param($s) $s.private_send_ready -eq $true
        } | Out-Null
    }
    foreach ($kind in $(if ($StopThenFollowup) { @('stop','followup') } elseif ($OnlyStop) { @('stop') } elseif ($FirstOnly) { @('first') } else { @('first','followup') })) {
        $report.stage = $kind; Write-Output "FRESH_TEXT_STAGE=$kind"
        $report.cases += Native-Send $kind $true ($NewConversation -and $kind -eq 'first')
    }
    if ($BackgroundResume -and !$report.background_resume.exercised) { throw 'background_active_window_not_observed' }
    if ($ExistingProjectFixture) {
        $report.project_membership_verified = Confirm-ChatGptFreshProjectMembership -Runtime $runtime -Path $expectedPath -ProjectId $projectId
    }
    $report.passed = $true; $report.stage = 'complete'
} catch {
    $report.error = if ($_.Exception.Message -match '^[a-z_:]+$') { $_.Exception.Message } else { 'acceptance_failed' }
    if ($_.Exception.Message -match '^Semantic UI acceptance failed: ([a-z_]+)$') { $report.error = $Matches[1] }
    if ($report.error -in @('foreground_package_mismatch','background_launcher_not_owned')) { $report.foreground_changed = $true }
    $report.failure_line = $_.InvocationInfo.ScriptLineNumber
} finally {
    if ($composerLease) {
        try { $report.composer_restored = Stop-ChatGptComposerDomLease -Runtime $runtime -Lease $composerLease }
        catch { $report.composer_restored = $false }
        if (!$report.composer_restored) { $report.passed = $false }
    }
    try {
        if ($report.foreground_changed) {
            $report.cleanup_deferred = $true
        } elseif ($opened -and $ownedNavigation) {
            $end = Trial $(if ($trialRequested) { 'end' } else { 'state' })
            $lastMain = Invoke-ChatGptWebSmokeMcp -Runtime $runtime -Tool ui_state -MainState
            $lastWeb = Invoke-ChatGptWebSmokeMcp -Runtime $runtime -Tool ui_state
            if (Test-ChatGptFreshTextRestoreSafe -AwaitingResult $awaitingResult -Trial $end -Main $lastMain -Web $lastWeb -ExpectedPath $expectedPath) {
                if ($originPath) {
                    $restored = Restore-WebChatNativeConversation -Runtime $runtime -ProviderId chatgpt_web -ConversationPath $originPath
                } elseif ($expectedPath -eq '') { $restored = $true }
                else {
                    Invoke-ChatGptWebSmokeAction -Runtime $runtime -Action start_new_web_chat_conversation | Out-Null
                    Wait-ChatGptWebSmokeState -Runtime $runtime -MainState -TimeoutSec 20 -Description 'original empty chat' -Predicate {
                        param($s) $s.social_chat.message_count -eq 0 -and [string]$s.social_chat.web_chat_conversation_path -eq ''
                    } | Out-Null
                    $restored = $true
                }
                if ($restored) { Invoke-ChatGptWebSmokeAction -Runtime $runtime -Action show_conversation_home | Out-Null }
            } else { $report.cleanup_deferred = $true }
        } elseif ($opened) { $restored = $true }
    } catch { $report.cleanup_deferred = $true }
    finally {
        $awakeRestored = Stop-ChatGptWebSmokeAwakeLease -Runtime $runtime
        if ($awaitingResult) {
            # Local-only controlled fixture receipt survives worktree cleanup. It is not a replay instruction.
            if (!$lastObservedPath) {
                $lastObservedPath = Get-ChatGptFreshPendingObservedPath -Web $lastWeb -Prompt $lastPrompt -UserMessageId $lastUserId
            }
            New-Item -ItemType Directory -Force -Path (Split-Path -Parent $pendingFile) | Out-Null
            @{schema='elon.fresh_text_pending.v1';source='native_fixture';new_conversation=[bool]$NewConversation;
                origin_path=$originPath;prompt=$lastPrompt;user_message_id=$lastUserId;
                project_id=$projectId;expected_path=$expectedPath;
                observed_path=$lastObservedPath;readback_completed=$false;
                click_acknowledged=$clickAcknowledged;trial=$report.last_trial;replay_allowed=$false;
                created_at=[DateTimeOffset]::UtcNow.ToString('o')} | ConvertTo-Json -Depth 6 |
                Set-Content -LiteralPath $pendingFile -Encoding utf8
            $report.pending_fixture_saved = $true
        }
        $report.restored = $restored; $report.awake_restored = [bool]$awakeRestored
        $report.write_unconfirmed = $awaitingResult
        $report | ConvertTo-Json -Depth 5 -Compress
    }
}
if (-not $report.passed -or -not $restored -or -not $awakeRestored) { throw 'fresh_text_acceptance_incomplete' }
