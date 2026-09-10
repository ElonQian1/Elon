#requires -Version 5.1

function Test-ChatGptRegeneratedReplyIdentity {
    param(
        [AllowNull()]$Receipt,
        [bool]$IdentityChanged,
        [bool]$ContentChanged,
        [bool]$RequireOfficialRuntime
    )
    if ($Receipt.expected_web_action -ne 'regenerate_response' -or
        $Receipt.status -ne 'succeeded' -or $Receipt.result.ok -ne $true) { return $false }
    $official = $Receipt.result.detail -ceq 'official_runtime_v1:regenerate_observed'
    if ($RequireOfficialRuntime -and !$official) { return $false }
    # The runtime proves a new provider variant and its original parent. A
    # native turn row is a display identity and may survive that replacement.
    return $IdentityChanged -or ($official -and $ContentChanged)
}

function Assert-ChatGptRegenerateForeground {
    param([Parameter(Mandatory = $true)]$Runtime)
    if (!(Test-WebChatNativeChatSurfaceForeground -Runtime $Runtime)) {
        throw 'Regenerate acceptance interrupted: native_chat_not_foreground.'
    }
}

function Get-ChatGptNativeRetryAdmission {
    param([AllowNull()]$Baseline, [AllowNull()]$Native)
    if ($Native.control_ok -ne $true -or $Native.provider_id -cne 'chatgpt_web') { return 'native_context_unavailable' }
    $url = $null
    if (![Uri]::TryCreate([string]$Baseline.conversation.url, [UriKind]::Absolute, [ref]$url) -or
        $Native.conversation_path -cne $url.AbsolutePath) { return 'native_conversation_changed' }
    if ($Native.streaming -isnot [bool] -or $Native.streaming) { return 'native_reply_active' }
    $assistant = @($Baseline.conversation.messages | Where-Object { $_.role -ceq 'assistant' }) | Select-Object -Last 1
    if (!$assistant.id -or $assistant.state -cne 'completed') { return 'native_target_unknown' }
    $rows = @($Native.messages | Where-Object { $_.source_message_id -ceq $assistant.id })
    if ($rows.Count -ne 1 -or $rows[0].role -cne 'friend') { return 'native_message_missing' }
    if ($rows[0].content_truncated -ne $false -or $rows[0].content -cne $assistant.content) { return 'native_message_changed' }
    if ('regenerate' -cnotin @($rows[0].actions)) { return 'native_retry_not_offered' }
    return 'ready'
}

function Get-ChatGptRegenerateDocumentContinuity {
    param([AllowNull()]$Baseline, [AllowNull()]$Current)

    if ($Current.surface -ne 'chatgpt_web') { return 'surface_unavailable' }
    if ($Current.bridge_state -ne 'ready') { return 'bridge_unavailable' }
    if ($Current.adapter_current -ne $true -or $Current.adapter_version -ne $Baseline.adapter_version) {
        return 'adapter_changed'
    }
    foreach ($state in @($Baseline, $Current)) {
        if (($state.page_generation -isnot [int] -and $state.page_generation -isnot [long]) -or
            $state.page_generation -le 0) { return 'document_unknown' }
    }
    if ($Current.page_generation -ne $Baseline.page_generation) { return 'document_changed' }
    if (!$Current.conversation.url -or $Current.conversation.url -cne $Baseline.conversation.url) {
        return 'conversation_changed'
    }
    if ($Current.streaming -isnot [bool] -or $Current.streaming) { return 'reply_active' }
    if ($null -eq $Current.input.text_length -or $Current.input.text_length -ne 0) { return 'draft_present' }
    if (@($Current.conversation.attachments).Count) { return 'attachment_present' }
    $before = @($Baseline.conversation.messages | Where-Object { $_.role -in @('user', 'assistant') })
    $after = @($Current.conversation.messages | Where-Object { $_.role -in @('user', 'assistant') })
    if ($before.Count -lt 2 -or $after.Count -ne $before.Count) { return 'turn_changed' }
    for ($i = 0; $i -lt $before.Count; $i++) {
        foreach ($field in @('id', 'role', 'state', 'content')) {
            if ([string]$after[$i].$field -cne [string]$before[$i].$field) { return 'turn_changed' }
        }
    }
    # The caller supplies a fresh page-command acknowledgement, not just cached UI state.
    return 'ready'
}

function Get-ChatGptExistingRegenerateProbe {
    param([Parameter(Mandatory = $true)]$State)
    $users = @($State.conversation.messages | Where-Object { $_.role -eq 'user' })
    $prompt = if ($users.Count -eq 1) { [string]$users[0].content } else { '' }
    $match = [regex]::Match($prompt,
        '^Reply with a fresh 12-character lowercase hexadecimal token, one space, then exactly: (ELON-CHATGPT-REGENERATE-[0-9]{10})$')
    if (!$match.Success -or $match.Value -cne $prompt) { throw 'Existing conversation is not an isolated regenerate probe.' }
    return [pscustomobject]@{ prompt = $prompt; marker = $match.Groups[1].Value }
}

function Get-ChatGptRegenerateReplyState {
    param(
        [AllowNull()]$State,
        [Parameter(Mandatory = $true)][string]$Prompt,
        [Parameter(Mandatory = $true)][string]$Marker
    )
    $users = @($State.conversation.messages | Where-Object { $_.role -eq 'user' })
    $assistants = @($State.conversation.messages | Where-Object { $_.role -eq 'assistant' })
    $assistant = $assistants | Select-Object -Last 1
    $promptMatches = $users.Count -eq 1 -and [string]$users[0].content -ceq $Prompt
    $markerMatches = $null -ne $assistant -and ([string]$assistant.content).Contains($Marker)
    $assistantState = if ($assistant.state -in @('completed', 'streaming', 'interrupted', 'error')) {
        [string]$assistant.state
    } else { 'unknown' }
    $reason = if ($State.surface -ne 'chatgpt_web') { 'native_surface_unavailable' }
        elseif ($State.adapter_current -ne $true) { 'adapter_unavailable' }
        elseif (!$promptMatches) { 'user_turn_mismatch' }
        elseif ($State.streaming -isnot [bool] -or $State.streaming) { 'reply_streaming_or_unknown' }
        elseif ($assistants.Count -eq 0) { 'assistant_missing' }
        elseif ($assistantState -ne 'completed') { 'assistant_incomplete' }
        elseif (!$markerMatches) { 'reply_marker_missing' }
        else { 'ready' }
    # Only structural evidence leaves the process; never export prompts or IDs.
    return [pscustomobject][ordered]@{
        schema = 'elon.chatgpt_web.regenerate_reply_state.v1'
        ready = $reason -eq 'ready'
        reason = $reason
        bridge = if ($State.bridge_state -in @('ready', 'connecting', 'web_only', 'login_required', 'error')) {
            [string]$State.bridge_state
        } else { 'unknown' }
        adapter_current = $State.adapter_current -eq $true
        streaming_known = $State.streaming -is [bool]
        streaming = $State.streaming -eq $true
        user_count = $users.Count
        assistant_count = $assistants.Count
        prompt_matches = $promptMatches
        assistant_state = $assistantState
        assistant_chars = ([string]$assistant.content).Length
        marker_matches = $markerMatches
    }
}
