#requires -Version 7.0
[CmdletBinding()]
param(
    [Parameter(Mandatory)][string]$DeviceSerial,
    [Parameter(Mandatory)][string]$ExpectedHardwareSerial,
    [switch]$ReuseFixture,
    [switch]$ExistingProjectFixture,
    [switch]$ProjectAttachmentControl,
    [switch]$FreshProjectFixture,
    [string]$FixtureCheckpoint = '',
    [string]$Adb = 'D:/Android/sdk/platform-tools/adb.exe'
)
$ErrorActionPreference = 'Stop'
if ($ProjectAttachmentControl -and -not $ExistingProjectFixture) {throw 'project_control_requires_existing_fixture'}
if ($FreshProjectFixture -and (-not $ExistingProjectFixture -or $ProjectAttachmentControl)) {throw 'invalid_fresh_project_fixture_mode'}
. (Join-Path $PSScriptRoot 'chatgpt-web-smoke-runtime.ps1')
. (Join-Path $PSScriptRoot 'chatgpt-web-smoke-evidence.ps1')
. (Join-Path $PSScriptRoot 'invoke-android-semantic-acceptance.ps1')
$r = New-ChatGptWebSmokeRuntime -Adb $Adb -DeviceSerial $DeviceSerial -ExpectedHardwareSerial $ExpectedHardwareSerial
$fixture = 'fixed_ascii_text_v1'
$name = 'elon-chatgpt-attachment-fixture-v1.txt'
$expectedHash = '75e2ed9bfe5772c9918e552ed07c2c0e689e7039367c81bb6906c63e396fa1f3'
$checkpoint=Join-Path (Split-Path -Parent $PSScriptRoot) '.ai-tmp/citation-download-fixture.json'
if ($FreshProjectFixture) {$checkpoint=Join-Path (Split-Path -Parent $PSScriptRoot) '.ai-tmp/project-citation-download-fixture.json'}
if ($FixtureCheckpoint) {$checkpoint=[IO.Path]::GetFullPath($FixtureCheckpoint)}
$prompt = 'Read the attached text file. Quote its exact first line and cite the uploaded file using a file citation in your answer. Do not create, copy, or modify any file.'
if ($ExistingProjectFixture -and -not $FreshProjectFixture) {
    $prompt = 'Use the previously uploaded elon-chatgpt-attachment-fixture-v1.txt. Quote its exact first line and cite that uploaded file using a file citation. Do not create, copy, or modify any file.'
}
$report = [ordered]@{schema='elon.chatgpt.citation_download_ui.v1'; passed=$false; stage='prepare'
    restored=$false; awake_restored=$false; content_exported=$false; send_attempts=0; download_attempts=0}
