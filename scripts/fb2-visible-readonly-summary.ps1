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
    }

    $summaryJson = $summary | ConvertTo-Json -Depth 8
    Set-Content -Path $OutputPath -Value $summaryJson -Encoding UTF8
    return $summary
}
