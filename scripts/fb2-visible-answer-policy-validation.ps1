#requires -Version 7.0

function Get-Fb2VisibleAnswerJsonProperty {
    param(
        [object]$Object,
        [string]$Name,
        [object]$Default = $null
    )

    if ($null -eq $Object) {
        return $Default
    }
    if ($Object -is [System.Collections.IDictionary] -and $Object.Contains($Name)) {
        return $Object[$Name]
    }
    $property = $Object.PSObject.Properties[$Name]
    if ($null -eq $property) {
        return $Default
    }
    return $property.Value
}

function Test-Fb2VisibleAnswerJsonPropertyExists {
    param(
        [object]$Object,
        [string]$Name
    )

    if ($null -eq $Object) {
        return $false
    }
    if ($Object -is [System.Collections.IDictionary]) {
        return $Object.Contains($Name)
    }
    return ($null -ne $Object.PSObject.Properties[$Name])
}

function Test-Fb2VisibleAnswerTextPresent {
    param([object]$Value)

    return -not [string]::IsNullOrWhiteSpace([string]$Value)
}

function Test-Fb2VisibleAnswerLengthEvidence {
    param([object]$Value)

    $text = [string]$Value
    if ([string]::IsNullOrWhiteSpace($text)) {
        return $false
    }
    if ($text -notmatch "\blength=\d+\b") {
        return $false
    }
    $match = [regex]::Match($text, "\blength=(\d+)\b")
    return ([int]$match.Groups[1].Value -gt 0)
}

function Add-Fb2VisibleAnswerMissing {
    param(
        [System.Collections.Generic.List[string]]$Missing,
        [object]$Evidence,
        [string]$Field,
        [string]$Mode = "non_empty"
    )

    $exists = Test-Fb2VisibleAnswerJsonPropertyExists -Object $Evidence -Name $Field
    $value = Get-Fb2VisibleAnswerJsonProperty $Evidence $Field
    $ok = switch ($Mode) {
        "exists" { $exists }
        "length" { $exists -and (Test-Fb2VisibleAnswerLengthEvidence $value) }
        default { $exists -and (Test-Fb2VisibleAnswerTextPresent $value) }
    }
    if (-not $ok) {
        $Missing.Add($Field) | Out-Null
    }
}

function Get-Fb2VisibleAnswerPolicyState {
    param([object]$Summary)

    if ($null -eq $Summary) {
        return [ordered]@{
            complete = $false
            mode = "missing_summary"
            missing = @("summary")
            optional_missing = @()
        }
    }

    $evidence = Get-Fb2VisibleAnswerJsonProperty $Summary "visible_answer_policy_evidence"
    if ($null -eq $evidence) {
        return [ordered]@{
            complete = $false
            mode = "missing_visible_answer_policy_evidence"
            missing = @("visible_answer_policy_evidence")
            optional_missing = @()
        }
    }

    $missing = [System.Collections.Generic.List[string]]::new()
    $optionalMissing = [System.Collections.Generic.List[string]]::new()

    foreach ($field in @(
            "visible_mention_reply_text",
            "selected_message_reply_text",
            "summary_post_text"
        )) {
        Add-Fb2VisibleAnswerMissing -Missing $missing -Evidence $evidence -Field $field -Mode "length"
    }

    foreach ($field in @(
            "visible_mention_sources",
            "visible_mention_fact_split",
            "visible_mention_risk_boundary",
            "selected_message_sources",
            "selected_message_fact_split",
            "selected_message_risk_boundary",
            "selected_message_rejects_claim",
            "selected_message_references_claim",
            "summary_post_sources",
            "summary_post_fact_split",
            "summary_post_risk_boundary"
        )) {
        Add-Fb2VisibleAnswerMissing -Missing $missing -Evidence $evidence -Field $field
    }

    foreach ($field in @(
            "visible_mention_no_guarantee",
            "selected_message_no_guarantee",
            "summary_post_no_guarantee"
        )) {
        Add-Fb2VisibleAnswerMissing -Missing $missing -Evidence $evidence -Field $field -Mode "exists"
    }

    foreach ($field in @("summary_post_model_ready")) {
        if (-not (Test-Fb2VisibleAnswerJsonPropertyExists -Object $evidence -Name $field)) {
            $optionalMissing.Add($field) | Out-Null
        }
    }

    [ordered]@{
        complete = ($missing.Count -eq 0)
        mode = "final_acceptance_visible_answer_policy_evidence"
        missing = @($missing)
        optional_missing = @($optionalMissing)
    }
}
