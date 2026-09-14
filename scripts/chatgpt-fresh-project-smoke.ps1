#requires -Version 7.0

function Test-ChatGptFreshProjectPath {
    param([string]$Path, [string]$ProjectId)
    return $ProjectId -cmatch '^g-p-[a-f0-9]{32}$' -and $Path -cmatch
        ('^/g/' + [regex]::Escape($ProjectId) + '(?:-[A-Za-z0-9_-]{1,124})?/c/[a-f0-9]{8}(?:-[a-f0-9]{4}){3}-[a-f0-9]{12}$')
}

function Test-ChatGptFreshProjectFixtureUsers {
    param([object[]]$Messages)
    $media = 'Read all three attached test files. Reply in English: quote the exact first line from each document, then describe the shapes in the image, including their counts and colors. If an attachment is unavailable, say so instead of guessing.'
    $allowed = @($media,
        'Use the previously uploaded elon-chatgpt-attachment-fixture-v1.txt. Quote its exact first line and cite that uploaded file using a file citation. Do not create, copy, or modify any file.',
        'Read the attached text file. Quote its exact first line and cite the uploaded file using a file citation in your answer. Do not create, copy, or modify any file.',
        'Reply exactly ELON_PROJECT_RUNTIME_337_OK. Do not use tools or modify files.',
        'Reply exactly ELON337READY. Do not use tools or modify files.')
    $users = @($Messages | Where-Object role -CEQ user)
    if (!$users.Count -or @($users | Where-Object content -CEQ $media).Count -ne 1) { return $false }
    foreach ($user in $users) {
        if ($user.content -isnot [string]) { return $false }
        if ($user.content -cnotin $allowed) {
            if ($user.content -cnotmatch '^ELON_FRESH_TEXT_ACCEPTANCE_V1 (?<kind>first|followup) (?<stamp>[0-9]{13})\. Reply exactly FRESH_(?<marker>FIRST|FOLLOWUP)_\k<stamp>\.$' -or
                $Matches.kind.ToUpperInvariant() -cne $Matches.marker -or
                @($users | Where-Object content -CEQ $user.content).Count -ne 1) { return $false }
        }
    }
    return $true
}

function Find-ChatGptFreshProjectFixture {
    param([Parameter(Mandatory)]$Runtime)
    $candidates = @()
    foreach ($offset in @(0,50,100,150)) {
        $page = Invoke-ChatGptWebSmokeAction -Runtime $Runtime -Action chatgpt_get_conversations -Arguments @{offset=$offset;limit=50}
        if ($page.control_ok -cne $true) { throw 'project_directory_unavailable' }
        $candidates += @($page.conversations | Where-Object {
            (Test-ChatGptFreshProjectPath $_.path $_.project_id) -and $_.title -match '(?i)fixture|attachment|test|media|file'
        })
        if ($candidates.Count -ge 8) { break }
        if (!$page.has_more) { break }
    }
    if (!$candidates.Count) { throw 'project_fixture_candidate_unavailable' }
    return @($candidates | Select-Object -First 8)
}

function Test-ChatGptFreshProjectMembershipReceipt {
    param($Before, $After)
    $previous = if ($null -eq $Before) { 0L } else { $Before.observed_at_ms }
    return ($previous -is [long] -or $previous -is [int]) -and
        ($After.observed_at_ms -is [long] -or $After.observed_at_ms -is [int]) -and
        $After.action -ceq 'probe_conversation_project' -and $After.ok -is [bool] -and $After.ok -and
        $After.observed_at_ms -gt $previous
}

