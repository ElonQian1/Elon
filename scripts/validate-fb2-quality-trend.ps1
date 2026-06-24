#requires -Version 7.0

param(
    [string]$SummaryPath = "",
    [string]$OutputPath = "",
    [double]$MaxLargeContextPackRate = 0.85,
    [int]$MinFeedbackCount = 2,
    [int]$MinMatchedCitedSourceCount = 2,
    [int]$MinNonSyntheticFeedbackCount = 1,
    [int]$MinOpinionAdoptionCount = 1,
    [int]$MinOpinionMemoryRefs = 1,
    [switch]$SelfTest
)

$ErrorActionPreference = "Stop"

function Get-Fb2QualityRepoRoot {
    Split-Path -Parent $PSScriptRoot
}

function Resolve-Fb2QualityPath {
    param(
        [string]$Path,
        [string]$Root
    )

    if ([string]::IsNullOrWhiteSpace($Path)) {
        return ""
    }
    if ([System.IO.Path]::IsPathRooted($Path)) {
        return $Path
    }
    return (Join-Path $Root $Path)
}

function Get-Fb2QualityProperty {
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
    if ($Object -is [System.Collections.Specialized.OrderedDictionary] -and $Object.Contains($Name)) {
        return $Object[$Name]
    }
    $property = $Object.PSObject.Properties[$Name]
    if ($null -eq $property) {
        return $Default
    }
    return $property.Value
}

function Get-LatestFb2QualitySummaryPath {
    param([string]$Root)

    $summaryDir = Join-Path $Root "target\fb2-ai-center"
    if (-not (Test-Path -LiteralPath $summaryDir)) {
        return ""
    }
    $latest = @(Get-ChildItem -LiteralPath $summaryDir -Filter "data-only-acceptance-*.json" -File -ErrorAction SilentlyContinue |
        Sort-Object LastWriteTimeUtc -Descending |
        Select-Object -First 1)
    if (@($latest).Count -eq 0) {
        return ""
    }
    return $latest[0].FullName
}

function Read-Fb2QualityJson {
    param([string]$Path)

    if ([string]::IsNullOrWhiteSpace($Path) -or -not (Test-Path -LiteralPath $Path)) {
        throw "Summary not found: $Path"
    }
    Get-Content -LiteralPath $Path -Raw | ConvertFrom-Json
}

function Add-Fb2QualityCheck {
    param(
        [System.Collections.ArrayList]$Checks,
        [string]$Name,
        [bool]$Passed,
        [string]$Details = ""
    )

    [void]$Checks.Add([ordered]@{
        name = $Name
        passed = $Passed
        details = $Details
    })
}

function Get-Fb2QualityMetricNumber {
    param(
        [object]$Evidence,
        [string]$Name
    )

    $raw = [string](Get-Fb2QualityProperty $Evidence $Name "")
    if ([string]::IsNullOrWhiteSpace($raw)) {
        return $null
    }
    if ($raw -match '\bvalue=(?<value>-?\d+(\.\d+)?)') {
        return [double]$Matches["value"]
    }
    if ($raw -match '^(?<value>-?\d+(\.\d+)?)$') {
        return [double]$Matches["value"]
    }
    return $null
}

function Get-Fb2QualityLogMetric {
    param(
        [string]$LogPath,
        [string]$MetricName
    )

    if ([string]::IsNullOrWhiteSpace($LogPath) -or -not (Test-Path -LiteralPath $LogPath)) {
        return [ordered]@{
            present = $false
            value = $null
            max = $null
            line = ""
        }
    }
    $line = @(Select-String -LiteralPath $LogPath -Pattern $MetricName -SimpleMatch -ErrorAction SilentlyContinue |
        Select-Object -Last 1).Line
    if ([string]::IsNullOrWhiteSpace($line)) {
        return [ordered]@{
            present = $false
            value = $null
            max = $null
            line = ""
        }
    }
    $value = $null
    $max = $null
    if ($line -match '\bvalue=(?<value>-?\d+(\.\d+)?)') {
        $value = [double]$Matches["value"]
    }
    if ($line -match '\bmax=(?<max>-?\d+(\.\d+)?)') {
        $max = [double]$Matches["max"]
    }
    [ordered]@{
        present = $true
        value = $value
        max = $max
        line = $line
    }
}

