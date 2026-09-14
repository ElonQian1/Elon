#requires -Version 7.0
. (Join-Path $PSScriptRoot 'chatgpt-fresh-text-smoke-evidence.ps1')
. (Join-Path $PSScriptRoot 'chatgpt-web-smoke-tool-reply.ps1')

function Test-ChatGptFreshToolClearReceipt {
    param([AllowNull()]$Before, [AllowNull()]$After)
    try {
        if ($Before.page_generation -le 0 -or $After.page_generation -ne $Before.page_generation -or
            $Before.conversation.url -cnotmatch '^https://chatgpt\.com/c/[a-f0-9-]+$' -or
            $After.conversation.url -cne $Before.conversation.url -or
            $After.streaming -isnot [bool] -or $After.streaming -or
            $After.input.text -cne '') { return $false }
        $prior = @($Before.command_requests | ForEach-Object request_id)
        $receipts = @($After.command_requests | Where-Object {
            $_.request_id -cnotin $prior -and $_.expected_web_action -ceq 'select_composer_tool'
        })
        return $receipts.Count -eq 1 -and $receipts[0].status -ceq 'succeeded' -and
            $receipts[0].result.ok -is [bool] -and $receipts[0].result.ok
    } catch { return $false }
}

function Test-ChatGptFreshToolCleared {
    param([AllowNull()]$Before, [AllowNull()]$After, [AllowNull()][object[]]$Items,
        [Parameter(Mandatory)][ValidateSet('web_search','image_generation')][string]$ToolId)
    if (!(Test-ChatGptFreshToolClearReceipt -Before $Before -After $After)) { return $false }
    try {
        $target = @($Items | Where-Object semantic -CEQ $ToolId)
        if ($target.Count -ne 1) { return $false }
        foreach ($item in $Items) {
            if ($item.selected -isnot [bool] -or $item.selected) { return $false }
        }
        return $true
    } catch { return $false }
}

function Get-ChatGptFreshToolNativeEvidence {
    param([AllowNull()]$Baseline, [AllowNull()]$Web, [AllowNull()]$Main,
        [AllowNull()]$Native, [Parameter(Mandatory)][string]$Prompt,
        [Parameter(Mandatory)][ValidateSet('citation','image')][string]$PartType)
    $result = [ordered]@{ready=$false;reason='conversation_changed';web_parts=0;native_parts=0}
    if (!(Test-ChatGptFreshSendContinuity -Before $Baseline -After $Web -Main $Main -Prompt $Prompt)) {
        return [pscustomobject]$result
    }
    if ($Web.streaming -isnot [bool] -or $Web.streaming -or
        $Main.social_chat.web_chat_streaming -isnot [bool] -or $Main.social_chat.web_chat_streaming) {
        $result.reason='streaming'; return [pscustomobject]$result
    }
    $users = @($Web.conversation.messages | Where-Object { $_.role -ceq 'user' -and $_.content -ceq $Prompt })
    try {
        $webEvidence = Get-ChatGptWebToolReplyEvidence -Messages @($Web.conversation.messages) `
            -UserMessageId ([string]$users[0].id) -ExpectedPartTypes @($PartType)
    } catch { $result.reason='web_reply_anchor'; return [pscustomobject]$result }
    if (!$webEvidence.matched) { $result.reason='web_output_missing'; return [pscustomobject]$result }
    $result.web_parts = $webEvidence.matching_part_count
    if ($Native.control_ok -ne $true -or $Native.provider_id -cne 'chatgpt_web' -or
        [string]$Native.conversation_path -cne [string]$Main.social_chat.web_chat_conversation_path -or
        $Native.streaming -isnot [bool] -or $Native.streaming) {
        $result.reason='native_context_unconfirmed'; return [pscustomobject]$result
    }
    $nativeUsers = @($Native.messages | Where-Object {
        $_.role -ceq 'user' -and $_.source_message_id -ceq $users[0].id -and $_.content -ceq $Prompt
    })
    if ($nativeUsers.Count -ne 1 -or $nativeUsers[0].content_truncated -ne $false) {
        $result.reason='native_user_anchor'; return [pscustomobject]$result
    }
    $index = [Array]::IndexOf(@($Native.messages), $nativeUsers[0])
    $following = @($Native.messages | Select-Object -Skip ($index + 1))
    if (@($following | Where-Object role -CEQ 'user').Count) {
        $result.reason='native_later_user'; return [pscustomobject]$result
    }
    $webIndex = [Array]::IndexOf(@($Web.conversation.messages), $users[0])
    $replyIds = @($Web.conversation.messages | Select-Object -Skip ($webIndex + 1) | Where-Object {
        $_.role -ceq 'assistant' -and $_.state -ceq 'completed'
    } | ForEach-Object id)
    $answers = @($following | Where-Object {
        $_.role -ceq 'friend' -and $_.source_message_id -cin $replyIds -and $_.parts_truncated -eq $false
    })
    $result.native_parts = @($answers | ForEach-Object parts | Where-Object type -CEQ $PartType).Count
    $result.ready = $result.native_parts -gt 0
    $result.reason = if ($result.ready) { 'ready' } else { 'native_output_missing' }
    return [pscustomobject]$result
}
