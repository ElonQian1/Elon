#requires -Version 7.0
[CmdletBinding()]
param(
    [string]$Adb = 'D:/Android/sdk/platform-tools/adb.exe',
    [string]$DeviceSerial,
    [string]$ExpectedHardwareSerial,
    [ValidateRange(1,50)][int]$MaxCandidates = 20,
    [switch]$LeaveResolvedOpen,
    [switch]$DefinitionsOnly
)
$ErrorActionPreference = 'Stop'
. (Join-Path $PSScriptRoot 'chatgpt-web-smoke-runtime.ps1')
. (Join-Path $PSScriptRoot 'chatgpt-fresh-trial-smoke.ps1')
. (Join-Path $PSScriptRoot 'chatgpt-fresh-text-smoke-evidence.ps1')

function Test-FreshPendingFixture {
    param($Pending)
    return $Pending.schema -ceq 'elon.fresh_text_pending.v1' -and $Pending.source -ceq 'native_fixture' -and
        $Pending.new_conversation -is [bool] -and $Pending.new_conversation -and
        $Pending.replay_allowed -is [bool] -and !$Pending.replay_allowed -and
        [string]$Pending.user_message_id -cmatch '^[a-f0-9]{8}(?:-[a-f0-9]{4}){3}-[a-f0-9]{12}$' -and
        ($null -eq $Pending.observed_path -or ($Pending.observed_path -is [string] -and
            ($Pending.observed_path -ceq '' -or $Pending.observed_path -cmatch '^/c/[a-f0-9]{8}(?:-[a-f0-9]{4}){3}-[a-f0-9]{12}$'))) -and
        [string]$Pending.prompt -cmatch '^ELON_FRESH_TEXT_ACCEPTANCE_V1 first (?<stamp>\d{13})\. Reply exactly FRESH_FIRST_\k<stamp>\.$'
}

function Select-FreshPendingCandidates {
    param($Pending, [array]$Conversations, [int]$Limit = 20)
    if (!(Test-FreshPendingFixture $Pending)) { throw 'invalid_pending_fixture' }
    # Dates narrow the read-only search; only exact message identity can resolve it.
    $day = ([DateTimeOffset]$Pending.created_at).ToLocalTime().ToString('yyyy-MM-dd')
    $seen = [Collections.Generic.HashSet[string]]::new([StringComparer]::Ordinal)
    $observed = if ($Pending.observed_path) {
        @([pscustomobject]@{id=$Pending.observed_path.Substring(3);path=$Pending.observed_path;project_id=$null})
    } else { @() }
    $ordered = @($observed) + @($Conversations | Where-Object { $day -in $_.activity_dates }) +
        @($Conversations | Where-Object { $day -notin $_.activity_dates })
    $valid = foreach ($row in $ordered) {
        if (!$row.project_id -and [string]$row.id -cmatch '^[a-f0-9]{8}(?:-[a-f0-9]{4}){3}-[a-f0-9]{12}$' -and
            [string]$row.path -ceq ('/c/' + $row.id) -and $seen.Add([string]$row.id)) { $row }
    }
    @($valid | Select-Object -First $Limit)
}

function Get-FreshPendingDirectoryCandidates {
    param($Runtime, $Pending, [Collections.IDictionary]$Report, [int]$Limit)
    $all = @(); $offset = 0
    for ($pageIndex = 0; $pageIndex -lt 8; $pageIndex++) {
        $Report.directory_reads++
        $page = Invoke-ChatGptWebSmokeAction -Runtime $Runtime -Action chatgpt_get_conversations -Arguments @{limit=50;offset=$offset}
        if ($page.control_ok -ne $true -or $page.offset -ne $offset) { throw 'directory_cache_page_invalid' }
        $all += @($page.conversations)
        $Report.cache_has_more = $page.has_more -eq $true
        if (!$page.has_more) { break }
        if ($page.next_offset -le $offset) { throw 'directory_cache_cursor_stalled' }
        $offset = $page.next_offset
    }
    $Report.cached_count = $all.Count
    Select-FreshPendingCandidates $Pending $all $Limit
}

