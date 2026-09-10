#requires -Version 7.0

function Test-ChatGptSharedLinkFixture {
    param($Context, [string]$ExpectedUrl, [string]$Prompt, [string]$Marker)
    if ($Marker -cnotmatch '^ELON-SHARE-UI-[a-f0-9]{32}$' -or
        $Prompt -cne ('Reply only with this exact test marker: ' + $Marker) -or
        $Context.conversation_url -cne $ExpectedUrl -or $Context.context_complete -ne $true -or
        $Context.context_streaming -ne $false -or $Context.has_more -ne $false -or
        $Context.has_more_before -ne $false -or $Context.message_offset -ne 0) { return $false }
    $rows = @($Context.messages)
    if ($rows.Count -lt 2 -or $rows.Count -gt 4 -or $Context.message_count -ne $rows.Count -or $rows[0].role -ne 'user' -or
        $rows[0].content -cne $Prompt) { return $false }
    $matched = $false
    for ($i = 0; $i -lt $rows.Count; $i++) {
        $row = $rows[$i]
        if ($row.state -ne 'completed' -or $row.content_truncated -ne $false -or
            @($row.parts).Count -ne 0 -or $row.parts_truncated -eq $true -or
            [string]::IsNullOrWhiteSpace([string]$row.content) -or ([string]$row.content).Length -gt 300) { return $false }
        if ($i -gt 0) {
            if ($row.role -ne 'assistant') { return $false }
            if (([string]$row.content).Trim() -ceq $Marker) { $matched = $true }
        }
    }
    return $matched
}