function Test-Fb2QualityAtLeast {
    param(
        [object]$Value,
        [double]$Minimum
    )

    return ($null -ne $Value -and [double]$Value -ge $Minimum)
}

function Test-Fb2QualityZero {
    param([object]$Value)

    return ($null -ne $Value -and [double]$Value -eq 0)
}

function Test-Fb2QualityDirectReadLine {
    param([object]$Line)

    $text = [string]$Line
    if ([string]::IsNullOrWhiteSpace($text)) {
        return $false
    }
    return (
        $text -match '\b(text_len|body_len)=\d+\b' -and
        $text -match '\btext_sha256=[0-9a-f]{64}\b'
    )
}

function Test-Fb2QualityLegacyDirectReadEvidence {
    param([object]$Summary)

    $evidence = Get-Fb2QualityProperty $Summary "visible_direct_read_evidence"
    if ($null -eq $evidence) {
        return $false
    }
    foreach ($key in @(
            "baseline_messages",
            "visible_mention_seed",
            "visible_mention_reply",
            "selected_message_seed",
            "selected_message_reply",
            "summary_post"
        )) {
        if (-not (Test-Fb2QualityDirectReadLine -Line (Get-Fb2QualityProperty $evidence $key ""))) {
            return $false
        }
    }
    return $true
}

function Get-Fb2QualityVisibleDirectReadState {
    param([object]$Summary)

    if ([bool](Get-Fb2QualityProperty $Summary "visible_direct_read_complete" $false)) {
        return [pscustomobject][ordered]@{
            complete = $true
            mode = "current_visible_direct_read_complete_gate"
        }
    }
    if (Test-Fb2QualityLegacyDirectReadEvidence -Summary $Summary) {
        return [pscustomobject][ordered]@{
            complete = $true
            mode = "legacy_visible_direct_read_evidence_object"
        }
    }
    return [pscustomobject][ordered]@{
        complete = $false
        mode = "missing_visible_direct_read_evidence"
    }
}

