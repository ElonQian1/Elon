#requires -Version 7.0

function Get-ChatGptFreshPendingObservedPath {
    param([AllowNull()]$Web, [string]$Prompt, [string]$UserMessageId)
    if ($UserMessageId -cnotmatch '^[a-f0-9]{8}(?:-[a-f0-9]{4}){3}-[a-f0-9]{12}$' -or
        $Prompt -cnotmatch '^ELON_FRESH_TEXT_ACCEPTANCE_V1 first (?<stamp>\d{13})\. Reply exactly FRESH_FIRST_\k<stamp>\.$' -or
        $Web.surface -cne 'chatgpt_web' -or $Web.authenticated -isnot [bool] -or !$Web.authenticated -or
        [string]$Web.conversation.url -cnotmatch '^https://chatgpt\.com(?<path>/c/[a-f0-9]{8}(?:-[a-f0-9]{4}){3}-[a-f0-9]{12})$') {
        return ''
    }
    $path = $Matches.path
    $users = @($Web.conversation.messages | Where-Object role -CEQ 'user')
    if ($users.Count -ne 1 -or $users[0].id -cne $UserMessageId -or $users[0].content -cne $Prompt) { return '' }
    # This is a read-only lookup hint, not terminal response or native UI proof.
    return $path
}

function Test-ChatGptFreshPendingReadback {
    param([AllowNull()]$Pending, [AllowNull()]$Web, [AllowNull()]$Main)
    if ($Pending.schema -cne 'elon.fresh_text_pending.v1' -or $Pending.source -cne 'native_fixture' -or
        $Pending.new_conversation -isnot [bool] -or !$Pending.new_conversation -or
        $Pending.replay_allowed -isnot [bool] -or $Pending.replay_allowed -or
        $Pending.readback_completed -isnot [bool] -or !$Pending.readback_completed -or
        [string]$Pending.resolved_path -cnotmatch '^/c/[a-f0-9]{8}(?:-[a-f0-9]{4}){3}-[a-f0-9]{12}$' -or
        [string]$Pending.user_message_id -cnotmatch '^[a-f0-9]{8}(?:-[a-f0-9]{4}){3}-[a-f0-9]{12}$' -or
        [string]$Pending.prompt -cnotmatch '^ELON_FRESH_TEXT_ACCEPTANCE_V1 first (?<stamp>\d{13})\. Reply exactly FRESH_FIRST_\k<stamp>\.$') { return $false }
    $marker = 'FRESH_FIRST_' + $Matches.stamp
    if ($Web.surface -cne 'chatgpt_web' -or $Web.authenticated -isnot [bool] -or !$Web.authenticated -or
        $Web.streaming -isnot [bool] -or $Web.streaming -or
        $Web.conversation.url -cne ('https://chatgpt.com' + $Pending.resolved_path) -or
        $Main.active_surface -cne 'social_ai' -or $Main.social_chat.web_chat_provider_id -cne 'chatgpt_web' -or
        $Main.social_chat.web_chat_conversation_path -cne $Pending.resolved_path) { return $false }
    $users = @($Web.conversation.messages | Where-Object role -CEQ 'user')
    if ($users.Count -ne 1 -or $users[0].id -cne $Pending.user_message_id -or
        $users[0].content -cne $Pending.prompt) { return $false }
    $answers = @($Web.conversation.messages | Where-Object { $_.role -ceq 'assistant' -and
        $_.state -ceq 'completed' -and ([string]$_.content -replace '\\([_-])', '$1').Contains($marker) })
    $native = @($Main.social_chat.messages | Where-Object { $_.role -ceq 'friend' -and
        ([string]$_.content -replace '\\([_-])', '$1').Contains($marker) })
    return $answers.Count -eq 1 -and $native.Count -eq 1
}

function Test-ChatGptFreshTextIdle {
    param([AllowNull()]$State)
    return $State.schema -ceq 'elon.fresh_text_trial.v1' -and
        ($State.version -is [int] -or $State.version -is [long]) -and $State.version -in @(7, 8) -and
        $State.pending -is [bool] -and !$State.pending -and
        $State.armed -is [bool] -and !$State.armed
}

