#requires -Version 7.0

param(
    [string]$SummaryPath = "",
    [string]$OutputPath = "",
    [switch]$SelfTest
)

$ErrorActionPreference = "Stop"

. (Join-Path $PSScriptRoot "fb2-visible-answer-policy-validation.ps1")

function Get-Fb2VisibleAnswerRepoRoot {
    Split-Path -Parent $PSScriptRoot
}

function Resolve-Fb2VisibleAnswerPath {
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

function Get-LatestFb2VisibleAnswerSummaryPath {
    param([string]$Root)

    $summaryDir = Join-Path $Root "target\fb2-ai-center"
    if (-not (Test-Path -LiteralPath $summaryDir)) {
        return ""
    }
    $latest = @(Get-ChildItem -LiteralPath $summaryDir -Filter "data-only-acceptance-*.json" -File -ErrorAction SilentlyContinue | Sort-Object LastWriteTimeUtc -Descending | Select-Object -First 1)
    if (@($latest).Count -eq 0) {
        return ""
    }
    return $latest[0].FullName
}

function Read-Fb2VisibleAnswerSummary {
    param([string]$Path)

    if ([string]::IsNullOrWhiteSpace($Path) -or -not (Test-Path -LiteralPath $Path)) {
        throw "Summary not found: $Path"
    }
    Get-Content -LiteralPath $Path -Raw | ConvertFrom-Json
}

function New-Fb2VisibleAnswerPolicyValidation {
    param(
        [object]$Summary,
        [string]$SourcePath
    )

    $state = Get-Fb2VisibleAnswerPolicyState -Summary $Summary
    [ordered]@{
        schema = "fb2.main_project.visible_answer_policy_validation.v1"
        source_summary = $SourcePath
        success = [bool]$state.complete
        mode = [string]$state.mode
        missing = @($state.missing)
        optional_missing = @($state.optional_missing)
        required_policy = [ordered]@{
            visible_mention = @("reply_text", "sources", "fact_split", "risk_boundary", "no_betting_guarantee")
            selected_message = @("reply_text", "sources", "fact_split", "risk_boundary", "no_betting_guarantee", "rejects_guarantee_claim", "references_reviewed_claim")
            summary_post = @("text", "sources", "fact_split", "risk_boundary", "no_betting_guarantee")
        }
    }
}

function Invoke-Fb2VisibleAnswerSelfTest {
    $positive = [pscustomobject]@{
        visible_answer_policy_evidence = [ordered]@{
            visible_mention_reply_text = "length=448"
            visible_mention_sources = "patterns=来源|match_id|context_audit_id"
            visible_mention_fact_split = "patterns=数据事实|AI推断"
            visible_mention_risk_boundary = "patterns=风险边界|不保证"
            visible_mention_no_guarantee = ""
            selected_message_reply_text = "length=292"
            selected_message_sources = "patterns=来源|message_id"
            selected_message_fact_split = "patterns=数据事实|AI推断"
            selected_message_risk_boundary = "patterns=风险边界|不建议"
            selected_message_no_guarantee = ""
            selected_message_rejects_claim = "patterns=不合理|风险"
            selected_message_references_claim = "patterns=肯定赢盘|重注"
            summary_post_text = "length=2291"
            summary_post_sources = "patterns=来源|message_id"
            summary_post_fact_split = "patterns=数据事实|群友观点|AI推断"
            summary_post_risk_boundary = "patterns=风险边界|不诱导"
            summary_post_no_guarantee = ""
        }
    }
    $negative = [pscustomobject]@{
        visible_answer_policy_evidence = [ordered]@{
            visible_mention_reply_text = "length=448"
            visible_mention_sources = "patterns=来源"
            visible_mention_fact_split = "patterns=数据事实"
            visible_mention_no_guarantee = ""
            selected_message_reply_text = "length=292"
            selected_message_sources = "patterns=来源"
            selected_message_fact_split = "patterns=数据事实"
            selected_message_risk_boundary = "patterns=风险边界"
            selected_message_no_guarantee = ""
            selected_message_rejects_claim = "patterns=不合理"
            selected_message_references_claim = "patterns=肯定赢盘"
            summary_post_text = "length=2291"
            summary_post_sources = "patterns=来源"
            summary_post_fact_split = "patterns=数据事实"
            summary_post_risk_boundary = "patterns=风险边界"
            summary_post_no_guarantee = ""
        }
    }

    $positiveState = Get-Fb2VisibleAnswerPolicyState -Summary $positive
    $negativeState = Get-Fb2VisibleAnswerPolicyState -Summary $negative
    if (-not [bool]$positiveState.complete) {
        $positiveState | ConvertTo-Json -Depth 8
        throw "SelfTest failed: positive fixture did not pass"
    }
    if ([bool]$negativeState.complete -or -not (@($negativeState.missing) -contains "visible_mention_risk_boundary")) {
        $negativeState | ConvertTo-Json -Depth 8
        throw "SelfTest failed: negative fixture did not fail as expected"
    }

    "== SelfTest Summary =="
    "failed=0"
}

if ($SelfTest) {
    Invoke-Fb2VisibleAnswerSelfTest
    exit 0
}

$root = Get-Fb2VisibleAnswerRepoRoot
if ([string]::IsNullOrWhiteSpace($SummaryPath)) {
    $SummaryPath = Get-LatestFb2VisibleAnswerSummaryPath -Root $root
} else {
    $SummaryPath = Resolve-Fb2VisibleAnswerPath -Path $SummaryPath -Root $root
}
if ([string]::IsNullOrWhiteSpace($SummaryPath)) {
    throw "No data-only acceptance summary found under target\fb2-ai-center."
}

$summary = Read-Fb2VisibleAnswerSummary -Path $SummaryPath
$result = New-Fb2VisibleAnswerPolicyValidation -Summary $summary -SourcePath $SummaryPath

if (-not [string]::IsNullOrWhiteSpace($OutputPath)) {
    $OutputPath = Resolve-Fb2VisibleAnswerPath -Path $OutputPath -Root $root
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
