#requires -Version 7.0

param(
    [string]$OutputDir = "",
    [string[]]$EvidenceDirs = @(),
    [string]$MainWorkspaceEvidenceDir = "",
    [string]$RefreshSummaryPath = "",
    [string]$HandoffPromptPath = "",
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
    $sampleRequestCommand = Get-Fb2RefreshCommandValue -Primary $liveCommands -Fallback $safeCommands -Name "generate_context_pack_sample_request"
    if ([string]::IsNullOrWhiteSpace($sampleRequestCommand)) {
        $sampleRequestCommand = "pwsh -NoProfile -ExecutionPolicy Bypass -File scripts\validate-fb2-context-pack.ps1 -PrintExportRequest -ExternalUserId <fb2_user_uuid_with_orders> -OutputPath target\fb2-ai-center\context-pack-sample-request-current.json"
    }
    $sampleSetCommand = Get-Fb2RefreshCommandValue -Primary $liveCommands -Fallback $safeCommands -Name "validate_context_pack_sample_set"
    if ([string]::IsNullOrWhiteSpace($sampleSetCommand)) {
        $sampleSetCommand = "pwsh -NoProfile -ExecutionPolicy Bypass -File scripts\validate-fb2-context-pack.ps1 -ValidateSampleSet -SamplesDir target\fb2-ai-center\samples -OutputPath target\fb2-ai-center\context-pack-samples-validation-current.json"
    }
    $exportedSampleSetCommand = Get-Fb2RefreshCommandValue -Primary $liveCommands -Fallback $safeCommands -Name "validate_exported_context_pack_sample_set"
    if ([string]::IsNullOrWhiteSpace($exportedSampleSetCommand)) {
        $exportedSampleSetCommand = "pwsh -NoProfile -ExecutionPolicy Bypass -File scripts\validate-fb2-context-pack.ps1 -ValidateSampleSet -SamplesDir <fb2_repo>\target\fb2-ai-center\samples -OutputPath target\fb2-ai-center\fb2-repo-context-pack-samples-validation-current.json"
    }

    [ordered]@{
        refresh_status = "pwsh -NoProfile -ExecutionPolicy Bypass -File scripts\fb2-ai-center-refresh-current-status.ps1"
        read_status_refresh = "Get-Content -Raw -LiteralPath target\fb2-ai-center\status-refresh-current.json | ConvertFrom-Json"
        generate_context_pack_sample_request = $sampleRequestCommand
        validate_context_pack_sample_set = $sampleSetCommand
        validate_exported_context_pack_sample_set = $exportedSampleSetCommand
        validate_current_state = "pwsh -NoProfile -ExecutionPolicy Bypass -File scripts\validate-fb2-ai-center-current-state.ps1"
        validate_read_only_direct_read = "pwsh -NoProfile -ExecutionPolicy Bypass -File scripts\validate-fb2-visible-readonly-summary.ps1 -SummaryPath target\fb2-ai-center\read-only-direct-read-current.json"
        validate_gap_action_board = "pwsh -NoProfile -ExecutionPolicy Bypass -File scripts\validate-fb2-ai-center-gap-action-board.ps1"
        validate_evidence_freshness = "pwsh -NoProfile -ExecutionPolicy Bypass -File scripts\validate-fb2-ai-center-evidence-freshness.ps1"
        validate_completion_matrix = "pwsh -NoProfile -ExecutionPolicy Bypass -File scripts\validate-fb2-ai-center-completion-matrix.ps1"
        validate_handoff_prompt = "pwsh -NoProfile -ExecutionPolicy Bypass -File scripts\validate-fb2-ai-center-handoff-prompt.ps1"
        validate_visible_answer_policy = "pwsh -NoProfile -ExecutionPolicy Bypass -File scripts\validate-fb2-visible-answer-policy.ps1 -SummaryPath <DATA_ONLY_ACCEPTANCE_JSON>"
        no_write_direct_read = Get-Fb2RefreshCommandValue -Primary $liveCommands -Fallback $safeCommands -Name "no_write_direct_read"
        data_only_preflight = Get-Fb2RefreshCommandValue -Primary $liveCommands -Fallback $safeCommands -Name "data_only_preflight"
        visible_regression_requires_authorization = Get-Fb2RefreshCommandValue -Primary $liveCommands -Fallback $safeCommands -Name "visible_regression_requires_authorization"
    }
}

