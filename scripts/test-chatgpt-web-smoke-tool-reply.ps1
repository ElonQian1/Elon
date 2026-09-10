$ErrorActionPreference = 'Stop'
. (Join-Path $PSScriptRoot 'chatgpt-web-smoke-tool-reply.ps1')
function Message([string]$Id, [string]$Role, [string]$State, [string[]]$Parts = @()) {
    [pscustomobject]@{id=$Id;role=$Role;state=$State;parts=@($Parts | ForEach-Object { [pscustomobject]@{type=$_} })}
}
function Check([bool]$Condition, [string]$Name) {
    if (!$Condition) { throw "Failed: $Name" }
    Write-Output "TOOL_REPLY_CASE=passed name=$Name"
}
$user = Message 'user-current' 'user' 'completed'
$image = Message 'assistant-image' 'assistant' 'completed' @('image', 'image')
$text = Message 'assistant-text' 'assistant' 'completed'
$result = Get-ChatGptWebToolReplyEvidence @($user,$image,$text) 'user-current' @('image')
Check ($result.matched -and $result.matching_part_count -eq 1) 'image_before_trailing_text'
$result = Get-ChatGptWebToolReplyEvidence @($user,$image,(Message 'pending' 'assistant' 'streaming')) 'user-current' @('image')
Check $result.matched 'completed_image_before_pending_text'
$result = Get-ChatGptWebToolReplyEvidence @($image,$user,$text) 'user-current' @('image')
Check (!$result.matched) 'old_image_is_not_current_reply'
$result = Get-ChatGptWebToolReplyEvidence @($user,(Message 'pending' 'assistant' 'streaming' @('image'))) 'user-current' @('image')
Check (!$result.matched) 'pending_image_is_not_completed'
$result = Get-ChatGptWebToolReplyEvidence @($user,(Message 'tool' 'tool' 'completed' @('image')),$text) 'user-current' @('image')
Check (!$result.matched) 'tool_internal_output_is_not_assistant_reply'
$result = Get-ChatGptWebToolReplyEvidence @($user,(Message 'citation' 'assistant' 'completed' @('citation')),$text) 'user-current' @('citation')
Check $result.matched 'citation_before_trailing_text'
$result = Get-ChatGptWebToolReplyEvidence @($user,$text) 'user-current' @()
Check $result.matched 'text_only_tool'
$result = Get-ChatGptWebToolReplyEvidence @($user) 'user-current' @()
Check (!$result.matched) 'no_assistant_reply'
foreach ($case in @(
    @{name='missing_anchor';messages=@($image,$text)},
    @{name='duplicate_anchor';messages=@($user,$user,$image)},
    @{name='later_user_turn';messages=@($user,(Message 'next' 'user' 'completed'),$image)}
)) {
    $rejected = $false
    try { Get-ChatGptWebToolReplyEvidence $case.messages 'user-current' @('image') | Out-Null } catch { $rejected = $true }
    Check $rejected $case.name
}
