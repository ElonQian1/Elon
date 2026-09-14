#requires -Version 7.0
. (Join-Path $PSScriptRoot 'chatgpt-fresh-text-smoke-evidence.ps1')

function Test-ChatGptFreshMediaFacts {
    param([AllowEmptyString()][string]$Text)
    $plain=$Text.Replace('\_', '_').Replace('**', '').Replace('`', '')
    return $plain.Contains('ELON_CHATGPT_ATTACHMENT_FIXTURE_V1=ready') -and
        $plain.Contains('ELON_PRIVATE_PDF_FIXTURE_V1=ready') -and
        $plain -match '(?is)\b(three|3)\s+(?:solid\s+)?blue\s+squares?\b' -and
        $plain -match '(?is)\b(one|1|a)\s+(?:solid\s+)?red\s+circle\b'
}

function Test-ChatGptFreshAttachmentNativeEvidence {
    param([AllowNull()]$Baseline,[AllowNull()]$Web,[AllowNull()]$Main,[AllowNull()]$Native,
        [Parameter(Mandatory)][string]$Prompt)
    if (!(Test-ChatGptFreshSendContinuity -Before $Baseline -After $Web -Main $Main -Prompt $Prompt)) {return $false}
    if ($Web.streaming -isnot [bool] -or $Web.streaming -or
        $Main.social_chat.web_chat_streaming -isnot [bool] -or $Main.social_chat.web_chat_streaming -or
        $Native.control_ok -cne $true -or $Native.provider_id -cne 'chatgpt_web' -or
        $Native.conversation_path -cne $Main.social_chat.web_chat_conversation_path -or
        $Native.streaming -isnot [bool] -or $Native.streaming) {return $false}
    $user=@($Web.conversation.messages|Where-Object {$_.role -ceq 'user' -and $_.content -ceq $Prompt})[0]
    $webIndex=[Array]::IndexOf(@($Web.conversation.messages),$user)
    $following=@($Web.conversation.messages|Select-Object -Skip ($webIndex+1))
    if (@($following|Where-Object role -CEQ user).Count) {return $false}
    $oldIds=@($Baseline.conversation.messages|ForEach-Object id)
    $answers=@($following|Where-Object {$_.role -ceq 'assistant' -and $_.state -ceq 'completed' -and $_.id -cnotin $oldIds})
    if (!$answers.Count -or !(Test-ChatGptFreshMediaFacts (@($answers|ForEach-Object content)-join "`n"))) {return $false}
    $nativeUser=@($Native.messages|Where-Object {$_.role -ceq 'user' -and $_.source_message_id -ceq $user.id -and $_.content -ceq $Prompt})
    if ($nativeUser.Count -ne 1 -or $nativeUser[0].content_truncated -cne $false) {return $false}
    $nativeIndex=[Array]::IndexOf(@($Native.messages),$nativeUser[0])
    $nativeFollowing=@($Native.messages|Select-Object -Skip ($nativeIndex+1))
    if (@($nativeFollowing|Where-Object role -CEQ user).Count) {return $false}
    $replyIds=@($answers|ForEach-Object id)
    $nativeAnswers=@($nativeFollowing|Where-Object {$_.role -ceq 'friend' -and
        $_.source_message_id -cin $replyIds -and $_.content_truncated -ceq $false})
    return $nativeAnswers.Count -gt 0 -and
        (Test-ChatGptFreshMediaFacts (@($nativeAnswers|ForEach-Object content)-join "`n"))
}