function Get-FreshPendingNextCandidate {
    param($Queue, $Runtime, $Pending, [Collections.IDictionary]$Report, [int]$Limit)
    if ($Queue.index -ge $Limit) { return $null }
    if ($Queue.index -ge $Queue.candidates.Count -and !$Queue.directory_loaded) {
        $seenPaths = @($Queue.candidates | ForEach-Object path)
        $Queue.candidates += @(Get-FreshPendingDirectoryCandidates -Runtime $Runtime -Pending $Pending -Report $Report -Limit $Limit |
            Where-Object { $_.path -cnotin $seenPaths })
        $Queue.directory_loaded = $true
    }
    $Report.candidates = $Queue.candidates.Count
    if ($Queue.index -ge $Queue.candidates.Count) { return $null }
    return $Queue.candidates[$Queue.index++]
}

function Test-FreshPendingReadOnlyIdle {
    param($Web, $Main, $Trial)
    return (Test-ChatGptFreshTextIdle $Trial) -and $Web.authenticated -eq $true -and
        $Web.streaming -is [bool] -and !$Web.streaming -and $Web.dictation_active -eq $false -and
        $Web.private_voice_native_research.phase -ceq 'idle' -and
        $Web.input.text -is [string] -and $Web.input.text -ceq '' -and
        $Main.input.has_text -is [bool] -and !$Main.input.has_text -and
        $Main.social_chat.web_chat_streaming -is [bool] -and !$Main.social_chat.web_chat_streaming
}

function Test-FreshPendingLookupReady {
    param($Web, [string]$ExpectedPath)
    return $ExpectedPath -cmatch '^/c/[a-f0-9]{8}(?:-[a-f0-9]{4}){3}-[a-f0-9]{12}$' -and
        $Web.conversation.url -ceq ('https://chatgpt.com' + $ExpectedPath) -and
        $Web.authenticated -eq $true -and $Web.adapter_current -eq $true -and
        $Web.bridge_state -ceq 'ready' -and $Web.streaming -eq $false -and
        $Web.conversation.message_count -gt 0
}

if ($DefinitionsOnly) { return }
$r = New-ChatGptWebSmokeRuntime -Adb $Adb -DeviceSerial $DeviceSerial -ExpectedHardwareSerial $ExpectedHardwareSerial -PollIntervalSec 1
$r.mcp_bootstrapped = $true
$root = Split-Path -Parent $PSScriptRoot
$commonGit = [IO.Path]::GetFullPath((& git -C $root rev-parse --git-common-dir).Trim(), $root)
$file = Join-Path $commonGit 'ai-acceptance-fixtures/fresh-text-pending.json'
$pending = Get-Content -LiteralPath $file -Raw | ConvertFrom-Json
if (!(Test-FreshPendingFixture $pending)) { throw 'invalid_pending_fixture' }
$hash = (Get-FileHash -LiteralPath $file -Algorithm SHA256).Hash
$origin = ''; $expected = ''; $opened = $false
$report = [ordered]@{schema='elon.fresh_pending_resolution.v1'; matched=$false; cached_count=0; directory_reads=0;
    cache_has_more=$false; candidates=0; inspected=0; exact_user_seen=$false; restored=$false;
    left_resolved_open=$false; awake_restored=$false; sends=0}
