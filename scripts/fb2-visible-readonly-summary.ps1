#requires -Version 7.0

function Get-BaselineSampleMessage {
    param([object[]]$Messages)

    $sample = @(@($Messages | Where-Object { -not [string]::IsNullOrWhiteSpace((Get-MessageText $_)) } | Select-Object -Last 1))
    if ($sample.Count -eq 0) {
        return $null
    }
    return $sample[0]
}

function Resolve-ReadOnlySummaryPath {
    param(
        [string]$RequestedPath,
        [string]$ScriptRoot
    )

    if (-not [string]::IsNullOrWhiteSpace($RequestedPath)) {
        $summaryDir = Split-Path -Parent $RequestedPath
        if ($summaryDir -and -not (Test-Path -LiteralPath $summaryDir)) {
            New-Item -ItemType Directory -Path $summaryDir -Force | Out-Null
        }
        return $RequestedPath
    }

    $root = Split-Path -Parent $ScriptRoot
    $summaryDir = Join-Path $root "target\fb2-ai-center"
    New-Item -ItemType Directory -Path $summaryDir -Force | Out-Null
    $stamp = (Get-Date).ToUniversalTime().ToString("yyyyMMddTHHmmssZ")
    return (Join-Path $summaryDir "read-only-direct-read-$stamp.json")
}

function Get-ReadOnlyMessageKind {
    param([object]$Message)

    if ($null -eq $Message) {
        return "unknown"
    }

    $id = [string]$Message.id
    if ($id.StartsWith("gai_", [System.StringComparison]::OrdinalIgnoreCase)) {
        return "ai_reply"
    }
    if ($id.StartsWith("gmsg_", [System.StringComparison]::OrdinalIgnoreCase)) {
        return "group_message"
    }
    if ($id.StartsWith("gsp_", [System.StringComparison]::OrdinalIgnoreCase)) {
        return "summary_post"
    }

    return "unknown"
}

function New-ReadOnlyMessageFingerprint {
    param(
        [object]$Message,
        [int]$Index
    )

    $text = Get-MessageText $Message
    [ordered]@{
        index = $Index
        message_id = if ($null -eq $Message) { "" } else { [string]$Message.id }
        kind = Get-ReadOnlyMessageKind $Message
        sender = Get-MessageSender $Message
        created_at = if ($null -eq $Message -or -not $Message.created_at) { "" } else { [string]$Message.created_at }
        text_len = $text.Length
        text_sha256 = Get-TextSha256 $text
    }
}

function New-ReadOnlyRecentMessageIndex {
    param(
        [object[]]$Messages,
        [int]$MaxMessages = 20
    )

    $items = @($Messages | Where-Object { $_ })
    if ($items.Count -eq 0) {
        return @()
    }

    $window = @($items | Select-Object -Last $MaxMessages)
    $result = New-Object System.Collections.Generic.List[object]
    for ($i = 0; $i -lt $window.Count; $i++) {
        [void]$result.Add((New-ReadOnlyMessageFingerprint -Message $window[$i] -Index $i))
    }
    return @($result.ToArray())
}

function Write-ReadOnlyDirectReadSummary {
    param(
        [string]$OutputPath,
        [object[]]$Messages,
        [object]$SampleMessage,
        [object]$ResolvedToken,
        [string]$ReadEvidence,
        [bool]$TextFingerprintComplete,
        [string]$StartedAt,
        [string]$CompletedAt,
        [string]$MainBase,
        [string]$Fb2Base,
        [string]$GroupId,
        [string]$Fb2UserId
    )

    $sampleText = if ($null -eq $SampleMessage) { "" } else { Get-MessageText $SampleMessage }
    $recentMessages = @(New-ReadOnlyRecentMessageIndex -Messages $Messages -MaxMessages 20)
    $recentAiMessages = @($recentMessages | Where-Object { [string]$_["kind"] -eq "ai_reply" })
    $latestAiMessage = @($recentAiMessages | Select-Object -Last 1)
    $summary = [ordered]@{
        schema = "fb2.main_project.visible_chat_readonly.v1"
        mode = "read_only_direct_read"
        writes = $false
        started_at = $StartedAt
        completed_at = $CompletedAt
        main_base = $MainBase
        fb2_base = $Fb2Base
        group_id = $GroupId
        token_source = [string]$ResolvedToken.Source
        fb2_user_id = $Fb2UserId
        message_count = @($Messages).Count
        sample_message_id = if ($null -eq $SampleMessage) { "" } else { [string]$SampleMessage.id }
        sample_sender = Get-MessageSender $SampleMessage
        sample_created_at = if ($null -eq $SampleMessage -or -not $SampleMessage.created_at) { "" } else { [string]$SampleMessage.created_at }
        sample_text_len = $sampleText.Length
        sample_text_sha256 = Get-TextSha256 $sampleText
        direct_read_evidence = $ReadEvidence
        direct_read_complete = (@($Messages).Count -gt 0 -and $TextFingerprintComplete)
        api = "/api/me/groups/{group_id}/messages"
        recent_message_limit = 20
        recent_message_count = $recentMessages.Count
        recent_ai_message_count = $recentAiMessages.Count
        latest_ai_message_id = if ($latestAiMessage.Count -eq 0) { "" } else { [string]$latestAiMessage[0]["message_id"] }
        recent_messages = @($recentMessages)
    }

    $summaryJson = $summary | ConvertTo-Json -Depth 8
    Set-Content -Path $OutputPath -Value $summaryJson -Encoding UTF8
    return $summary
}
