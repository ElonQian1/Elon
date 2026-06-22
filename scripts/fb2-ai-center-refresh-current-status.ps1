#requires -Version 7.0

param(
    [string]$OutputDir = "",
    [string[]]$EvidenceDirs = @(),
    [string]$MainWorkspaceEvidenceDir = "",
    [string]$RefreshSummaryPath = "",
    [switch]$SkipPublicContract,
    [switch]$SelfTest
)

$ErrorActionPreference = "Stop"

function Get-Fb2RefreshRepoRoot {
    Split-Path -Parent $PSScriptRoot
}

function Resolve-Fb2RefreshPath {
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

function Add-Fb2RefreshDirectory {
    param(
        [System.Collections.ArrayList]$Target,
        [hashtable]$Seen,
        [string]$Directory,
        [string]$Root,
        [switch]$RequireExists
    )

    $path = Resolve-Fb2RefreshPath -Path $Directory -Root $Root
    if ([string]::IsNullOrWhiteSpace($path)) {
        return
    }
    if ($RequireExists -and -not (Test-Path -LiteralPath $path)) {
        return
    }
    try {
        $fullPath = [System.IO.Path]::GetFullPath($path)
    } catch {
        $fullPath = $path
    }
    $key = $fullPath.ToLowerInvariant()
    if (-not $Seen.ContainsKey($key)) {
        $Seen[$key] = $true
        [void]$Target.Add($fullPath)
    }
}

function Read-Fb2RefreshJson {
    param([string]$Path)

    if ([string]::IsNullOrWhiteSpace($Path) -or -not (Test-Path -LiteralPath $Path)) {
        return $null
    }
    Get-Content -LiteralPath $Path -Raw | ConvertFrom-Json
}

function Get-Fb2RefreshProperty {
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

function Get-Fb2RefreshCommandValue {
    param(
        [object]$Primary,
        [object]$Fallback,
        [string]$Name
    )

    $value = [string](Get-Fb2RefreshProperty $Primary $Name "")
    if (-not [string]::IsNullOrWhiteSpace($value)) {
        return $value
    }
    return [string](Get-Fb2RefreshProperty $Fallback $Name "")
}

function New-Fb2RefreshOwnerActions {
    param(
        [object]$Status,
        [object]$GoalAudit
    )

    $tokenPresent = [bool]$Status.environment.fb2_ai_center_token_present
    $dataGoalComplete = [bool]$GoalAudit.data_goal_complete
    $fullFinalComplete = [bool]$GoalAudit.full_final_complete

    [ordered]@{
        main_project = if ($dataGoalComplete -and -not $tokenPresent) {
            "keep_contract_and_status_regressions_green_until_FB2_AI_CENTER_TOKEN_is_available"
        } else {
            "refresh_status_goal_audit_and_handoff_after_each_contract_or_smoke_change"
        }
        fb2_project = if (-not $tokenPresent) {
            "provide_FB2_AI_CENTER_TOKEN_or_export_equivalent_live_Context_Pack_permission_quality_evidence"
        } else {
            "keep_live_context_pack_orders_platform_summary_group_opinion_and_feedback_endpoints_current"
        }
        shared = if ($fullFinalComplete) {
            "final_acceptance_complete"
        } elseif ($dataGoalComplete) {
            "run_DataOnlyAcceptance_PreflightOnly_with_token_then_refresh_status_refresh_current_json"
        } else {
            "close_missing_non_voice_requirements_before_visible_or_full_final_acceptance"
        }
    }
}

function New-Fb2RefreshNextCommands {
    param([object]$Status)

    $livePreflight = Get-Fb2RefreshProperty $Status "live_preflight_request"
    $liveCommands = Get-Fb2RefreshProperty $livePreflight "commands"
    $coordination = Get-Fb2RefreshProperty $Status "coordination"
    $safeCommands = Get-Fb2RefreshProperty $coordination "safe_commands"

    [ordered]@{
        refresh_status = "pwsh -NoProfile -ExecutionPolicy Bypass -File scripts\fb2-ai-center-refresh-current-status.ps1"
        read_status_refresh = "Get-Content -Raw -LiteralPath target\fb2-ai-center\status-refresh-current.json | ConvertFrom-Json"
        no_write_direct_read = Get-Fb2RefreshCommandValue -Primary $liveCommands -Fallback $safeCommands -Name "no_write_direct_read"
        data_only_preflight = Get-Fb2RefreshCommandValue -Primary $liveCommands -Fallback $safeCommands -Name "data_only_preflight"
        visible_regression_requires_authorization = Get-Fb2RefreshCommandValue -Primary $liveCommands -Fallback $safeCommands -Name "visible_regression_requires_authorization"
    }
}

function New-Fb2RefreshBlockingState {
    param(
        [object]$Status,
        [object]$GoalAudit
    )

    [ordered]@{
        blocked_by_external_secret = -not [bool]$Status.environment.fb2_ai_center_token_present
        external_secret = "FB2_AI_CENTER_TOKEN"
        deferred_by_user = @($Status.goal_gap_audit.deferred_by_user)
        safe_to_continue_without_secret = @(
            "public_contract_regression",
            "status_refresh_selftest",
            "offline_context_pack_sample_validation",
            "handoff_documentation"
        )
        requires_secret = @(
            "live_context_pack_permission_quality_refresh",
            "current_user_order_live_verification",
            "platform_order_summary_live_verification",
            "feedback_quality_live_refresh"
        )
        next_minimum_action = [string]$GoalAudit.next_minimum_action
    }
}

function Assert-Fb2RefreshSelfTest {
    param(
        [bool]$Condition,
        [string]$Message
    )

    if (-not $Condition) {
        throw "SelfTest failed: $Message"
    }
}

function Invoke-Fb2RefreshSelfTest {
    $tempRoot = Join-Path ([System.IO.Path]::GetTempPath()) ("fb2-ai-center-refresh-selftest-" + [guid]::NewGuid().ToString("N"))
    $output = Join-Path $tempRoot "out"
    $missingEvidence = Join-Path $tempRoot "missing-evidence"
    try {
        New-Item -ItemType Directory -Force -Path $output | Out-Null
        # 自测必须无网络、无 token、无主工作区依赖；这里只验证编排脚本能生成稳定机器摘要。
        $raw = & $PSCommandPath -OutputDir $output -MainWorkspaceEvidenceDir $missingEvidence -SkipPublicContract
        $summary = $raw | ConvertFrom-Json
        Assert-Fb2RefreshSelfTest ($summary.schema -eq "fb2.main_project.status_refresh.v1") "schema"
        Assert-Fb2RefreshSelfTest ([string]$summary.output_dir -eq $output) "output_dir"
        Assert-Fb2RefreshSelfTest (@($summary.evidence_dirs).Count -eq 1) "isolated evidence dirs"
        Assert-Fb2RefreshSelfTest (-not [bool]$summary.public_contract_ready) "public contract skipped"
        Assert-Fb2RefreshSelfTest (Test-Path -LiteralPath ([string]$summary.files.status)) "status file exists"
        Assert-Fb2RefreshSelfTest (Test-Path -LiteralPath ([string]$summary.files.goal_audit)) "goal audit file exists"
        Assert-Fb2RefreshSelfTest (Test-Path -LiteralPath ([string]$summary.files.handoff_markdown)) "handoff markdown exists"
        Assert-Fb2RefreshSelfTest (Test-Path -LiteralPath ([string]$summary.files.status_refresh)) "status refresh file exists"
        Assert-Fb2RefreshSelfTest (-not [string]::IsNullOrWhiteSpace([string]$summary.owner_next_actions.main_project)) "main owner action"
        Assert-Fb2RefreshSelfTest (-not [string]::IsNullOrWhiteSpace([string]$summary.owner_next_actions.fb2_project)) "fb2 owner action"
        Assert-Fb2RefreshSelfTest ([bool]$summary.blocking_state.blocked_by_external_secret) "selftest token blocked"
        Assert-Fb2RefreshSelfTest ([string]$summary.blocking_state.external_secret -eq "FB2_AI_CENTER_TOKEN") "external secret name"
        Assert-Fb2RefreshSelfTest (-not [string]::IsNullOrWhiteSpace([string]$summary.next_commands.refresh_status)) "refresh command"
        Assert-Fb2RefreshSelfTest (-not [string]::IsNullOrWhiteSpace([string]$summary.next_commands.data_only_preflight)) "data-only preflight command"
        Assert-Fb2RefreshSelfTest (-not [string]::IsNullOrWhiteSpace([string]$summary.next_minimum_action)) "next action"
        "== SelfTest Summary =="
        "failed=0"
    } finally {
        if (Test-Path -LiteralPath $tempRoot) {
            Remove-Item -LiteralPath $tempRoot -Recurse -Force
        }
    }
}

if ($SelfTest) {
    Invoke-Fb2RefreshSelfTest
    exit 0
}

$root = Get-Fb2RefreshRepoRoot
if ([string]::IsNullOrWhiteSpace($OutputDir)) {
    $OutputDir = Join-Path $root "target\fb2-ai-center"
} else {
    $OutputDir = Resolve-Fb2RefreshPath -Path $OutputDir -Root $root
}
New-Item -ItemType Directory -Force -Path $OutputDir | Out-Null

if ([string]::IsNullOrWhiteSpace($RefreshSummaryPath)) {
    $RefreshSummaryPath = Join-Path $OutputDir "status-refresh-current.json"
} else {
    $RefreshSummaryPath = Resolve-Fb2RefreshPath -Path $RefreshSummaryPath -Root $root
}
$refreshParent = Split-Path -Parent $RefreshSummaryPath
if (-not [string]::IsNullOrWhiteSpace($refreshParent)) {
    New-Item -ItemType Directory -Force -Path $refreshParent | Out-Null
}

$evidence = [System.Collections.ArrayList]::new()
$seen = @{}

# 当前 worktree 的 target 永远排第一；额外目录只作为历史证据兜底，不覆盖更新的当前产物。
Add-Fb2RefreshDirectory -Target $evidence -Seen $seen -Directory $OutputDir -Root $root
foreach ($dir in @($EvidenceDirs)) {
    Add-Fb2RefreshDirectory -Target $evidence -Seen $seen -Directory $dir -Root $root -RequireExists
}

if ([string]::IsNullOrWhiteSpace($MainWorkspaceEvidenceDir)) {
    $MainWorkspaceEvidenceDir = "D:\rust\active-projects\elon cli\target\fb2-ai-center"
}
Add-Fb2RefreshDirectory -Target $evidence -Seen $seen -Directory $MainWorkspaceEvidenceDir -Root $root -RequireExists

$publicPath = Join-Path $OutputDir "public-contract-status-current.json"
$statusPath = Join-Path $OutputDir "status-current.json"
$goalAuditPath = Join-Path $OutputDir "goal-audit-current.json"
$goalAuditMarkdownPath = Join-Path $OutputDir "goal-audit-current.md"
$handoffPath = Join-Path $OutputDir "handoff-current.json"
$handoffMarkdownPath = Join-Path $OutputDir "handoff-current.md"

if (-not $SkipPublicContract) {
    & (Join-Path $PSScriptRoot "fb2-public-contract-status.ps1") -OutputPath $publicPath | Out-Null
}

& (Join-Path $PSScriptRoot "smoke-fb2-ai-center-status.ps1") `
    -SummaryDir $OutputDir `
    -EvidenceDirs @($evidence) `
    -OutputPath $statusPath | Out-Null

& (Join-Path $PSScriptRoot "fb2-ai-center-goal-audit-report.ps1") `
    -StatusPath $statusPath `
    -OutputPath $goalAuditPath `
    -MarkdownPath $goalAuditMarkdownPath | Out-Null

& (Join-Path $PSScriptRoot "fb2-ai-center-handoff-report.ps1") `
    -StatusPath $statusPath `
    -OutputPath $handoffPath `
    -MarkdownPath $handoffMarkdownPath | Out-Null

$status = Read-Fb2RefreshJson -Path $statusPath
$goalAudit = Read-Fb2RefreshJson -Path $goalAuditPath
$public = Read-Fb2RefreshJson -Path $publicPath
$ownerNextActions = New-Fb2RefreshOwnerActions -Status $status -GoalAudit $goalAudit
$blockingState = New-Fb2RefreshBlockingState -Status $status -GoalAudit $goalAudit
$nextCommands = New-Fb2RefreshNextCommands -Status $status

$refreshSummary = [pscustomobject]@{
    schema = "fb2.main_project.status_refresh.v1"
    output_dir = $OutputDir
    evidence_dirs = @($evidence)
    files = [ordered]@{
        status_refresh = $RefreshSummaryPath
        public_contract_status = $publicPath
        status = $statusPath
        goal_audit = $goalAuditPath
        goal_audit_markdown = $goalAuditMarkdownPath
        handoff = $handoffPath
        handoff_markdown = $handoffMarkdownPath
    }
    public_contract_ready = [bool]($public -and $public.success)
    user_scenario_audit_ready = [bool]$status.latest_user_scenario_audit.complete
    non_voice_historical_evidence_ready = [bool]$status.readiness.non_voice_historical_evidence_ready
    data_goal_complete = [bool]$goalAudit.data_goal_complete
    full_final_complete = [bool]$goalAudit.full_final_complete
    token_present = [bool]$status.environment.fb2_ai_center_token_present
    next_minimum_action = [string]$goalAudit.next_minimum_action
    owner_next_actions = $ownerNextActions
    blocking_state = $blockingState
    next_commands = $nextCommands
    missing_non_voice_requirements = @($goalAudit.missing_non_voice_requirements)
    deferred_requirements = @($goalAudit.deferred_requirements)
}

$refreshJson = $refreshSummary | ConvertTo-Json -Depth 8
Set-Content -LiteralPath $RefreshSummaryPath -Value $refreshJson -Encoding UTF8
$refreshJson