Assert-ChatGptWebSmokeTrustedDevice -Runtime $r
Start-ChatGptWebSmokeAwakeLease -Runtime $r | Out-Null
try {
    $main = Open-WebChatNativeChatSurface -Runtime $r -ProviderId chatgpt_web -TimeoutSec 30
    $origin = [string]$main.social_chat.web_chat_conversation_path; $expected = $origin; $opened = $true
    $web = Invoke-ChatGptWebSmokeMcp -Runtime $r -Tool ui_state
    $trial = Invoke-ChatGptFreshTrial -Runtime $r -Mode state
    if (!(Test-FreshPendingReadOnlyIdle $web $main $trial)) { throw 'pending_resolution_not_idle' }
    $queue = @{candidates=@(Select-FreshPendingCandidates $pending @() $MaxCandidates);index=0;directory_loaded=$false}
    while ($report.inspected -lt $MaxCandidates) {
        $candidate = Get-FreshPendingNextCandidate -Queue $queue -Runtime $r -Pending $pending -Report $report -Limit $MaxCandidates
        if ($null -eq $candidate) { break }
        $main = Invoke-ChatGptWebSmokeMcp -Runtime $r -Tool ui_state -MainState
        $web = Invoke-ChatGptWebSmokeMcp -Runtime $r -Tool ui_state
        $trial = Invoke-ChatGptFreshTrial -Runtime $r -Mode state
        if (!(Test-FreshPendingReadOnlyIdle $web $main $trial) -or
            [string]$main.social_chat.web_chat_conversation_path -cne $expected) {
            $report.context = [ordered]@{native_surface=($main.active_surface -ceq 'social_ai');
                native_route=([string]$main.social_chat.web_chat_conversation_path -ceq $expected);
                bridge_ready=($web.bridge_state -ceq 'ready');adapter_current=($web.adapter_current -eq $true);
                authenticated=($web.authenticated -eq $true);trial_idle=(Test-ChatGptFreshTextIdle $trial);
                idle=(Test-FreshPendingReadOnlyIdle $web $main $trial)}
            throw 'pending_resolution_context_changed'
        }
        $expected = [string]$candidate.path
        Invoke-ChatGptWebSmokeAction -Runtime $r -Action open_web_chat_conversation -Arguments @{conversation_path=$expected} | Out-Null
        $web = Wait-ChatGptWebSmokeState -Runtime $r -TimeoutSec 15 -Description 'read-only pending fixture lookup' -Predicate {
            param($s) Test-FreshPendingLookupReady $s $expected
        }
        Wait-ChatGptWebSmokeState -Runtime $r -TimeoutSec 15 -MainState -Description 'native pending lookup route' -Predicate {
            param($s) $s.active_surface -ceq 'social_ai' -and
                $s.social_chat.web_chat_provider_id -ceq 'chatgpt_web' -and
                $s.social_chat.web_chat_conversation_path -ceq $expected -and
                $s.social_chat.web_chat_streaming -eq $false
        } | Out-Null
        $report.inspected++
        $exact = @($web.conversation.messages | Where-Object { $_.role -ceq 'user' -and
            $_.id -ceq $pending.user_message_id -and $_.content -ceq $pending.prompt }).Count -eq 1
        Write-Host "PENDING_LOOKUP inspected=$($report.inspected) exact_user=$exact"
        if (!$exact) { continue }
        $report.exact_user_seen = $true
        $candidateProof = $pending | ConvertTo-Json -Depth 20 | ConvertFrom-Json
        $candidateProof | Add-Member resolved_path $expected -Force
        $candidateProof | Add-Member readback_completed $true -Force
        $readbackUntil = [DateTimeOffset]::UtcNow.AddSeconds(15)
        do {
            $web = Invoke-ChatGptWebSmokeMcp -Runtime $r -Tool ui_state
            $main = Invoke-ChatGptWebSmokeMcp -Runtime $r -Tool ui_state -MainState
            $verified = Test-ChatGptFreshPendingReadback $candidateProof $web $main
            if ($verified) { break }
            Start-Sleep -Milliseconds 300
        } while ([DateTimeOffset]::UtcNow -lt $readbackUntil)
        if (!$verified) { throw 'pending_native_readback_incomplete' }
        if ((Get-FileHash -LiteralPath $file -Algorithm SHA256).Hash -cne $hash) { throw 'pending_handoff_changed' }
        $candidateProof | ConvertTo-Json -Depth 20 | Set-Content -LiteralPath $file -Encoding utf8
        $report.matched = $true
        break
    }
} catch {
    $report.error = if ($_.Exception.Message -cmatch '^[a-z_]+$') { $_.Exception.Message } else { 'readonly_resolution_failed' }
    $report.failure_line = $_.InvocationInfo.ScriptLineNumber
} finally {
    try {
        if ($opened) {
            $main = Invoke-ChatGptWebSmokeMcp -Runtime $r -Tool ui_state -MainState
            $web = Invoke-ChatGptWebSmokeMcp -Runtime $r -Tool ui_state
            $trial = Invoke-ChatGptFreshTrial -Runtime $r -Mode state
            if ((Test-FreshPendingReadOnlyIdle $web $main $trial) -and
                [string]$main.social_chat.web_chat_conversation_path -ceq $expected) {
                if ($report.matched -and $LeaveResolvedOpen) { $report.left_resolved_open = $true }
                elseif ($origin -ceq $expected) { $report.restored = $true }
                elseif ($origin) { $report.restored = Restore-WebChatNativeConversation -Runtime $r -ProviderId chatgpt_web -ConversationPath $origin }
            }
        }
    } catch { $report.restoration_error = 'restoration_unconfirmed'
    } finally { $report.awake_restored = Stop-ChatGptWebSmokeAwakeLease -Runtime $r }
    Write-Output ($report | ConvertTo-Json -Compress -Depth 5)
}
if (!$report.matched) { exit 1 }
