#requires -Version 5.1

function Get-ChatGptNativeSelectorsFromXml {
    param([Parameter(Mandatory = $true)][string]$UiXml)

    return @(
        [regex]::Matches($UiXml, 'content-desc="([^"]*chatgpt-native:[^"]*)"') |
            ForEach-Object { $_.Groups[1].Value } |
            Sort-Object -Unique
    )
}

function Test-ChatGptResourceVisible {
    param(
        [Parameter(Mandatory = $true)][string]$UiXml,
        [Parameter(Mandatory = $true)][string]$ResourceId,
        [string]$PackageName = "com.elon.app"
    )

    return $UiXml.Contains("resource-id=`"${PackageName}:id/${ResourceId}`"")
}

function Get-ChatGptContextPagingEvidence {
    param(
        [Parameter(Mandatory = $true)][scriptblock]$InvokeUiAction,
        [Parameter(Mandatory = $true)][int]$MessageOffset
    )

    $first = & $InvokeUiAction "chatgpt_get_context" @{
        message_offset = $MessageOffset
        message_limit = 1
    }
    $replay = $null
    if ($first.control_ok -eq $true -and -not [string]::IsNullOrWhiteSpace([string]$first.message_cursor)) {
        $replay = & $InvokeUiAction "chatgpt_get_context" @{
            message_cursor = [string]$first.message_cursor
            message_limit = 1
        }
    }
    $next = $null
    if ($first.control_ok -eq $true -and $first.has_more -eq $true) {
        $next = & $InvokeUiAction "chatgpt_get_context" @{
            message_cursor = [string]$first.next_message_cursor
            message_limit = 1
        }
    }

    return [pscustomobject]@{
        first = $first
        replay = $replay
        next = $next
    }
}

function Normalize-ChatGptProbeReply {
    param([AllowEmptyString()][string]$Text)

    return $Text.Replace('\_', '_').Replace('\-', '-').Trim()
}

function Wait-ChatGptStreamingState {
    param(
        [Parameter(Mandatory = $true)][scriptblock]$InvokeUiState,
        [Parameter(Mandatory = $true)][bool]$Expected,
        [Parameter(Mandatory = $true)][int]$TimeoutSec,
        [int]$PollIntervalMilliseconds = 250
    )

    $deadline = [DateTimeOffset]::UtcNow.AddSeconds($TimeoutSec)
    do {
        $state = & $InvokeUiState
        if ([bool]$state.streaming -eq $Expected) { return $state }
        Start-Sleep -Milliseconds $PollIntervalMilliseconds
    } while ([DateTimeOffset]::UtcNow -lt $deadline)
    throw "Timed out waiting for ChatGPT streaming state: $Expected"
}

function Wait-ChatGptCommandReceipt {
    param(
        [Parameter(Mandatory = $true)][scriptblock]$InvokeUiState,
        [Parameter(Mandatory = $true)][string]$RequestId,
        [Parameter(Mandatory = $true)][string]$ExpectedAction,
        [Parameter(Mandatory = $true)][int]$TimeoutSec,
        [Parameter(Mandatory = $true)][int]$PollIntervalSec
    )

    $deadline = [DateTimeOffset]::UtcNow.AddSeconds($TimeoutSec)
    do {
        $state = & $InvokeUiState
        $receipt = @($state.command_requests) |
            Where-Object { [string]$_.request_id -eq $RequestId } |
            Select-Object -Last 1
        if ($null -ne $receipt -and [string]$receipt.status -eq "failed") {
            throw "ChatGPT command failed: $ExpectedAction"
        }
        if (
            $null -ne $receipt -and
            [string]$receipt.status -eq "succeeded" -and
            [string]$receipt.expected_web_action -eq $ExpectedAction -and
            $receipt.result.ok -eq $true
        ) {
            return [pscustomobject]@{ state = $state; receipt = $receipt }
        }
        Start-Sleep -Seconds $PollIntervalSec
    } while ([DateTimeOffset]::UtcNow -lt $deadline)
    throw "Timed out waiting for ChatGPT command: $ExpectedAction"
}

function Wait-ChatGptProbeReply {
    param(
        [Parameter(Mandatory = $true)][scriptblock]$InvokeUiState,
        [Parameter(Mandatory = $true)][string]$RequestId,
        [Parameter(Mandatory = $true)][string]$Marker,
        [Parameter(Mandatory = $true)][long]$AfterMs,
        [Parameter(Mandatory = $true)][int]$TimeoutSec,
        [Parameter(Mandatory = $true)][int]$PollIntervalSec
    )

    $deadline = [DateTimeOffset]::UtcNow.AddSeconds($TimeoutSec)
    $last = $null
    $lastReceipt = $null
    do {
        $last = & $InvokeUiState
        $lastReceipt = @($last.command_requests) |
            Where-Object { [string]$_.request_id -eq $RequestId } |
            Select-Object -First 1
        $messages = @($last.conversation.messages)
        $lastMessage = $messages | Select-Object -Last 1
        if (
            $null -ne $lastReceipt -and
            [string]$lastReceipt.expected_web_action -eq "send_prompt" -and
            [string]$lastReceipt.status -eq "succeeded" -and
            $lastReceipt.result.ok -eq $true -and
            [long]$lastReceipt.completed_at_ms -gt $AfterMs -and
            $last.streaming -eq $false -and
            $messages.Count -ge 2 -and
            [string]$lastMessage.role -eq "assistant" -and
            (Normalize-ChatGptProbeReply ([string]$lastMessage.content)) -like "*$Marker*"
        ) {
            return $last
        }
        Start-Sleep -Seconds $PollIntervalSec
    } while ([DateTimeOffset]::UtcNow -lt $deadline)
    throw "Timed out waiting for ChatGPT Web probe receipt. Receipt=$($lastReceipt.status), last action=$($last.last_command.action)."
}
