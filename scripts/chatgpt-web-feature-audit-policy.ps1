#requires -Version 5.1

function Test-ChatGptWebFeatureMatrix {
    param([Parameter(Mandatory = $true)]$Matrix)

    $reasons = [System.Collections.Generic.List[string]]::new()
    $manifest = $Matrix.manifest
    if ($Matrix.control_ok -ne $true) { $reasons.Add("matrix_control_failed") }
    if ($Matrix.ready_for_mcp -ne $true) { $reasons.Add("mcp_not_ready") }
    if ([string]$manifest.page_kind -ne "feature") { $reasons.Add("not_feature_page") }
    if ([string]$manifest.compatibility -ne "healthy") { $reasons.Add("manifest_not_healthy") }
    if ($manifest.controls_truncated -eq $true) { $reasons.Add("manifest_controls_truncated") }
    if ([int]$manifest.generic_control_count -ne 0) { $reasons.Add("generic_controls_present") }
    if ([int]$manifest.unexpected_official_fallback_control_count -ne 0) {
        $reasons.Add("unexpected_official_fallback_controls_present")
    }
    if (@($Matrix.unknown_semantics).Count -ne 0) { $reasons.Add("unknown_semantics") }
    if (@($Matrix.unknown_capabilities).Count -ne 0) { $reasons.Add("unknown_capabilities") }
    if ($Matrix.adaptation_review.required -eq $true) { $reasons.Add("adaptation_review_required") }

    return [pscustomobject]@{
        passed = $reasons.Count -eq 0
        reasons = @($reasons)
        control_count = [int]$manifest.control_count
        native_control_count = [int]$manifest.native_control_count
        generic_control_count = [int]$manifest.generic_control_count
        unexpected_fallback_count = [int]$manifest.unexpected_official_fallback_control_count
    }
}