$origin=$null; $navigated=$false; $menuOpen=$false; $detailOpen=$false; $downloadOpen=$false; $probe=$false
$verifiedProjectPath=''; $verifiedProjectId=''
function Native { Get-ChatGptWebNativeChatState -Runtime $r }
function Web {
    if (-not (Test-WebChatNativeChatSurfaceForeground -Runtime $r)) { throw 'foreground_changed' }
    Invoke-ChatGptWebSmokeMcp -Runtime $r -Tool ui_state
}
function Act([string]$Action, [hashtable]$Arguments=@{}) {
    $result=Invoke-ChatGptWebSmokeAction -Runtime $r -Action $Action -Arguments $Arguments
    if ($result.control_ok -eq $false) { throw 'native_action_rejected' }
    $result
}
function Command([string]$Action, [hashtable]$Arguments, [string]$Expected) {
    $sent=Act $Action $Arguments
    Wait-ChatGptCommandReceipt -InvokeUiState { Web } -RequestId $sent.command_receipt.request_id `
        -ExpectedAction $Expected -TimeoutSec 25 -PollIntervalSec 1
}
function Ui([string]$Step, [hashtable]$Parameters=@{}, [switch]$Download) {
    $report.ui_step=$Step
    $class=if ($Download) {'LibraryUiAcceptance'} else {'ConversationUiAcceptance'}
    $prefix=if ($Download) {'LIBRARY_UI_RESULT'} else {'CONVERSATION_UI_RESULT'}
    Invoke-AndroidSemanticAcceptance -Runtime $r -TestClass $class -Step $Step -Parameters $Parameters -ResultPrefix $prefix
}
function Saved {
    $raw=Invoke-ChatGptWebSmokeAdb -Runtime $r -Arguments @('shell',
        "find /sdcard/Download -maxdepth 1 -type f -name 'elon-*-$name'") -TimeoutSec 6
    @($raw -split "\r?\n" | Where-Object {$_ -cmatch ('^/sdcard/Download/elon-[A-Za-z0-9_-]+-'+[regex]::Escape($name)+'$')})
}
function Confirm-Saved([string]$Path) {
    # Verify fixed fixture bytes on the handset; never pull private files.
    $raw=Invoke-ChatGptWebSmokeAdb -Runtime $r -Arguments @('shell','sha256sum',$Path) -TimeoutSec 6
    if (($raw -split '\s+')[0] -cne $expectedHash) {throw 'saved_fixture_hash_mismatch'}
    $size=Invoke-ChatGptWebSmokeAdb -Runtime $r -Arguments @('shell','stat','-c','%s',$Path) -TimeoutSec 6
    if ($size.Trim() -cne '78') {throw 'saved_fixture_size_mismatch'}
    @{bytes=78;sha256=$expectedHash;hash_verified=$true;download_retained=$true}
}
function Open-ExistingFixture {
    if (-not (Test-Path -LiteralPath $checkpoint)) {throw 'fixture_checkpoint_unavailable'}
    $saved=Get-Content -LiteralPath $checkpoint -Raw | ConvertFrom-Json
    $validPath=if ($FreshProjectFixture) {
        $saved.path -cmatch '^/g/g-p-[a-f0-9]{32}(?:-[A-Za-z0-9_-]{1,124})?/c/[a-f0-9]{8}(?:-[a-f0-9]{4}){3}-[a-f0-9]{12}$'
    } else {$saved.path -cmatch '^/c/[A-Za-z0-9_-]{1,160}$'}
    if ($saved.schema -ne 'elon.citation_fixture.v1' -or $saved.device -cne $ExpectedHardwareSerial -or
        -not $validPath) {throw 'fixture_checkpoint_invalid'}
    Command 'chatgpt_open_conversation' @{conversation_path=$saved.path} 'open_conversation' | Out-Null
    $deadline=[DateTimeOffset]::UtcNow.AddSeconds(20)
    do {
        $s=Native; $w=Web
        $users=@($s.social_chat.messages | Where-Object role -eq user)
        $matchesPrompt=$users.Count -eq 1 -and ([string]$users[0].content).Contains($prompt,[StringComparison]::Ordinal)
        if ($s.social_chat.web_chat_conversation_path -eq $saved.path -and
            ([uri]$w.conversation.url).AbsolutePath -eq $saved.path -and $matchesPrompt -and
            -not $s.input.text -and -not $w.streaming -and -not $w.dictation_active -and [int]$w.input.official_draft_length -eq 0) {return}
        Start-Sleep -Milliseconds 500
    } while ([DateTimeOffset]::UtcNow -lt $deadline)
    throw 'existing_synthetic_fixture_unavailable'
}
function Open-ProjectFixture {
    $mediaPrompt = 'Read all three attached test files. Reply in English: quote the exact first line from each document, then describe the shapes in the image, including their counts and colors. If an attachment is unavailable, say so instead of guessing.'
    $completedProbes=@(
        'Use the previously uploaded elon-chatgpt-attachment-fixture-v1.txt. Quote its exact first line and cite that uploaded file using a file citation. Do not create, copy, or modify any file.',
        'Reply exactly ELON_PROJECT_RUNTIME_337_OK. Do not use tools or modify files.',
        'Reply exactly ELON337READY. Do not use tools or modify files.'
    )
    # A cached row is a locator only; live path, fixed messages and file index
    # below establish the target before any write or download.
    $candidate=$null
    foreach ($offset in @(0,50,100,150)) {
        $page=Act 'chatgpt_get_conversations' @{offset=$offset;limit=50}
        $candidate=$page.conversations | Where-Object {
            $_.project_id -cmatch '^g-p-[a-f0-9]{32}$' -and $_.title -match '(?i)fixture|attachment|test|media|file'
        } | Select-Object -First 1
        if ($candidate -or -not $page.has_more) {break}
    }
    $report.cached_directory_candidate=$page.stale -eq $true
    if (-not $candidate -or $candidate.path -cnotmatch ('^/g/'+[regex]::Escape($candidate.project_id)+
        '(?:-[A-Za-z0-9_-]{1,124})?/c/[a-f0-9]{8}(?:-[a-f0-9]{4}){3}-[a-f0-9]{12}$')) {
        throw 'project_fixture_candidate_unavailable'
    }
    if ($candidate.project_path -cnotmatch ('^/g/'+[regex]::Escape($candidate.project_id)+'(?:-[A-Za-z0-9_-]{1,124})?/project$')) {throw 'project_fixture_path_unconfirmed'}
    $script:verifiedProjectPath=$candidate.project_path
    $script:verifiedProjectId=$candidate.project_id
    Command 'chatgpt_open_conversation' @{conversation_path=$candidate.path} 'open_conversation' | Out-Null
    $deadline=[DateTimeOffset]::UtcNow.AddSeconds(25)
    do {
        $s=Native; $w=Web
        $users=@($s.social_chat.messages | Where-Object role -eq user)
        $media=@($users | Where-Object {([string]$_.content).Contains($mediaPrompt,[StringComparison]::Ordinal)})
        $other=@($users | Where-Object {
            $body=[string]$_.content
            -not $body.Contains($mediaPrompt,[StringComparison]::Ordinal) -and
                -not $body.Contains($prompt,[StringComparison]::Ordinal) -and
                -not @($completedProbes | Where-Object {$body.Contains($_,[StringComparison]::Ordinal)}).Count
        })
        $ready=$s.social_chat.web_chat_conversation_path -eq $candidate.path -and
            ([uri]$w.conversation.url).AbsolutePath -eq $candidate.path -and $media.Count -eq 1 -and
            $other.Count -eq 0 -and -not $s.input.text -and -not $w.streaming -and
            -not $w.dictation_active -and [int]$w.input.official_draft_length -eq 0
        if ($ready) {break}
        Start-Sleep -Milliseconds 500
    } while ([DateTimeOffset]::UtcNow -lt $deadline)
    if (-not $ready) {throw 'project_synthetic_fixture_unconfirmed'}
    Command 'chatgpt_list_conversation_files' @{conversation_path=$candidate.path} 'list_conversation_files' | Out-Null
    $index=(Web).conversation_files
    if ($index.stale -or $index.conversation_path -ne $candidate.path -or
        @($index.files | Where-Object {$_.role -eq 'user' -and $_.name -ceq $name}).Count -lt 1) {
        throw 'project_fixture_attachment_unconfirmed'
    }
    $report.project_fixture_verified=$true
    $report.existing_fixture_reused=$true
    if ($FreshProjectFixture) {return}
    if ($ProjectAttachmentControl) {return}
    $citation=@($index.files | Where-Object {$_.role -eq 'assistant' -and $_.name -ceq $name -and $_.download_handle})
    if ($citation.Count -gt 0) {return}
    # A fixed follow-up is sent at most once; an uncertain result is never replayed.
    if ($users.Count -ne 1) {throw 'project_fixture_prior_send_unconfirmed'}
    $assistantBefore=@($s.social_chat.messages | Where-Object role -eq friend).Count
    Act 'set_input_text' @{text=$prompt} | Out-Null
    $report.stage='project_citation_followup'; $report.send_attempts=1
    $report.synthetic_remote_artifacts_may_remain=$true
    Act 'send_input' | Out-Null
    $deadline=[DateTimeOffset]::UtcNow.AddSeconds(100)
    do {
        $s=Native; $w=Web
        $assistant=@($s.social_chat.messages | Where-Object role -eq friend)
        $last=([string]($assistant | Select-Object -Last 1).content).Replace('\_', '_').Replace('**', '').Replace('`', '')
        $done=$assistant.Count -gt $assistantBefore -and $last.Contains('ELON_CHATGPT_ATTACHMENT_FIXTURE_V1=ready') -and
            -not $w.streaming -and -not $s.social_chat.web_chat_streaming -and -not $s.input.text
        if ($done) {break}
        Start-Sleep -Seconds 1
    } while ([DateTimeOffset]::UtcNow -lt $deadline)
    $report.runtime_send=$s.social_chat.web_chat_last_send_command.detail -eq 'official_runtime_v1:accepted'
    $report.user_rows=@($s.social_chat.messages | Where-Object role -eq user).Count
    if (-not $done -or -not $report.runtime_send -or $report.user_rows -ne 2 -or
        $s.social_chat.web_chat_conversation_path -ne $candidate.path) {throw 'project_citation_reply_unconfirmed'}
}
try {
    Assert-ChatGptWebSmokeTrustedDevice -Runtime $r
    if (-not (Get-ChatGptWebSmokeUserReadiness -Runtime $r).ready) {throw 'device_locked'}
    $origin=Native; $before=Web
    if ($origin.active_surface -ne 'social_ai' -or $origin.social_chat.web_chat_provider_id -ne 'chatgpt_web' -or
        -not $before.authenticated -or -not $before.adapter_current) {throw 'surface_not_ready'}
    if ($origin.input.text -or $before.streaming -or $before.dictation_active -or
        [int]$before.input.official_draft_length -gt 0 -or $before.file_download.can_cancel -or
        [int]$origin.social_chat.web_chat_pending_attachment_count -gt 0) {throw 'existing_work_in_progress'}
    if ($fixture -notin $origin.chatgpt_web_acceptance_attachment.supported_fixture_ids) {throw 'fixture_not_supported'}
    if ($FreshProjectFixture -and -not $ReuseFixture -and (Test-Path -LiteralPath $checkpoint)) {throw 'fresh_fixture_exists_use_reuse'}
    Start-ChatGptWebSmokeAwakeLease -Runtime $r | Out-Null
    $report.adapter=$before.adapter_version
    $navigated=$true
    if ($ExistingProjectFixture -and -not $FreshProjectFixture) {
        $report.stage='reuse_project_fixture'
        Open-ProjectFixture
    } elseif ($ReuseFixture) {
        $report.stage='reuse_existing_fixture'
        Open-ExistingFixture
        $report.existing_fixture_reused=$true
    } else {
    if ($FreshProjectFixture) {
        Open-ProjectFixture
        Act 'open_web_chat_project' @{project_path=$verifiedProjectPath} | Out-Null
    } else {Act 'start_new_web_chat_conversation' | Out-Null}
    $deadline=[DateTimeOffset]::UtcNow.AddSeconds(30)
    do {
        $s=Native; $w=Web
        # The website canonicalizes the display slug; project identity must match,
        # not the mutable title in the cached route.
        $blankPath=if ($FreshProjectFixture) {$verifiedProjectPath} else {'/'}
        $url=[uri]$w.conversation.url
        $blankRoute=if ($FreshProjectFixture) {
            $url.AbsolutePath -cmatch ('^/g/'+[regex]::Escape($verifiedProjectId)+'(?:-[A-Za-z0-9_-]{1,124})?/project$')
        } else {$url.AbsolutePath -ceq '/'}
        $blank=$url.GetLeftPart([UriPartial]::Authority) -ceq 'https://chatgpt.com' -and
            -not $url.Query -and -not $url.Fragment -and $blankRoute -and $w.composer_ready -and
            @($s.social_chat.messages).Count -eq 0 -and -not $s.input.text -and [int]$w.input.official_draft_length -eq 0
        if ($blank) {break}
        Start-Sleep -Milliseconds 500
    } while ([DateTimeOffset]::UtcNow -lt $deadline)
    if (-not $blank) {
        $report.blank_state=@{web_path_matches=([uri]$w.conversation.url).AbsolutePath -eq $blankPath
            native_path_matches=$s.social_chat.web_chat_conversation_path -eq $blankPath
            composer_ready=$w.composer_ready;native_messages=@($s.social_chat.messages).Count
            native_draft_present=[bool]$s.input.text;web_draft_present=[int]$w.input.official_draft_length -gt 0}
        throw 'blank_chat_not_ready'
    }
    $temporary=$w.ui_manifest.controls | Where-Object semantic -eq temporary_chat | Select-Object -First 1
    if (-not $FreshProjectFixture -and ($null -eq $temporary -or $temporary.selected -ne $false)) {throw 'ordinary_chat_required'}
    $report.stage='stage_fixture'
    Act 'stage_chatgpt_web_acceptance_attachment' @{fixture_id=$fixture} | Out-Null
    $s=Native
    if (-not $s.chatgpt_web_acceptance_attachment.fixture_staged -or
        [int]$s.chatgpt_web_acceptance_attachment.composer_pending_count -ne 1) {throw 'fixture_not_staged'}
    Act 'set_input_text' @{text=$prompt} | Out-Null
    $since=[DateTimeOffset]::UtcNow.ToUnixTimeMilliseconds()
    $report.stage='send_once'
    $report.synthetic_remote_artifacts_may_remain=$true
    $report.send_attempts=1
    Act 'send_input' | Out-Null
    $deadline=[DateTimeOffset]::UtcNow.AddSeconds(120)
    do {
        $s=Native; $w=Web
        $reply=@($s.social_chat.messages | Where-Object role -eq friend | ForEach-Object content) -join "`n"
        $plain=$reply.Replace('\_', '_').Replace('**', '').Replace('`', '')
        $read=$plain.Contains('ELON_CHATGPT_ATTACHMENT_FIXTURE_V1=ready')
        $done=$read -and -not $s.social_chat.web_chat_streaming -and -not $w.streaming -and
            $s.social_chat.web_chat_attachment_phase -eq 'completed'
        if ($done -or $s.social_chat.web_chat_attachment_phase -eq 'failed') {break}
        Start-Sleep -Seconds 1
    } while ([DateTimeOffset]::UtcNow -lt $deadline)
    $upload=$s.chatgpt_web_mcp.last_attachment_upload
    $report.private_upload=$upload.ok -and $upload.detail -eq 'private_attachment_associated' -and $upload.observed_at_ms -ge $since
    $report.runtime_send=$s.social_chat.web_chat_last_send_command.detail -eq 'official_runtime_v1:accepted'
    $report.content_read=$read
    $report.citation_marker=$reply.Contains('filecite')
    $report.user_rows=@($s.social_chat.messages | Where-Object role -eq user).Count
    $report.response_ms=[DateTimeOffset]::UtcNow.ToUnixTimeMilliseconds()-$since
    if (-not $done -or -not $report.private_upload -or $report.user_rows -ne 1) {throw 'fixture_reply_unconfirmed'}
    if ($FreshProjectFixture -and $s.social_chat.web_chat_conversation_path -cnotmatch ('^/g/'+[regex]::Escape($verifiedProjectId)+'(?:-[A-Za-z0-9_-]{1,124})?/c/')) {throw 'fresh_project_membership_unconfirmed'}
    }
    $s=Native
    $path=$s.social_chat.web_chat_conversation_path
    if ($path -notmatch '^(/g/g-p-[a-f0-9]{32}(?:-[A-Za-z0-9_-]{1,124})?)?/c/[A-Za-z0-9_-]{1,160}$') {throw 'fixture_conversation_unconfirmed'}
    if (-not $ReuseFixture -and (-not $ExistingProjectFixture -or $FreshProjectFixture)) {
        # Local navigation checkpoint only; never emit it in logs or retry the send.
        New-Item -ItemType Directory -Force -Path (Split-Path -Parent $checkpoint) | Out-Null
        [IO.File]::WriteAllText($checkpoint,(@{schema='elon.citation_fixture.v1';device=$ExpectedHardwareSerial;path=$path} | ConvertTo-Json -Compress))
    }
    $report.stage='native_file_index'
    $menuOpen=$true
    Ui 'header' | Out-Null
    Ui 'current_settings' | Out-Null
    Ui 'files' | Out-Null
    Ui 'files_refresh' | Out-Null
    Ui 'files_wait' | Out-Null
    $deadline=[DateTimeOffset]::UtcNow.AddSeconds(20)
    do {
        $w=Web; $index=$w.conversation_files
        if ($index -and -not $index.stale -and $index.conversation_path -eq $path) {break}
        Start-Sleep -Milliseconds 500
    } while ([DateTimeOffset]::UtcNow -lt $deadline)
    if (-not $index -or $index.stale -or $index.conversation_path -ne $path) {throw 'file_index_unconfirmed'}
    $files=@($index.files)
    $report.index_files=$files.Count
    $report.assistant_files=@($files | Where-Object role -eq assistant).Count
    $selectedRole=if ($ProjectAttachmentControl) {'user'} else {'assistant'}
    $report.download_role=$selectedRole
    $positions=@(for ($i=0;$i -lt $files.Count;$i++) {
        if ($files[$i].role -eq $selectedRole -and $files[$i].name -ceq $name -and $files[$i].download_handle) {$i}
    })
    if ($positions.Count -ne 1) {throw $(if ($ProjectAttachmentControl) {'project_attachment_control_unavailable'} else {'assistant_fixture_citation_unavailable'})}
    $savedBefore=@(Saved)
    Command 'chatgpt_private_protocol_probe' @{mode='start'} 'private_protocol_probe' | Out-Null
    $probe=$true
    $idsBefore=@((Web).command_requests | ForEach-Object request_id)
    Ui 'file_row' @{file_index=$positions[0]} | Out-Null
    $menuOpen=$false; $detailOpen=$true
    $report.stage='native_download_once'
    $report.download_attempts=1; $downloadOpen=$true
    Ui 'download' -Download | Out-Null
    $detailOpen=$false
    $result=Ui 'wait_download' -Download
    $w=Web
    $receipt=@($w.command_requests | Where-Object {
        $_.request_id -notin $idsBefore -and $_.expected_web_action -eq 'download_conversation_file'
    }) | Select-Object -Last 1
    if ($receipt.status -ne 'succeeded' -or $receipt.result.detail -ne 'download_saved' -or -not $result.download_saved) {throw 'download_save_unconfirmed'}
    $created=@(Saved | Where-Object {$_ -notin $savedBefore})
    if ($created.Count -ne 1) {throw 'saved_fixture_not_unique'}
    $report.storage=Confirm-Saved $created[0]
    $report.native_download_saved=$true
    Ui 'close_download' -Download | Out-Null
    $downloadOpen=$false
    $report.stage='complete'; $report.passed=$true
} catch {
    $message=$_.Exception.Message
    $report.error=if ($message -cmatch '^[a-z_]+$') {$message}
        elseif ($message -match '^Semantic UI acceptance failed: ([a-z_]+)$') {$Matches[1]} else {'citation_acceptance_failed'}
    if ($report.download_attempts -gt 0) {
        try {
            $downloadState=Web
            $report.download_job=$downloadState.file_download | Select-Object state,received_bytes,total_bytes,can_cancel
            $failed=@($downloadState.command_requests | Where-Object {
                $_.request_id -notin $idsBefore -and $_.expected_web_action -eq 'download_conversation_file'
            }) | Select-Object -Last 1
            $report.download_status=$failed.status
            $report.download_detail=if ($failed.result.detail -cmatch '^[a-z_]+$') {$failed.result.detail} else {'other'}
            $source=Command 'chatgpt_private_protocol_probe' @{mode='file_download_source'} 'private_protocol_probe'
            $report.download_source=($source.receipt.result.detail | ConvertFrom-Json) |
                Select-Object schema,observed,origin,path,binding,query_count
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
        if ($downloadOpen) {Ui 'close_download' -Download | Out-Null}
        if ($detailOpen) {Ui 'close_detail' -Download | Out-Null}
        if ($menuOpen) {Ui 'back' | Out-Null}
        if ($navigated) {
            $s=Native; $w=Web
            if ($w.streaming -or $w.dictation_active -or $w.file_download.can_cancel) {throw 'operation_still_active'}
            if ($s.input.text -and $s.input.text -cne $prompt) {throw 'changed_draft_preserved'}
            if ($s.chatgpt_web_acceptance_attachment.fixture_staged) {Act 'remove_chatgpt_web_acceptance_attachment' @{fixture_id=$fixture} | Out-Null}
            if ($s.input.text -ceq $prompt) {Act 'set_input_text' @{text=''} | Out-Null}
            if ($origin.social_chat.web_chat_conversation_path) {
                Command 'chatgpt_open_conversation' @{conversation_path=$origin.social_chat.web_chat_conversation_path} 'open_conversation' | Out-Null
            } else {Act 'start_new_web_chat_conversation' | Out-Null}
        }
        if ($origin) {
            $deadline=[DateTimeOffset]::UtcNow.AddSeconds(20)
            do {
                $s=Native; $w=Web
                $report.restored=$s.social_chat.web_chat_conversation_path -eq $origin.social_chat.web_chat_conversation_path -and
                    $s.input.text -ceq $origin.input.text -and @($s.social_chat.messages).Count -eq @($origin.social_chat.messages).Count -and
                    [int]$w.input.official_draft_length -eq 0
                if ($report.restored) {break}
                Start-Sleep -Milliseconds 500
            } while ([DateTimeOffset]::UtcNow -lt $deadline)
        }
    } catch {$report.restored=$false}
    try {$report.awake_restored=Stop-ChatGptWebSmokeAwakeLease -Runtime $r} catch {$report.awake_restored=$false}
    $report.passed=$report.passed -and $report.restored -and $report.awake_restored
    $report | ConvertTo-Json -Depth 6 -Compress
}
if (-not $report.passed) {exit 1}