function New-Fb2QualityTrendValidation {
    param(
        [object]$Summary,
        [string]$SourcePath,
        [string]$RepoRoot
    )

    $checks = [System.Collections.ArrayList]::new()
    $evidence = Get-Fb2QualityProperty $Summary "final_acceptance_evidence"
    $coverage = Get-Fb2QualityProperty $Summary "feedback_coverage"
    $feedbackEvidence = @(Get-Fb2QualityProperty $Summary "feedback_evidence" @())
    $logPath = Resolve-Fb2QualityPath -Path ([string](Get-Fb2QualityProperty $Summary "final_acceptance_log_path" "")) -Root $RepoRoot

    $feedbackCount = Get-Fb2QualityMetricNumber -Evidence $evidence -Name "quality_feedback_count"
    $matchedCitedSources = Get-Fb2QualityMetricNumber -Evidence $evidence -Name "quality_matched_cited_sources"
    $unmatchedCitedSources = Get-Fb2QualityMetricNumber -Evidence $evidence -Name "quality_unmatched_cited_sources"
    $missingContextCount = Get-Fb2QualityMetricNumber -Evidence $evidence -Name "quality_missing_context_count"
    $wrongContextCount = Get-Fb2QualityMetricNumber -Evidence $evidence -Name "quality_wrong_context_count"
    $nonSyntheticFeedbackCount = Get-Fb2QualityMetricNumber -Evidence $evidence -Name "quality_non_synthetic_feedback_count"
    $nonSyntheticAdoptionCount = Get-Fb2QualityMetricNumber -Evidence $evidence -Name "quality_non_synthetic_adoption_count"
    $nonSyntheticMemoryRefs = Get-Fb2QualityMetricNumber -Evidence $evidence -Name "quality_non_synthetic_memory_refs"
    $largeContextPackRate = Get-Fb2QualityLogMetric -LogPath $logPath -MetricName "quality large_context_pack_rate"
    $visibleDirectReadState = Get-Fb2QualityVisibleDirectReadState -Summary $Summary

    Add-Fb2QualityCheck $checks "summary success" ([bool](Get-Fb2QualityProperty $Summary "success" $false))
    Add-Fb2QualityCheck $checks "data-only voice deferred" ([string](Get-Fb2QualityProperty $Summary "voice_status" "") -eq "deferred_by_user")
    Add-Fb2QualityCheck $checks "visible chat passed" ([int](Get-Fb2QualityProperty $Summary "visible_chat_exit_code" -1) -eq 0)
    Add-Fb2QualityCheck $checks "final acceptance data-only passed" ([int](Get-Fb2QualityProperty $Summary "final_acceptance_exit_code" -1) -eq 0)
    Add-Fb2QualityCheck $checks "visible direct read complete" ([bool]$visibleDirectReadState.complete) ([string]$visibleDirectReadState.mode)
    Add-Fb2QualityCheck $checks "feedback coverage complete" ([bool](Get-Fb2QualityProperty $coverage "complete" $false))
    Add-Fb2QualityCheck $checks "feedback coverage observed all required" ([int](Get-Fb2QualityProperty $coverage "observed_count" 0) -ge [int](Get-Fb2QualityProperty $coverage "required_count" 3))
    Add-Fb2QualityCheck $checks "feedback evidence count" (@($feedbackEvidence).Count -ge [int](Get-Fb2QualityProperty $coverage "required_count" 3)) "count=$(@($feedbackEvidence).Count)"
    Add-Fb2QualityCheck $checks "quality feedback count" (Test-Fb2QualityAtLeast -Value $feedbackCount -Minimum $MinFeedbackCount) "value=$feedbackCount min=$MinFeedbackCount"
    Add-Fb2QualityCheck $checks "quality matched cited sources" (Test-Fb2QualityAtLeast -Value $matchedCitedSources -Minimum $MinMatchedCitedSourceCount) "value=$matchedCitedSources min=$MinMatchedCitedSourceCount"
    Add-Fb2QualityCheck $checks "quality unmatched cited sources zero" (Test-Fb2QualityZero -Value $unmatchedCitedSources) "value=$unmatchedCitedSources"
    Add-Fb2QualityCheck $checks "quality missing context zero" (Test-Fb2QualityZero -Value $missingContextCount) "value=$missingContextCount"
    Add-Fb2QualityCheck $checks "quality wrong context zero" (Test-Fb2QualityZero -Value $wrongContextCount) "value=$wrongContextCount"
    Add-Fb2QualityCheck $checks "quality non-synthetic feedback count" (Test-Fb2QualityAtLeast -Value $nonSyntheticFeedbackCount -Minimum $MinNonSyntheticFeedbackCount) "value=$nonSyntheticFeedbackCount min=$MinNonSyntheticFeedbackCount"
    Add-Fb2QualityCheck $checks "quality non-synthetic adoption count" (Test-Fb2QualityAtLeast -Value $nonSyntheticAdoptionCount -Minimum $MinOpinionAdoptionCount) "value=$nonSyntheticAdoptionCount min=$MinOpinionAdoptionCount"
    Add-Fb2QualityCheck $checks "quality non-synthetic memory refs" (Test-Fb2QualityAtLeast -Value $nonSyntheticMemoryRefs -Minimum $MinOpinionMemoryRefs) "value=$nonSyntheticMemoryRefs min=$MinOpinionMemoryRefs"
    Add-Fb2QualityCheck $checks "quality large context metric present" ([bool]$largeContextPackRate.present) ([string]$largeContextPackRate.line)
    Add-Fb2QualityCheck $checks "quality large context rate within budget" ($null -ne $largeContextPackRate.value -and [double]$largeContextPackRate.value -le $MaxLargeContextPackRate) "value=$($largeContextPackRate.value) max=$MaxLargeContextPackRate"

    $failed = @($checks | Where-Object { -not [bool]$_.passed })
    [ordered]@{
        schema = "fb2.main_project.quality_trend_validation.v1"
        source_summary = $SourcePath
        source_log = $logPath
        success = (@($failed).Count -eq 0)
        check_count = @($checks).Count
        failed_count = @($failed).Count
        failed = @($failed)
        checks = @($checks)
        metrics = [ordered]@{
            feedback_count = $feedbackCount
            matched_cited_source_count = $matchedCitedSources
            unmatched_cited_source_count = $unmatchedCitedSources
            missing_context_count = $missingContextCount
            wrong_context_count = $wrongContextCount
            non_synthetic_feedback_count = $nonSyntheticFeedbackCount
            non_synthetic_adoption_count = $nonSyntheticAdoptionCount
            non_synthetic_memory_refs = $nonSyntheticMemoryRefs
            large_context_pack_rate = $largeContextPackRate.value
            max_large_context_pack_rate = $MaxLargeContextPackRate
            visible_direct_read_mode = [string]$visibleDirectReadState.mode
        }
        note = "Validates the latest non-voice fb2 AI Center quality trend from local acceptance artifacts: feedback coverage, source matching, missing/wrong context, opinion adoption, and large Context Pack rate. It does not read secrets or write group messages."
    }
}

