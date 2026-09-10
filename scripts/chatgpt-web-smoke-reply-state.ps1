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