function Test-ChatGptFreshSendEvidence {
    param([AllowNull()]$Before, [AllowNull()]$After, [AllowNull()]$Receipt, [switch]$UseDefault)
    foreach ($value in @($Before.armed, $Before.pending, $After.dispatched, $After.accepted,
            $After.reconciled, $After.pending, $Receipt.result.ok)) {
        if ($value -isnot [bool]) { return $false }
    }
    foreach ($value in @($Before.version, $After.version, $Before.attempts, $After.attempts, $After.stream_events)) {
        if (($value -isnot [int] -and $value -isnot [long]) -or $value -lt 0) { return $false }
    }
    return $Before.schema -ceq 'elon.fresh_text_trial.v1' -and $After.schema -ceq 'elon.fresh_text_trial.v1' -and
        $Before.version -in @(7, 8) -and $After.version -in @(7, 8) -and !$Before.pending -and
        ($UseDefault -or $Before.armed) -and $After.operation -ceq 'send' -and
        $After.attempts -eq ($Before.attempts + 1) -and $After.dispatched -and $After.accepted -and
        $After.reconciled -and !$After.pending -and $After.stream_events -gt 0 -and
        $After.history -ceq 'reconciled' -and
        $Receipt.expected_web_action -ceq 'send_prompt' -and $Receipt.status -ceq 'succeeded' -and
        $Receipt.result.ok -and $Receipt.result.detail -ceq 'private_text_v1:accepted'
}

function Test-ChatGptFreshSendContinuity {
    param([AllowNull()]$Before, [AllowNull()]$After, [AllowNull()]$Main,
        [Parameter(Mandatory)][string]$Prompt, [switch]$NewConversation)
    $path = [string]$Main.social_chat.web_chat_conversation_path
    if ($path -cnotmatch '^/c/[a-f0-9]{8}-[a-f0-9]{4}-[a-f0-9]{4}-[a-f0-9]{4}-[a-f0-9]{12}$' -or
        [string]$After.conversation.url -cne "https://chatgpt.com$path") { return $false }
    if ($NewConversation) {
        if (@($Before.conversation.messages).Count -ne 0 -or
            [string]$Before.conversation.url -notmatch '^https://chatgpt.com/?(?:\?[^#]*)?$') { return $false }
    } elseif ([string]$Before.conversation.url -cne [string]$After.conversation.url) { return $false }
    $users = @($After.conversation.messages | Where-Object { $_.role -ceq 'user' })
    $sent = @($users | Where-Object { [string]$_.content -ceq $Prompt })
    if ($sent.Count -ne 1 -or !$sent[0].id) { return $false }
    $prior = @($Before.conversation.messages | Where-Object { $_.role -ceq 'user' })
    if ($users.Count -ne $prior.Count + 1) { return $false }
    foreach ($old in $prior) {
        if (!$old.id -or @($users | Where-Object { $_.id -ceq $old.id -and $_.content -ceq $old.content }).Count -ne 1) {
            return $false
        }
    }
    return $true
}

function Test-ChatGptFreshTextRestoreSafe {
    param([bool]$AwaitingResult, [AllowNull()]$Trial, [AllowNull()]$Main, [AllowNull()]$Web, [string]$ExpectedPath)
    return !$AwaitingResult -and (Test-ChatGptFreshTextIdle $Trial) -and
        $Main.active_surface -ceq 'social_ai' -and $Main.social_chat.web_chat_provider_id -ceq 'chatgpt_web' -and
        $Main.social_chat.web_chat_streaming -is [bool] -and !$Main.social_chat.web_chat_streaming -and
        [string]$Main.social_chat.web_chat_conversation_path -ceq $ExpectedPath -and
        $Main.input.has_text -is [bool] -and !$Main.input.has_text -and
        $Web.surface -ceq 'chatgpt_web' -and $Web.streaming -is [bool] -and !$Web.streaming -and
        ($ExpectedPath -ceq '' -and $Web.conversation.url -cmatch '^https://chatgpt.com/?$' -or
            $ExpectedPath -cne '' -and $Web.conversation.url -ceq "https://chatgpt.com$ExpectedPath") -and
        $Web.input.text -is [string] -and $Web.input.text -ceq ''
}