function New-Fb2QualityFixtureSummary {
    param(
        [string]$LogPath,
        [bool]$Success = $true,
        [int]$Unmatched = 0,
        [int]$Missing = 0,
        [int]$Wrong = 0,
        [int]$FeedbackObserved = 3,
        [int]$Adoption = 1
    )

    [pscustomobject][ordered]@{
        schema = "fb2.main_project.final_acceptance_summary.v1"
        mode = "visible_data_only_acceptance"
        voice_status = "deferred_by_user"
        visible_chat_exit_code = 0
        final_acceptance_exit_code = 0
        final_acceptance_log_path = $LogPath
        visible_direct_read_complete = $true
        visible_direct_read_evidence = [pscustomobject][ordered]@{
            baseline_messages = "group=ext_fb2_official count=80 sample_message=gai_sample text_len=292 text_sha256=$('a' * 64)"
            visible_mention_seed = "group=ext_fb2_official message=gmsg_seed text_len=83 text_sha256=$('b' * 64)"
            visible_mention_reply = "group=ext_fb2_official message=gai_reply text_len=448 text_sha256=$('c' * 64)"
            selected_message_seed = "group=ext_fb2_official message=gmsg_selected text_len=71 text_sha256=$('d' * 64)"
            selected_message_reply = "group=ext_fb2_official message=gai_selected_reply text_len=292 text_sha256=$('e' * 64)"
            summary_post = "group=ext_fb2_official post=gsp_summary status=ready text_len=2291 text_sha256=$('f' * 64)"
        }
        feedback_evidence = @(
            [pscustomobject]@{ scenario = "visible @EL fb2 feedback"; feedback_id = "fb1" },
            [pscustomobject]@{ scenario = "selected-message AI回复 fb2 feedback"; feedback_id = "fb2" },
            [pscustomobject]@{ scenario = "summary-post fb2 feedback"; feedback_id = "fb3" }
        )
        feedback_coverage = [ordered]@{
            required_count = 3
            observed_count = $FeedbackObserved
            visible_mention = $true
            selected_message = $true
            summary_post = $true
            missing_required = @()
            complete = ($FeedbackObserved -ge 3)
        }
        final_acceptance_evidence = [ordered]@{
            quality_feedback_count = "value=3 min=2"
            quality_matched_cited_sources = "value=3 min=2"
            quality_unmatched_cited_sources = "value=$Unmatched"
            quality_missing_context_count = "value=$Missing"
            quality_wrong_context_count = "value=$Wrong"
            quality_non_synthetic_feedback_count = "value=3 min=1"
            quality_non_synthetic_adoption_count = "value=$Adoption min=1"
            quality_non_synthetic_memory_refs = "value=1"
        }
        success = $Success
    }
}

