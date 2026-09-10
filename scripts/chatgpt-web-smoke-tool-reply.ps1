#requires -Version 5.1

function Get-ChatGptWebToolReplyEvidence {
    param(
        [Parameter(Mandatory = $true)][AllowEmptyCollection()][object[]]$Messages,
        [Parameter(Mandatory = $true)][string]$UserMessageId,
        [Parameter(Mandatory = $true)][AllowEmptyCollection()][string[]]$ExpectedPartTypes
    )
    $anchors = @($Messages | Where-Object { $_.role -eq 'user' -and $_.id -eq $UserMessageId })
    if ($anchors.Count -ne 1) { throw 'Tool reply anchor missing or ambiguous.' }
    $anchorIndex = [Array]::IndexOf($Messages, $anchors[0])
    $following = @($Messages | Select-Object -Skip ($anchorIndex + 1))
    if (@($following | Where-Object { $_.role -eq 'user' }).Count) {
        throw 'Tool reply moved into a different user turn.'
    }
    $completed = @($following | Where-Object { $_.role -eq 'assistant' -and $_.state -eq 'completed' })
    $types = @($completed | ForEach-Object { $_.parts } | ForEach-Object { [string]$_.type } |
        Where-Object { $_ } | Sort-Object -Unique)
    $matches = @($types | Where-Object { $_ -in $ExpectedPartTypes }).Count
    [pscustomobject]@{
        matched = $completed.Count -gt 0 -and ($ExpectedPartTypes.Count -eq 0 -or $matches -gt 0)
        observed_part_types = $types
        matching_part_count = $matches
        completed_assistant_messages = $completed.Count
    }
}
