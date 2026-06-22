#requires -Version 7.0

function Get-Fb2DataOnlyJsonProperty {
    param(
        [object]$Object,
        [string]$Name,
        [object]$Default = $null
    )

    if ($null -eq $Object) {
        return $Default
    }
    $property = $Object.PSObject.Properties[$Name]
    if ($null -eq $property) {
        return $Default
    }
    return $property.Value
}

function Test-Fb2DataOnlyTruthyJsonValue {
    param([object]$Value)

    if ($null -eq $Value) {
        return $false
    }
    if ($Value -is [bool]) {
        return [bool]$Value
    }
    return ([string]$Value) -match "^(true|True|1)$"
}

function Test-Fb2DataOnlyDirectReadEvidenceFingerprint {
    param([object]$Value)

    $text = [string]$Value
    if ([string]::IsNullOrWhiteSpace($text)) {
        return $false
    }
    if ($text -notmatch "\btext_len=\d+\b") {
        return $false
    }
    if ($text -notmatch "\btext_sha256=[0-9a-fA-F]{8,}\b") {
        return $false
    }
    return $true
}

function Get-Fb2DataOnlyDirectReadEvidenceState {
    param([object]$Summary)

    if ($null -eq $Summary) {
        return [ordered]@{
            complete = $false
            mode = "missing_summary"
            missing = @("summary")
        }
    }
    if (Test-Fb2DataOnlyTruthyJsonValue (Get-Fb2DataOnlyJsonProperty $Summary "visible_direct_read_complete")) {
        return [ordered]@{
            complete = $true
            mode = "current_boolean_gate"
            missing = @()
        }
    }

    $evidence = Get-Fb2DataOnlyJsonProperty $Summary "visible_direct_read_evidence"
    if ($null -eq $evidence) {
        return [ordered]@{
            complete = $false
            mode = "missing_legacy_evidence_object"
            missing = @("visible_direct_read_evidence")
        }
    }

    $requiredFields = @(
        "baseline_messages",
        "visible_mention_seed",
        "visible_mention_reply",
        "selected_message_seed",
        "selected_message_reply",
        "summary_post"
    )
    $missing = @()
    foreach ($field in $requiredFields) {
        if (-not (Test-Fb2DataOnlyDirectReadEvidenceFingerprint (Get-Fb2DataOnlyJsonProperty $evidence $field))) {
            $missing += $field
        }
    }

    [ordered]@{
        complete = ($missing.Count -eq 0)
        mode = "legacy_evidence_object"
        missing = $missing
    }
}
