#requires -Version 5.1

function Get-ChatGptStoppedTurnEvidence {
    param($State, [string]$FirstPrompt, [string]$SecondPrompt,
        [string]$PartialReply, [string]$ExpectedReply)

    $rows = @($State.social_chat.messages)
    $users = @($rows | Where-Object { $_.role -eq 'user' })
    $answers = @($rows | Where-Object { $_.role -eq 'friend' })
    $ordered = $rows.Count -eq 4 -and
        ($rows.role -join ',') -eq 'user,friend,user,friend'
    $firstRetained = $answers.Count -ge 1 -and $PartialReply.Length -ge 80 -and
        ([string]$answers[0].content).StartsWith($PartialReply, [StringComparison]::Ordinal)
    $promptsSeparate = $users.Count -eq 2 -and
        [string]$users[0].content -ceq $FirstPrompt -and
        [string]$users[1].content -ceq $SecondPrompt
    $finalMatches = $answers.Count -eq 2 -and
        ([string]$answers[1].content).Trim().TrimEnd('.') -ceq $ExpectedReply
    return [ordered]@{
        passed = $ordered -and $firstRetained -and $promptsSeparate -and $finalMatches -and
            $State.social_chat.web_chat_streaming -eq $false
        user_rows = $users.Count
        assistant_rows = $answers.Count
        ordered_turns = $ordered
        prompts_separate = $promptsSeparate
        stopped_reply_retained = $firstRetained
        followup_reply_matched = $finalMatches
    }
}