function Get-Fb2RefreshRequirementGroup {
    param([string]$Id)

    switch -Regex ($Id) {
        "^(context_pack_contract|main_project_contract_smoke|domain_context_index_contract)$" { return "main_project_contract" }
        "^(today_matches_analysis|my_ticket_analysis|platform_order_risk|group_opinion_summary|selected_message_review|group_discussion_summary_post|source_reference_audit)$" { return "user_scenarios" }
        "^(permission_safety|feedback_quality_loop)$" { return "permission_and_quality" }
        "^direct_group_chat_read$" { return "group_chat_direct_read" }
        "^voice_final_evidence$" { return "voice_deferred_by_user" }
        default { return "other" }
    }
}

function Get-Fb2RefreshRequirementOwner {
    param([string]$Group)

    switch ($Group) {
        "main_project_contract" { return "main_project" }
        "user_scenarios" { return "shared" }
        "permission_and_quality" { return "shared" }
        "group_chat_direct_read" { return "shared" }
        "voice_deferred_by_user" { return "paused_by_user" }
        default { return "shared" }
    }
}

function New-Fb2RefreshCompletionMatrix {
    param(
        [object]$Status,
        [object]$GoalAudit
    )

    $requirements = @($GoalAudit.requirements)
    $items = @(
        foreach ($requirement in $requirements) {
            $id = [string]$requirement.id
            $group = Get-Fb2RefreshRequirementGroup -Id $id
            [ordered]@{
                id = $id
                group = $group
                owner = Get-Fb2RefreshRequirementOwner -Group $group
                title = [string]$requirement.title
                status = [string]$requirement.status
                complete = [bool]$requirement.complete
                deferred = [bool]$requirement.deferred
                evidence = [string]$requirement.evidence
                missing = [string]$requirement.missing
            }
        }
    )

    $completeCount = @($items | Where-Object { [bool]$_.complete }).Count
    $deferredCount = @($items | Where-Object { [bool]$_.deferred }).Count
    $incompleteCount = @($items | Where-Object { -not [bool]$_.complete -and -not [bool]$_.deferred }).Count
    $tokenPresent = [bool]$Status.environment.fb2_ai_center_token_present

    [ordered]@{
        schema = "fb2.main_project.completion_matrix.v1"
        totals = [ordered]@{
            total = @($items).Count
            complete = $completeCount
            deferred = $deferredCount
            incomplete = $incompleteCount
        }
        gates = [ordered]@{
            data_goal_complete = [bool]$GoalAudit.data_goal_complete
            full_final_complete = [bool]$GoalAudit.full_final_complete
            token_present = $tokenPresent
            voice_deferred_by_user = @($GoalAudit.deferred_requirements) -contains "voice_final_evidence"
            next_minimum_action = [string]$GoalAudit.next_minimum_action
        }
        groups = [ordered]@{
            main_project_contract = @($items | Where-Object { $_.group -eq "main_project_contract" }).Count
            user_scenarios = @($items | Where-Object { $_.group -eq "user_scenarios" }).Count
            permission_and_quality = @($items | Where-Object { $_.group -eq "permission_and_quality" }).Count
            group_chat_direct_read = @($items | Where-Object { $_.group -eq "group_chat_direct_read" }).Count
            voice_deferred_by_user = @($items | Where-Object { $_.group -eq "voice_deferred_by_user" }).Count
            other = @($items | Where-Object { $_.group -eq "other" }).Count
        }
        requirements = $items
    }
}

function Get-Fb2RefreshPathScope {
    param(
        [string]$Path,
        [string]$OutputDir,
        [string[]]$EvidenceDirs
    )

    if ([string]::IsNullOrWhiteSpace($Path)) {
        return "missing"
    }
    try {
        $fullPath = [System.IO.Path]::GetFullPath($Path)
        $fullOutput = [System.IO.Path]::GetFullPath($OutputDir)
    } catch {
        return "unknown"
    }
    if ($fullPath.StartsWith($fullOutput, [System.StringComparison]::OrdinalIgnoreCase)) {
        return "current_output_dir"
    }
    foreach ($dir in @($EvidenceDirs)) {
        if ([string]::IsNullOrWhiteSpace($dir)) {
            continue
        }
        try {
            $fullDir = [System.IO.Path]::GetFullPath($dir)
        } catch {
            continue
        }
        if ($fullPath.StartsWith($fullDir, [System.StringComparison]::OrdinalIgnoreCase)) {
            return "history_evidence_dir"
        }
    }
    return "outside_evidence_dirs"
}

