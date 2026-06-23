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

function Test-ReadOnlyDirectReadRawBodyFieldFree {
    param([object]$Value)

    if ($null -eq $Value) {
        return $true
    }

    $forbiddenNames = @(
        "text",
        "body",
        "content",
        "message",
        "message_text",
        "raw_text",
        "sample_text",
        "full_text"
    )

    foreach ($property in @($Value.PSObject.Properties)) {
        if (@($forbiddenNames) -contains $property.Name) {
            return $false
        }
    }
    return $true
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
    if (-not (Test-ReadOnlyDirectReadRawBodyFieldFree $Summary)) {
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
            if (-not (Test-ReadOnlyDirectReadRawBodyFieldFree $message)) {
                return $false
            }
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

function New-ReadOnlyDirectReadValidation {
    param(
        [object]$Summary,
        [string]$SourcePath
    )

    $complete = Test-ReadOnlyDirectReadSummaryComplete $Summary
    $evidence = if ($complete) { Build-ReadOnlyDirectReadEvidence $Summary } else { $null }
    [ordered]@{
        schema = "fb2.main_project.visible_chat_readonly_validation.v1"
        source_summary = $SourcePath
        success = [bool]$complete
        evidence = $evidence
        checks = [ordered]@{
            schema = ([string]$Summary.schema -eq "fb2.main_project.visible_chat_readonly.v1")
            mode = ([string]$Summary.mode -eq "read_only_direct_read")
            writes_false = (-not [bool]$Summary.writes)
            direct_read_complete = [bool]$Summary.direct_read_complete
            message_count_positive = ([int]$Summary.message_count -gt 0)
            sample_text_len_positive = ([int]$Summary.sample_text_len -gt 0)
            sample_text_sha256_present = ([string]$Summary.sample_text_sha256 -match "^[0-9a-fA-F]{8,}$")
            raw_body_field_free = (Test-ReadOnlyDirectReadRawBodyFieldFree $Summary)
            recent_messages_count_matches = (
                $null -eq $Summary.PSObject.Properties["recent_messages"] -or
                [int]$Summary.recent_message_count -eq @($Summary.recent_messages).Count
            )
        }
    }
}

function Invoke-ReadOnlyDirectReadValidationSelfTest {
    $valid = [pscustomobject]@{
        schema = "fb2.main_project.visible_chat_readonly.v1"
        mode = "read_only_direct_read"
        writes = $false
        api = "/api/me/groups/{group_id}/messages"
        group_id = "ext_fb2_official"
        direct_read_complete = $true
        message_count = 1
        sample_message_id = "gai_readonly"
        sample_sender = "usr_elon_ai"
        sample_text_len = 12
        sample_text_sha256 = "abcdef0123456789"
        direct_read_evidence = "group=ext_fb2_official count=1 sample_message=gai_readonly text_len=12 text_sha256=abcdef0123456789"
        recent_message_count = 1
        recent_ai_message_count = 1
        latest_ai_message_id = "gai_readonly"
        recent_messages = @(
            [pscustomobject]@{
                index = 0
                message_id = "gai_readonly"
                kind = "ai_reply"
                sender = "usr_elon_ai"
                created_at = "2026-06-23T00:00:00Z"
                text_len = 12
                text_sha256 = "abcdef0123456789"
            }
        )
    }
    $failed = 0
    if (-not (Test-ReadOnlyDirectReadSummaryComplete $valid)) { $failed++ }
    if (-not [bool](New-ReadOnlyDirectReadValidation -Summary $valid -SourcePath "selftest-valid.json").success) { $failed++ }

    $withWrite = $valid | ConvertTo-Json -Depth 8 | ConvertFrom-Json
    $withWrite.writes = $true
    if (Test-ReadOnlyDirectReadSummaryComplete $withWrite) { $failed++ }

    $missingHash = $valid | ConvertTo-Json -Depth 8 | ConvertFrom-Json
    $missingHash.sample_text_sha256 = ""
    if (Test-ReadOnlyDirectReadSummaryComplete $missingHash) { $failed++ }

    $rawBody = $valid | ConvertTo-Json -Depth 8 | ConvertFrom-Json
    Add-Member -InputObject $rawBody -NotePropertyName "text" -NotePropertyValue "raw message body"
    if (Test-ReadOnlyDirectReadSummaryComplete $rawBody) { $failed++ }

    $recentRawBody = $valid | ConvertTo-Json -Depth 8 | ConvertFrom-Json
    Add-Member -InputObject $recentRawBody.recent_messages[0] -NotePropertyName "content" -NotePropertyValue "raw message body"
    if (Test-ReadOnlyDirectReadSummaryComplete $recentRawBody) { $failed++ }

    Write-Output "== SelfTest Summary =="
    Write-Output "failed=$failed"
    if ($failed -gt 0) {
        exit 1
    }
}
