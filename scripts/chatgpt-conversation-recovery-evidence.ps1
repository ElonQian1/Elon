#requires -Version 7.0

function Get-ChatGptRecoveryBodyDigest {
    param([object[]]$Messages)
    if (!$Messages.Count -or $Messages.Count -gt 200) { return $null }
    $rows = @()
    foreach ($message in $Messages) {
        if ($message.role -cnotin @('user','assistant','friend') -or
            $message.content -isnot [string] -or $message.content.Length -gt 240000) { return $null }
        $rows += [ordered]@{id=[string]$message.id;role=$message.role;content=$message.content}
    }
    $bytes = [Text.Encoding]::UTF8.GetBytes(($rows | ConvertTo-Json -Depth 4 -Compress))
    return [Convert]::ToHexString([Security.Cryptography.SHA256]::HashData($bytes))
}

function Get-ChatGptConversationRecoveryEvidence {
    param($Main, $Web, [string]$Path, [int]$AdapterVersion)
    foreach ($value in @($Web.authenticated, $Main.input.has_text, $Main.social_chat.web_chat_streaming,
        $Web.streaming, $Web.dictation_active)) { if ($value -isnot [bool]) { return $null } }
    if ($Web.adapter_version -isnot [int] -and $Web.adapter_version -isnot [long]) { return $null }
    if ($Path -cnotmatch '^/(?:g/g-p-[a-f0-9]{32}(?:-[A-Za-z0-9_-]{1,124})?/)?c/[a-f0-9]{8}(?:-[a-f0-9]{4}){3}-[a-f0-9]{12}$' -or
        $Web.adapter_version -ne $AdapterVersion -or $Web.authenticated -cne $true -or
        $Main.active_surface -cne 'social_ai' -or $Main.social_chat.interaction_mode -cne 'chat' -or
        $Main.social_chat.web_chat_provider_id -cne 'chatgpt_web' -or
        $Main.social_chat.web_chat_conversation_path -cne $Path -or
        $Web.conversation.url -cne ('https://chatgpt.com' + $Path) -or
        $Main.input.has_text -cne $false -or $Web.input.text -cne '' -or
        $Main.social_chat.web_chat_streaming -cne $false -or $Web.streaming -cne $false -or
        $Web.dictation_active -cne $false -or $Web.private_voice_native_research.phase -cne 'idle') { return $null }
    $native = Get-ChatGptRecoveryBodyDigest @($Main.social_chat.messages)
    $official = Get-ChatGptRecoveryBodyDigest @($Web.conversation.messages)
    if (!$native -or !$official) { return $null }
    if ($Web.conversation.context_complete -isnot [bool] -or !$Web.conversation.context_complete -or
        $Web.conversation.messages_truncated -isnot [bool] -or $Web.conversation.messages_truncated -or
        $Web.conversation.message_window_start -ne 0 -or
        $Web.conversation.available_message_count -ne @($Web.conversation.messages).Count -or
        $Web.conversation.message_count -ne @($Web.conversation.messages).Count -or
        @($Main.social_chat.messages).Count -ne @($Web.conversation.messages).Count) { return $null }
    return [pscustomobject]@{native_digest=$native;web_digest=$official;
        native_count=@($Main.social_chat.messages).Count;web_count=@($Web.conversation.messages).Count}
}

function Test-ChatGptConversationRecoveryMatch {
    param($Before, $After)
    return $null -ne $Before -and $null -ne $After -and
        $Before.native_count -gt 0 -and $Before.web_count -gt 0 -and
        $Before.native_count -eq $After.native_count -and $Before.web_count -eq $After.web_count -and
        $Before.native_digest -cmatch '^[A-F0-9]{64}$' -and $Before.web_digest -cmatch '^[A-F0-9]{64}$' -and
        $Before.native_digest -ceq $After.native_digest -and $Before.web_digest -ceq $After.web_digest
}

function Get-ChatGptConversationRecoveryDiagnostic {
    param($Main, [string]$Path)
    # Use the nested projection from the same native snapshot, not a later MCP read.
    $web = $Main.chatgpt_web_mcp
    $surface = if ($Main.active_surface -cin @('social_ai','friend_chat','project_chat',
        'conversation_home','project_space','project_plaza','profile','agent','unknown')) {
        $Main.active_surface
    } else { 'unknown' }
    $nativeRoute = ![string]::IsNullOrWhiteSpace($Path) -and
        $Main.social_chat.web_chat_conversation_path -ceq $Path
    $webRoute = ![string]::IsNullOrWhiteSpace($Path) -and
        $web.conversation.url -ceq ('https://chatgpt.com' + $Path)
    $layer = if ($surface -ceq 'unknown') { 'native_state' }
        elseif ($surface -cne 'social_ai') { 'native_surface' }
        elseif ($null -eq $web) { 'web_projection' }
        elseif (!$nativeRoute -or !$webRoute) { 'conversation_route' }
        else { 'body_or_readiness' }
    return [pscustomobject]@{
        active_surface = $surface
        web_projection_present = $null -ne $web
        native_route_equal = $nativeRoute
        web_route_equal = $webRoute
        native_message_count = if ($Main.social_chat.messages -is [array]) { $Main.social_chat.messages.Count } else { $null }
        web_message_count = if ($web.conversation.messages -is [array]) { $web.conversation.messages.Count } else { $null }
        failure_layer = $layer
    }
}