function New-Fb2RefreshArtifactFreshness {
    param(
        [string]$Name,
        [string]$Path,
        [string]$OutputDir,
        [string[]]$EvidenceDirs,
        [datetime]$NowUtc,
        [switch]$GeneratedInCurrentRun
    )

    $exists = -not [string]::IsNullOrWhiteSpace($Path) -and (Test-Path -LiteralPath $Path)
    $lastWriteUtc = $null
    $ageMinutes = $null
    if ($GeneratedInCurrentRun) {
        # 这两个文件在最终 summary 写入后才落盘；这里用本轮生成时间，避免交接误读为旧 artifact。
        $exists = -not [string]::IsNullOrWhiteSpace($Path)
        $lastWriteUtc = $NowUtc.ToString("o")
        $ageMinutes = 0.0
    } elseif ($exists) {
        $item = Get-Item -LiteralPath $Path
        $lastWriteUtc = $item.LastWriteTimeUtc.ToString("o")
        $ageMinutes = [math]::Round(($NowUtc - $item.LastWriteTimeUtc).TotalMinutes, 2)
    }

    [ordered]@{
        name = $Name
        path = $Path
        exists = [bool]$exists
        source_scope = Get-Fb2RefreshPathScope -Path $Path -OutputDir $OutputDir -EvidenceDirs $EvidenceDirs
        last_write_utc = $lastWriteUtc
        age_minutes = $ageMinutes
    }
}

function New-Fb2RefreshEvidenceFreshness {
    param(
        [string]$OutputDir,
        [string[]]$EvidenceDirs,
        [object]$Files,
        [object]$Status,
        [object]$GoalAudit
    )

    $nowUtc = [datetime]::UtcNow
    $generatedInCurrentRun = @{
        status_refresh = $true
        handoff_prompt = $true
    }
    $artifactNames = @(
        "public_contract_status",
        "status",
        "goal_audit",
        "goal_audit_markdown",
        "handoff",
        "handoff_markdown",
        "status_refresh",
        "handoff_prompt",
        "exported_context_pack_sample_set_validation"
    )
    $artifacts = @(
        foreach ($name in $artifactNames) {
            New-Fb2RefreshArtifactFreshness `
                -Name $name `
                -Path ([string](Get-Fb2RefreshProperty $Files $name "")) `
                -OutputDir $OutputDir `
                -EvidenceDirs $EvidenceDirs `
                -NowUtc $nowUtc `
                -GeneratedInCurrentRun:$generatedInCurrentRun.ContainsKey($name)
        }
    )
    $currentOutputCount = @($artifacts | Where-Object { $_.source_scope -eq "current_output_dir" -and $_.exists }).Count
    $historyCount = @($artifacts | Where-Object { $_.source_scope -eq "history_evidence_dir" -and $_.exists }).Count

    [ordered]@{
        schema = "fb2.main_project.evidence_freshness.v1"
        generated_at_utc = $nowUtc.ToString("o")
        note = "artifact freshness only; protected live fb2 data still requires FB2_AI_CENTER_TOKEN"
        current_output_dir = $OutputDir
        evidence_dirs = @($EvidenceDirs)
        artifact_count = @($artifacts).Count
        current_output_artifact_count = $currentOutputCount
        history_artifact_count = $historyCount
        token_present = [bool]$Status.environment.fb2_ai_center_token_present
        data_goal_complete = [bool]$GoalAudit.data_goal_complete
        full_final_complete = [bool]$GoalAudit.full_final_complete
        artifacts = $artifacts
    }
}

function New-Fb2RefreshGapAction {
    param(
        [string]$Id,
        [string]$Status,
        [string]$Owner,
        [string]$EvidenceNeeded,
        [string]$Command,
        [string]$Notes,
        [bool]$CanRunWithoutSecret,
        [bool]$RequiresVisibleGroupWrite,
        [bool]$DeferredByUser
    )

    [ordered]@{
        id = $Id
        status = $Status
        owner = $Owner
        evidence_needed = $EvidenceNeeded
        command = $Command
        notes = $Notes
        can_run_without_secret = $CanRunWithoutSecret
        requires_visible_group_write = $RequiresVisibleGroupWrite
        deferred_by_user = $DeferredByUser
    }
}

