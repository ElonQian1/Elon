pub(crate) fn agent_runtime_context_helpers() -> &'static str {
    r#"
function Get-AgentMessageContentLength {
    param([AllowNull()]$Message)
    if ($null -eq $Message) { return 0 }
    $content = ''
    if ($Message -is [hashtable]) {
        $content = [string]$Message['content']
    } elseif ($Message.PSObject.Properties['content']) {
        $content = [string]$Message.content
    }
    if ($null -eq $content) { return 0 }
    return $content.Length
}

function Get-AgentMessagesTotalChars {
    param([AllowNull()]$Messages)
    $total = 0
    foreach ($message in @($Messages)) {
        $total += Get-AgentMessageContentLength $message
    }
    return $total
}

function Compress-AgentRuntimeMessages {
    param(
        [Parameter(Mandatory = $true)]$Messages,
        [Parameter(Mandatory = $true)][int]$Turn
    )
    $items = @($Messages)
    $beforeChars = Get-AgentMessagesTotalChars $items
    if ($beforeChars -le $MaxContextChars -or $items.Count -le 4) {
        return [pscustomobject]@{
            Messages = $items
            Compacted = $false
            BeforeChars = $beforeChars
            AfterChars = $beforeChars
            OmittedMessages = 0
            OmittedChars = 0
        }
    }

    $head = @($items[0], $items[1])
    $headChars = Get-AgentMessagesTotalChars $head
    $summaryReserveChars = 700
    $tailBudget = [Math]::Max(1000, $MaxContextChars - $headChars - $summaryReserveChars)
    $tail = @()
    $tailChars = 0

    for ($i = $items.Count - 1; $i -ge 2; $i--) {
        $candidate = $items[$i]
        $candidateChars = Get-AgentMessageContentLength $candidate
        if (@($tail).Count -eq 0 -or ($tailChars + $candidateChars) -le $tailBudget) {
            $tail = @($candidate) + @($tail)
            $tailChars += $candidateChars
        } else {
            break
        }
    }

    $omittedMessages = [Math]::Max(0, $items.Count - $head.Count - @($tail).Count)
    if ($omittedMessages -eq 0) {
        return [pscustomobject]@{
            Messages = $items
            Compacted = $false
            BeforeChars = $beforeChars
            AfterChars = $beforeChars
            OmittedMessages = 0
            OmittedChars = 0
        }
    }

    $omittedChars = [Math]::Max(0, $beforeChars - $headChars - $tailChars)
    $summary = "Context compacted by Elon local runtime at turn $Turn. Omitted $omittedMessages older assistant/tool-result messages ($omittedChars chars). Recent messages are kept. Re-read files with read_file_range or list_dir when details are needed."
    $compressed = @()
    $compressed += $head
    $compressed += @{ role = 'user'; content = $summary }
    $compressed += $tail
    $afterChars = Get-AgentMessagesTotalChars $compressed
    $Script:AgentContextCompactionCount += 1

    Write-AgentRunEvent -Type 'context_compacted' -Data ([ordered]@{
        turn = $Turn
        before_chars = $beforeChars
        after_chars = $afterChars
        omitted_messages = $omittedMessages
        omitted_chars = $omittedChars
        max_context_chars = $MaxContextChars
        compaction_count = $Script:AgentContextCompactionCount
    })

    return [pscustomobject]@{
        Messages = $compressed
        Compacted = $true
        BeforeChars = $beforeChars
        AfterChars = $afterChars
        OmittedMessages = $omittedMessages
        OmittedChars = $omittedChars
    }
}
"#
}
