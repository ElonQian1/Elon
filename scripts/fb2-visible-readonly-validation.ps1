#requires -Version 7.0

function Read-JsonFileOrNull {
    param([string]$Path)

    if ([string]::IsNullOrWhiteSpace($Path)) {
        return $null
    }
    if (-not (Test-Path -LiteralPath $Path)) {
        return $null
    }

    try {
        Get-Content -Raw -LiteralPath $Path | ConvertFrom-Json
    } catch {
        $null
    }
}

function Build-ReadOnlyDirectReadEvidence {
    param([object]$Summary)

    [ordered]@{
        schema = [string]$Summary.schema
        api = [string]$Summary.api
        group_id = [string]$Summary.group_id
        writes = [bool]$Summary.writes
        direct_read_complete = [bool]$Summary.direct_read_complete
        message_count = [int]$Summary.message_count
        sample_message_id = [string]$Summary.sample_message_id
        sample_sender = [string]$Summary.sample_sender
        sample_text_len = [int]$Summary.sample_text_len
        sample_text_sha256 = [string]$Summary.sample_text_sha256
        direct_read_evidence = [string]$Summary.direct_read_evidence
        recent_message_count = [int]$Summary.recent_message_count
        recent_ai_message_count = [int]$Summary.recent_ai_message_count
        latest_ai_message_id = [string]$Summary.latest_ai_message_id
    }
}

function Test-ReadOnlyDirectReadSummaryComplete {
    param([object]$Summary)

    if ($null -eq $Summary) {
        return $false
    }
    if ([string]$Summary.schema -ne "fb2.main_project.visible_chat_readonly.v1") {
        return $false
    }
    if ([string]$Summary.mode -ne "read_only_direct_read") {
        return $false
    }
    if ([bool]$Summary.writes) {
        return $false
    }
    if (-not [bool]$Summary.direct_read_complete) {
        return $false
    }
    if ([int]$Summary.message_count -lt 1) {
        return $false
    }
    foreach ($field in @("group_id", "sample_message_id", "sample_sender", "direct_read_evidence", "api")) {
        if ([string]::IsNullOrWhiteSpace([string]$Summary.$field)) {
            return $false
        }
    }
    if ([int]$Summary.sample_text_len -lt 1) {
        return $false
    }
    if ([string]$Summary.sample_text_sha256 -notmatch "^[0-9a-fA-F]{8,}$") {
        return $false
    }

    # 只读预检必须带正文指纹，避免只用消息 ID 或截图冒充接口回读。
    $evidence = [string]$Summary.direct_read_evidence
    if ($evidence -notmatch "\btext_len=\d+\b") {
        return $false
    }
    if ($evidence -notmatch "\btext_sha256=[0-9a-fA-F]{8,}\b") {
        return $false
    }

    $recentMessagesProperty = $Summary.PSObject.Properties["recent_messages"]
    if ($null -ne $recentMessagesProperty) {
        $recentMessages = @($Summary.recent_messages)
        if ([int]$Summary.recent_message_count -ne $recentMessages.Count) {
            return $false
        }
        foreach ($message in $recentMessages) {
            if ([string]::IsNullOrWhiteSpace([string]$message.message_id)) {
                return $false
            }
            if ([int]$message.text_len -lt 1) {
                return $false
            }
            if ([string]$message.text_sha256 -notmatch "^[0-9a-fA-F]{8,}$") {
                return $false
            }
        }
    }

    return $true
}
