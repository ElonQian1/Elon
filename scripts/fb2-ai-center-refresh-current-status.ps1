#requires -Version 7.0

param(
    [string]$OutputDir = "",
    [string[]]$EvidenceDirs = @(),
    [string]$MainWorkspaceEvidenceDir = "",
    [switch]$SkipPublicContract
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

$root = Get-Fb2RefreshRepoRoot
if ([string]::IsNullOrWhiteSpace($OutputDir)) {
    $OutputDir = Join-Path $root "target\fb2-ai-center"
} else {
    $OutputDir = Resolve-Fb2RefreshPath -Path $OutputDir -Root $root
}
New-Item -ItemType Directory -Force -Path $OutputDir | Out-Null

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

[pscustomobject]@{
    schema = "fb2.main_project.status_refresh.v1"
    output_dir = $OutputDir
    evidence_dirs = @($evidence)
    files = [ordered]@{
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
    missing_non_voice_requirements = @($goalAudit.missing_non_voice_requirements)
    deferred_requirements = @($goalAudit.deferred_requirements)
} | ConvertTo-Json -Depth 8