function Confirm-ChatGptFreshProjectMembership {
    param([Parameter(Mandatory)]$Runtime, [string]$Path, [string]$ProjectId)
    if (!(Test-ChatGptFreshProjectPath $Path $ProjectId)) { throw 'project_path_invalid' }
    $before = (Invoke-ChatGptWebSmokeMcp -Runtime $Runtime -Tool ui_state).last_project_membership_probe
    $action = Invoke-ChatGptWebSmokeAction -Runtime $Runtime -Action refresh_web_chat_conversations `
        -Arguments @{project_id=$ProjectId;conversation_path=$Path}
    if ($action.control_ok -cne $true) { throw 'project_membership_request_rejected' }
    $deadline = [DateTimeOffset]::UtcNow.AddSeconds(25)
    do {
        $web = Invoke-ChatGptWebSmokeMcp -Runtime $Runtime -Tool ui_state
        $probe = $web.last_project_membership_probe
        if ($web.conversation.url -cne ('https://chatgpt.com' + $Path)) { throw 'project_context_changed' }
        if (Test-ChatGptFreshProjectMembershipReceipt $before $probe) { return $true }
        Start-Sleep -Milliseconds 500
    } while ([DateTimeOffset]::UtcNow -lt $deadline)
    throw 'project_membership_unconfirmed'
}

function Test-ChatGptFreshProjectFiles {
    param($Receipt, $Index, [string]$Path)
    return $Receipt.status -ceq 'succeeded' -and $Receipt.result.ok -is [bool] -and $Receipt.result.ok -and
        $Index.stale -is [bool] -and !$Index.stale -and $Index.conversation_path -ceq $Path -and
        @($Index.files | Where-Object {$_.role -ceq 'user' -and $_.name -ceq 'elon-chatgpt-attachment-fixture-v1.txt'}).Count -gt 0
}

function Test-ChatGptFreshProjectFallbackReadback {
    param($Pending, $Web, $Main)
    if ($Pending.schema -cne 'elon.fresh_text_pending.v1' -or $Pending.source -cne 'native_fixture' -or
        $Pending.new_conversation -isnot [bool] -or $Pending.new_conversation -or
        $Pending.replay_allowed -isnot [bool] -or $Pending.replay_allowed -or
        !(Test-ChatGptFreshProjectPath $Pending.expected_path $Pending.project_id) -or
        $Pending.prompt -cnotmatch '^ELON_FRESH_TEXT_ACCEPTANCE_V1 first (?<stamp>[0-9]{13})\. Reply exactly FRESH_FIRST_\k<stamp>\.$') { return $false }
    $marker = 'FRESH_FIRST_' + $Matches.stamp
    if ($Web.authenticated -isnot [bool] -or !$Web.authenticated -or $Web.streaming -isnot [bool] -or $Web.streaming -or
        $Web.conversation.url -cne ('https://chatgpt.com' + $Pending.expected_path) -or
        $Main.active_surface -cne 'social_ai' -or $Main.social_chat.web_chat_provider_id -cne 'chatgpt_web' -or
        $Main.social_chat.web_chat_conversation_path -cne $Pending.expected_path -or
        !(Test-ChatGptFreshProjectFixtureUsers @($Web.conversation.messages))) { return $false }
    $users = @($Web.conversation.messages | Where-Object { $_.role -ceq 'user' -and $_.content -ceq $Pending.prompt })
    if ($users.Count -ne 1 -or $users[0].id -cnotmatch '^[a-f0-9]{8}(?:-[a-f0-9]{4}){3}-[a-f0-9]{12}$' -or
        ($Pending.user_message_id -and $Pending.user_message_id -cne $users[0].id)) { return $false }
    $inside = $false; $answers = 0
    foreach ($message in $Web.conversation.messages) {
        if ($message.role -ceq 'user') {
            if ($inside) { break }
            $inside = $message.id -ceq $users[0].id
        } elseif ($inside -and $message.role -ceq 'assistant' -and $message.state -ceq 'completed' -and
            ([string]$message.content -replace '\\([_-])', '$1').Contains($marker)) { $answers++ }
    }
    $native = @($Main.social_chat.messages | Where-Object { $_.role -ceq 'friend' -and
        ([string]$_.content -replace '\\([_-])', '$1').Contains($marker) })
    return $answers -eq 1 -and $native.Count -eq 1
}

function Assert-ChatGptFreshProjectFixture {
    param([Parameter(Mandatory)]$Runtime, [string]$Path, [string]$ProjectId,
        [ValidateRange(5,30)][int]$OwnershipTimeoutSec = 15)
    if (!(Test-ChatGptFreshProjectPath $Path $ProjectId)) { throw 'project_path_invalid' }
    $deadline = [DateTimeOffset]::UtcNow.AddSeconds($OwnershipTimeoutSec)
    do {
        $main = Invoke-ChatGptWebSmokeMcp -Runtime $Runtime -Tool ui_state -MainState
        $web = Invoke-ChatGptWebSmokeMcp -Runtime $Runtime -Tool ui_state
        $owned = $main.social_chat.web_chat_conversation_path -ceq $Path -and
            $web.conversation.url -ceq ('https://chatgpt.com' + $Path) -and
            (Test-ChatGptFreshProjectFixtureUsers @($web.conversation.messages)) -and
            (Test-ChatGptFreshProjectFixtureUsers @($main.social_chat.messages))
        if ($owned -and $main.input.has_text -ceq $false -and $web.input.text -ceq '' -and
            $web.streaming -ceq $false -and $web.dictation_active -ceq $false) { break }
        Start-Sleep -Milliseconds 500
    } while ([DateTimeOffset]::UtcNow -lt $deadline)
    if (!$owned) { throw 'project_synthetic_fixture_unconfirmed' }
    if ($main.input.has_text -cne $false -or $web.input.text -cne '' -or $web.streaming -cne $false -or
        $web.dictation_active -cne $false) { throw 'project_fixture_not_idle' }
    $action = Invoke-ChatGptWebSmokeAction -Runtime $Runtime -Action chatgpt_list_conversation_files -Arguments @{conversation_path=$Path}
    if ($action.control_ok -cne $true -or !$action.command_receipt.request_id) { throw 'project_fixture_files_rejected' }
    $requestId = $action.command_receipt.request_id
    $retriedRead = $false
    $deadline = [DateTimeOffset]::UtcNow.AddSeconds(25)
    do {
        $web = Invoke-ChatGptWebSmokeMcp -Runtime $Runtime -Tool ui_state
        $receipt = @($web.command_requests | Where-Object {
            $_.request_id -ceq $requestId -and $_.expected_web_action -ceq 'list_conversation_files'
        }) | Select-Object -Last 1
        $index = $web.conversation_files
        if (Test-ChatGptFreshProjectFiles $receipt $index $Path) { break }
        if (!$retriedRead -and $receipt.status -ceq 'failed' -and $receipt.result.detail -ceq 'files_context_changed' -and
            $web.conversation.url -ceq ('https://chatgpt.com' + $Path) -and
            (Test-ChatGptFreshProjectFixtureUsers @($web.conversation.messages))) {
            # A navigation can retire an in-flight read. Never retry a send here.
            $retriedRead = $true
            $action = Invoke-ChatGptWebSmokeAction -Runtime $Runtime -Action chatgpt_list_conversation_files -Arguments @{conversation_path=$Path}
            if ($action.control_ok -cne $true -or !$action.command_receipt.request_id) { throw 'project_fixture_files_rejected' }
            $requestId = $action.command_receipt.request_id
        }
        Start-Sleep -Milliseconds 500
    } while ([DateTimeOffset]::UtcNow -lt $deadline)
    if (!(Test-ChatGptFreshProjectFiles $receipt $index $Path)) {
        throw 'project_fixture_attachment_unconfirmed'
    }
    Confirm-ChatGptFreshProjectMembership -Runtime $Runtime -Path $Path -ProjectId $ProjectId | Out-Null
}