function New-Fb2RefreshGapActionBoard {
    param(
        [object]$Status,
        [object]$GoalAudit,
        [object]$BlockingState,
        [object]$NextCommands,
        [object]$CompletionMatrix
    )

    $missing = @($Status.goal_gap_audit.missing) + @($GoalAudit.missing_non_voice_requirements)
    $missing = @($missing | Where-Object { -not [string]::IsNullOrWhiteSpace([string]$_) } | Select-Object -Unique)
    $deferred = @($GoalAudit.deferred_requirements) + @($Status.goal_gap_audit.deferred_by_user)
    $deferred = @($deferred | Where-Object { -not [string]::IsNullOrWhiteSpace([string]$_) } | Select-Object -Unique)

    $actions = [System.Collections.ArrayList]::new()
    foreach ($id in $missing) {
        $text = [string]$id
        if ($text -eq "FB2_AI_CENTER_TOKEN_live_permission_quality_refresh") {
            [void]$actions.Add((New-Fb2RefreshGapAction `
                -Id $text `
                -Status "blocked_by_external_secret" `
                -Owner "fb2_project_and_shared" `
                -EvidenceNeeded "FB2_AI_CENTER_TOKEN or equivalent exported live Context Pack / permission / quality evidence" `
                -Command ([string]$NextCommands.data_only_preflight) `
                -Notes "Run no-write DataOnlyAcceptance preflight after the service token is available; do not treat historical artifacts as a fresh live refresh." `
                -CanRunWithoutSecret $false `
                -RequiresVisibleGroupWrite $false `
                -DeferredByUser $false))
            continue
        }
        if ($text -eq "full_final_acceptance_same_batch_voice_and_visible_chat") {
            [void]$actions.Add((New-Fb2RefreshGapAction `
                -Id $text `
                -Status "waiting_on_voice_and_authorized_visible_regression" `
                -Owner "shared" `
                -EvidenceNeeded "same-batch full final summary with voice_status=required, visible direct-read evidence, feedback coverage and voice final evidence" `
                -Command ([string]$NextCommands.visible_regression_requires_authorization) `
                -Notes "Visible group writes require explicit authorization; this cannot be completed while ASR/TTS final evidence is paused." `
                -CanRunWithoutSecret $false `
                -RequiresVisibleGroupWrite $true `
                -DeferredByUser $false))
            continue
        }
        if ($text -eq "voice_final_evidence" -or $text -eq "ASR_TTS_final_evidence") {
            [void]$actions.Add((New-Fb2RefreshGapAction `
                -Id $text `
                -Status "deferred_by_user" `
                -Owner "paused_by_user" `
                -EvidenceNeeded "real device ASR/TTS final-ready evidence JSON and matching final acceptance run" `
                -Command "" `
                -Notes "ASR/TTS work is intentionally paused by user; keep this visible but do not resume voice work in the current non-voice phase." `
                -CanRunWithoutSecret $false `
                -RequiresVisibleGroupWrite $false `
                -DeferredByUser $true))
            continue
        }

        $requirement = @($CompletionMatrix.requirements | Where-Object { [string]$_.id -eq $text } | Select-Object -First 1)
        $owner = if (@($requirement).Count -gt 0 -and -not [string]::IsNullOrWhiteSpace([string]$requirement[0].owner)) {
            [string]$requirement[0].owner
        } else {
            "shared"
        }
        [void]$actions.Add((New-Fb2RefreshGapAction `
            -Id $text `
            -Status "missing" `
            -Owner $owner `
            -EvidenceNeeded "fresh passing evidence for requirement $text" `
            -Command ([string]$NextCommands.refresh_status) `
            -Notes "Refresh status after the owning side updates its contract, Context Pack output, tool manifest or validation evidence." `
            -CanRunWithoutSecret $true `
            -RequiresVisibleGroupWrite $false `
            -DeferredByUser $false))
    }

    foreach ($id in $deferred) {
        $text = [string]$id
        if (@($actions | Where-Object { [string]$_.id -eq $text }).Count -gt 0) {
            continue
        }
        if ($text -eq "voice_final_evidence" -or $text -eq "ASR_TTS_final_evidence") {
            [void]$actions.Add((New-Fb2RefreshGapAction `
                -Id $text `
                -Status "deferred_by_user" `
                -Owner "paused_by_user" `
                -EvidenceNeeded "real device ASR/TTS final-ready evidence JSON and matching final acceptance run" `
                -Command "" `
                -Notes "ASR/TTS work is intentionally paused by user; do not use data-only acceptance to mark full final complete." `
                -CanRunWithoutSecret $false `
                -RequiresVisibleGroupWrite $false `
                -DeferredByUser $true))
            continue
        }
        [void]$actions.Add((New-Fb2RefreshGapAction `
            -Id $text `
            -Status "deferred" `
            -Owner "shared" `
            -EvidenceNeeded "fresh evidence for deferred requirement $text" `
            -Command "" `
            -Notes "Deferred requirement; keep it visible in handoff until explicitly resumed or completed." `
            -CanRunWithoutSecret $false `
            -RequiresVisibleGroupWrite $false `
            -DeferredByUser $true))
    }

    [ordered]@{
        schema = "fb2.main_project.gap_action_board.v1"
        next_minimum_action = [string]$GoalAudit.next_minimum_action
        blocked_by_external_secret = [bool]$BlockingState.blocked_by_external_secret
        external_secret = [string]$BlockingState.external_secret
        action_count = @($actions).Count
        actions = @($actions)
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
        Assert-Fb2RefreshSelfTest (Test-Path -LiteralPath ([string]$summary.files.handoff_prompt)) "handoff prompt file exists"
        Assert-Fb2RefreshSelfTest (-not [string]::IsNullOrWhiteSpace([string]$summary.owner_next_actions.main_project)) "main owner action"
        Assert-Fb2RefreshSelfTest (-not [string]::IsNullOrWhiteSpace([string]$summary.owner_next_actions.fb2_project)) "fb2 owner action"
        Assert-Fb2RefreshSelfTest ([bool]$summary.blocking_state.blocked_by_external_secret) "selftest token blocked"
        Assert-Fb2RefreshSelfTest ([string]$summary.blocking_state.external_secret -eq "FB2_AI_CENTER_TOKEN") "external secret name"
        Assert-Fb2RefreshSelfTest (-not [string]::IsNullOrWhiteSpace([string]$summary.next_commands.refresh_status)) "refresh command"
        Assert-Fb2RefreshSelfTest (-not [string]::IsNullOrWhiteSpace([string]$summary.next_commands.generate_context_pack_sample_request)) "context pack sample request command"
        Assert-Fb2RefreshSelfTest (-not [string]::IsNullOrWhiteSpace([string]$summary.next_commands.validate_context_pack_sample_set)) "context pack sample set validation command"
        Assert-Fb2RefreshSelfTest (-not [string]::IsNullOrWhiteSpace([string]$summary.next_commands.validate_exported_context_pack_sample_set)) "exported context pack sample set validation command"
        Assert-Fb2RefreshSelfTest (-not [string]::IsNullOrWhiteSpace([string]$summary.next_commands.validate_current_state)) "current state validation command"
        Assert-Fb2RefreshSelfTest (-not [string]::IsNullOrWhiteSpace([string]$summary.next_commands.validate_read_only_direct_read)) "read-only direct read validation command"
        Assert-Fb2RefreshSelfTest (-not [string]::IsNullOrWhiteSpace([string]$summary.next_commands.validate_gap_action_board)) "gap action validation command"
        Assert-Fb2RefreshSelfTest (-not [string]::IsNullOrWhiteSpace([string]$summary.next_commands.validate_evidence_freshness)) "evidence freshness validation command"
        Assert-Fb2RefreshSelfTest (-not [string]::IsNullOrWhiteSpace([string]$summary.next_commands.validate_completion_matrix)) "completion matrix validation command"
        Assert-Fb2RefreshSelfTest (-not [string]::IsNullOrWhiteSpace([string]$summary.next_commands.validate_handoff_prompt)) "handoff prompt validation command"
        Assert-Fb2RefreshSelfTest (-not [string]::IsNullOrWhiteSpace([string]$summary.next_commands.validate_visible_answer_policy)) "visible answer policy validation command"
        Assert-Fb2RefreshSelfTest (-not [string]::IsNullOrWhiteSpace([string]$summary.next_commands.data_only_preflight)) "data-only preflight command"
        Assert-Fb2RefreshSelfTest ([string]$summary.completion_matrix.schema -eq "fb2.main_project.completion_matrix.v1") "completion matrix schema"
        Assert-Fb2RefreshSelfTest (@($summary.completion_matrix.requirements).Count -gt 0) "completion matrix requirements"
        Assert-Fb2RefreshSelfTest ([string]$summary.evidence_freshness.schema -eq "fb2.main_project.evidence_freshness.v1") "evidence freshness schema"
        Assert-Fb2RefreshSelfTest (@($summary.evidence_freshness.artifacts).Count -gt 0) "evidence freshness artifacts"
        $generatedArtifacts = @($summary.evidence_freshness.artifacts | Where-Object { @("status_refresh", "handoff_prompt") -contains [string]$_.name })
        Assert-Fb2RefreshSelfTest (@($generatedArtifacts).Count -eq 2) "generated artifacts present"
        foreach ($artifact in $generatedArtifacts) {
            Assert-Fb2RefreshSelfTest ([bool]$artifact.exists) "generated artifact exists flag $($artifact.name)"
            Assert-Fb2RefreshSelfTest ([string]$artifact.source_scope -eq "current_output_dir") "generated artifact scope $($artifact.name)"
            Assert-Fb2RefreshSelfTest ([double]$artifact.age_minutes -eq 0.0) "generated artifact current age $($artifact.name)"
        }
        Assert-Fb2RefreshSelfTest ([string]$summary.gap_action_board.schema -eq "fb2.main_project.gap_action_board.v1") "gap action board schema"
        Assert-Fb2RefreshSelfTest (@($summary.gap_action_board.actions).Count -gt 0) "gap action board actions"
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
if ([string]::IsNullOrWhiteSpace($HandoffPromptPath)) {
    $HandoffPromptPath = Join-Path $OutputDir "handoff-prompt-current.md"
} else {
    $HandoffPromptPath = Resolve-Fb2RefreshPath -Path $HandoffPromptPath -Root $root
}
$handoffPromptParent = Split-Path -Parent $HandoffPromptPath
if (-not [string]::IsNullOrWhiteSpace($handoffPromptParent)) {
    New-Item -ItemType Directory -Force -Path $handoffPromptParent | Out-Null
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
$completionMatrix = New-Fb2RefreshCompletionMatrix -Status $status -GoalAudit $goalAudit
$gapActionBoard = New-Fb2RefreshGapActionBoard `
    -Status $status `
    -GoalAudit $goalAudit `
    -BlockingState $blockingState `
    -NextCommands $nextCommands `
    -CompletionMatrix $completionMatrix
$files = [ordered]@{
    status_refresh = $RefreshSummaryPath
    public_contract_status = $publicPath
    status = $statusPath
    goal_audit = $goalAuditPath
    goal_audit_markdown = $goalAuditMarkdownPath
    handoff = $handoffPath
    handoff_markdown = $handoffMarkdownPath
    handoff_prompt = $HandoffPromptPath
    exported_context_pack_sample_set_validation = (Join-Path $OutputDir "fb2-repo-context-pack-samples-validation-current.json")
}
$evidenceFreshness = New-Fb2RefreshEvidenceFreshness `
    -OutputDir $OutputDir `
    -EvidenceDirs @($evidence) `
    -Files $files `
    -Status $status `
    -GoalAudit $goalAudit

$refreshSummary = [pscustomobject]@{
    schema = "fb2.main_project.status_refresh.v1"
    output_dir = $OutputDir
    evidence_dirs = @($evidence)
    files = $files
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
    completion_matrix = $completionMatrix
    gap_action_board = $gapActionBoard
    evidence_freshness = $evidenceFreshness
    missing_non_voice_requirements = @($goalAudit.missing_non_voice_requirements)
    deferred_requirements = @($goalAudit.deferred_requirements)
}

$refreshJson = $refreshSummary | ConvertTo-Json -Depth 8
Set-Content -LiteralPath $RefreshSummaryPath -Value $refreshJson -Encoding UTF8
& (Join-Path $PSScriptRoot "fb2-ai-center-handoff-prompt.ps1") `
    -RefreshPath $RefreshSummaryPath `
    -OutputPath $HandoffPromptPath | Out-Null
$refreshJson
