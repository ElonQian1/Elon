#requires -Version 7.0

function Get-ChatGptWritingSaveEvidence {
    param([AllowNull()]$Web, [AllowEmptyCollection()][string[]]$BeforeRequestIds,
        [Parameter(Mandatory)][string]$ExpectedPath,
        [Parameter(Mandatory)][string]$ExpectedGeneration,
        [Parameter(Mandatory)][string]$MessageId,
        [Parameter(Mandatory)][ValidateRange(0,15)][int]$PartIndex)
    $result=[ordered]@{confirmed=$false;reason='context_changed'}
    try {
        $url=[uri]$Web.conversation.url
        if ($url.Scheme -cne 'https' -or $url.Host -cne 'chatgpt.com' -or
            $url.UserInfo -or $url.Port -ne 443 -or $url.Query -or $url.Fragment -or
            $url.AbsolutePath -cne $ExpectedPath -or
            [string]$Web.page_generation -cne $ExpectedGeneration -or
            $Web.authenticated -isnot [bool] -or !$Web.authenticated -or
            $Web.streaming -isnot [bool] -or $Web.streaming -or
            $Web.dictation_active -isnot [bool] -or $Web.dictation_active) {
            return [pscustomobject]$result
        }
        $messages=@($Web.conversation.messages|Where-Object { $_.id -ceq $MessageId })
        if ($messages.Count -ne 1 -or $messages[0].role -cne 'assistant' -or
            $messages[0].state -cne 'completed' -or
            @($messages[0].parts).Count -le $PartIndex -or
            $messages[0].parts[$PartIndex].type -cne 'writing_block') {
            $result.reason='source_changed'; return [pscustomobject]$result
        }
        $new=@($Web.command_requests|Where-Object { $_.request_id -cnotin $BeforeRequestIds })
        $writes=@($new|Where-Object { $_.expected_web_action -ceq 'writing_block' })
        if ($writes.Count -ne 1 -or @($new|Where-Object {
            $_.expected_web_action -cin @('send_prompt','regenerate','delete_conversation','move_conversation_to_project')
        }).Count -gt 0) {
            $result.reason='receipt_ambiguous'; return [pscustomobject]$result
        }
        $receipt=$writes[0]
        if ([string]$receipt.request_id -cnotmatch '^mcp_[a-z0-9]{1,32}$' -or
            $receipt.status -cne 'succeeded' -or $receipt.result.ok -isnot [bool] -or
            !$receipt.result.ok -or $receipt.result.detail -cne 'writing_saved') {
            $result.reason='save_unconfirmed'; return [pscustomobject]$result
        }
        $result.confirmed=$true
        $result.reason='saved'
    } catch { $result.reason='evidence_unavailable' }
    return [pscustomobject]$result
}

function Test-ChatGptWritingRestoreAllowed {
    param([bool]$WriteAttempted, [AllowNull()]$Evidence)
    if (!$WriteAttempted) { return $true }
    try { return $Evidence.confirmed -is [bool] -and $Evidence.confirmed }
    catch { return $false }
}