function Invoke-Fb2QualitySelfTest {
    $tempRoot = Join-Path ([System.IO.Path]::GetTempPath()) ("fb2-quality-trend-selftest-" + [guid]::NewGuid().ToString("N"))
    try {
        New-Item -ItemType Directory -Force -Path $tempRoot | Out-Null
        $goodLog = Join-Path $tempRoot "good.log"
        Set-Content -LiteralPath $goodLog -Value "OK`tquality large_context_pack_rate`tvalue=0.5 max=0.85" -Encoding UTF8
        $good = New-Fb2QualityFixtureSummary -LogPath $goodLog
        $goodResult = New-Fb2QualityTrendValidation -Summary $good -SourcePath "selftest-good.json" -RepoRoot $tempRoot
        $failed = 0
        if (-not [bool]$goodResult.success) {
            $goodResult | ConvertTo-Json -Depth 8
            $failed++
        }
        $legacyDirectRead = New-Fb2QualityFixtureSummary -LogPath $goodLog
        $legacyDirectRead.PSObject.Properties.Remove("visible_direct_read_complete")
        $legacyResult = New-Fb2QualityTrendValidation -Summary $legacyDirectRead -SourcePath "selftest-legacy-direct-read.json" -RepoRoot $tempRoot
        if (-not [bool]$legacyResult.success -or [string]$legacyResult.metrics.visible_direct_read_mode -ne "legacy_visible_direct_read_evidence_object") {
            $failed++
        }

        $cases = @(
            @{ name = "unmatched"; summary = (New-Fb2QualityFixtureSummary -LogPath $goodLog -Unmatched 1) },
            @{ name = "missing-context"; summary = (New-Fb2QualityFixtureSummary -LogPath $goodLog -Missing 1) },
            @{ name = "wrong-context"; summary = (New-Fb2QualityFixtureSummary -LogPath $goodLog -Wrong 1) },
            @{ name = "feedback-incomplete"; summary = (New-Fb2QualityFixtureSummary -LogPath $goodLog -FeedbackObserved 2) },
            @{ name = "adoption-missing"; summary = (New-Fb2QualityFixtureSummary -LogPath $goodLog -Adoption 0) }
        )
        foreach ($case in $cases) {
            $result = New-Fb2QualityTrendValidation -Summary $case.summary -SourcePath ("selftest-" + [string]$case.name + ".json") -RepoRoot $tempRoot
            if ([bool]$result.success) {
                $failed++
            }
        }

        $largeLog = Join-Path $tempRoot "large.log"
        Set-Content -LiteralPath $largeLog -Value "OK`tquality large_context_pack_rate`tvalue=0.95 max=0.85" -Encoding UTF8
        $large = New-Fb2QualityFixtureSummary -LogPath $largeLog
        $largeResult = New-Fb2QualityTrendValidation -Summary $large -SourcePath "selftest-large.json" -RepoRoot $tempRoot
        if ([bool]$largeResult.success) {
            $failed++
        }

        Write-Output "== SelfTest Summary =="
        Write-Output "failed=$failed"
        if ($failed -gt 0) {
            exit 1
        }
    } finally {
        if (Test-Path -LiteralPath $tempRoot) {
            Remove-Item -LiteralPath $tempRoot -Recurse -Force
        }
    }
}

if ($SelfTest) {
    Invoke-Fb2QualitySelfTest
    exit 0
}

$root = Get-Fb2QualityRepoRoot
if ([string]::IsNullOrWhiteSpace($SummaryPath)) {
    $SummaryPath = Get-LatestFb2QualitySummaryPath -Root $root
} else {
    $SummaryPath = Resolve-Fb2QualityPath -Path $SummaryPath -Root $root
}
if ([string]::IsNullOrWhiteSpace($SummaryPath)) {
    throw "No data-only acceptance summary found under target\fb2-ai-center."
}

$summary = Read-Fb2QualityJson -Path $SummaryPath
$result = New-Fb2QualityTrendValidation -Summary $summary -SourcePath $SummaryPath -RepoRoot $root

if (-not [string]::IsNullOrWhiteSpace($OutputPath)) {
    $OutputPath = Resolve-Fb2QualityPath -Path $OutputPath -Root $root
    $parent = Split-Path -Parent $OutputPath
    if (-not [string]::IsNullOrWhiteSpace($parent)) {
        New-Item -ItemType Directory -Force -Path $parent | Out-Null
    }
    $result | ConvertTo-Json -Depth 8 | Set-Content -LiteralPath $OutputPath -Encoding UTF8
}

$result | ConvertTo-Json -Depth 8
if (-not [bool]$result.success) {
    exit 1
}
