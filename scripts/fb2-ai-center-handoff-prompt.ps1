#requires -Version 7.0

param(
    [string]$RefreshPath = "",
    [string]$OutputPath = "",
    [switch]$SelfTest
)

$ErrorActionPreference = "Stop"

function Get-Fb2PromptRepoRoot {
    Split-Path -Parent $PSScriptRoot
}

function Resolve-Fb2PromptPath {
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

function Get-Fb2PromptProperty {
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

function Get-Fb2PromptQualityHistoryKinds {
    @(
        "feedback",
        "opinion_adoption",
        "opinion_result_review_summary"
    )
}

function Split-Fb2PromptSourceKinds {
    param([string[]]$SourceKinds)

    $qualityKinds = @(Get-Fb2PromptQualityHistoryKinds)
    $uniqueKinds = @($SourceKinds | Where-Object { -not [string]::IsNullOrWhiteSpace([string]$_) } | Select-Object -Unique)
    [ordered]@{
        business = @($uniqueKinds | Where-Object { $qualityKinds -notcontains [string]$_ })
        quality_history = @($uniqueKinds | Where-Object { $qualityKinds -contains [string]$_ })
    }
}

function Read-Fb2PromptJson {
    param([string]$Path)

    if ([string]::IsNullOrWhiteSpace($Path) -or -not (Test-Path -LiteralPath $Path)) {
        throw "Refresh summary not found: $Path. Run scripts\fb2-ai-center-refresh-current-status.ps1 first."
    }
    Get-Content -LiteralPath $Path -Raw | ConvertFrom-Json
}

function ConvertTo-Fb2PromptText {
    param([object]$Value)

    if ($null -eq $Value) {
        return ""
    }
    return [string]$Value
}

function Protect-Fb2PromptSecret {
    param([string]$Text)

    if ([string]::IsNullOrWhiteSpace($Text)) {
        return ""
    }

    $redacted = $Text -replace "(?i)(FB2_AI_CENTER_TOKEN\s*=\s*)['""][^'""]+['""]", '${1}<FB2_AI_CENTER_TOKEN>'
    $redacted = $redacted -replace "(?i)(-Fb2AiCenterToken\s+)(?!<FB2_AI_CENTER_TOKEN>)[^\s]+", '${1}<FB2_AI_CENTER_TOKEN>'
    $redacted = $redacted -replace "(?i)(-Fb2Token\s+)(?!<FB2_AI_CENTER_TOKEN>)[^\s]+", '${1}<FB2_AI_CENTER_TOKEN>'
    $redacted = $redacted -replace "(?i)(-Fb2Password\s+)(?!<FB2_PASSWORD>)[^\s]+", '${1}<FB2_PASSWORD>'
    return $redacted
}

function Format-Fb2PromptCell {
    param(
        [object]$Value,
        [int]$MaxLength = 180
    )

    $text = Protect-Fb2PromptSecret -Text (ConvertTo-Fb2PromptText $Value)
    $text = $text -replace "(`r`n|`n|`r)", " "
    $text = $text.Replace("|", "/")
    if ($text.Length -gt $MaxLength) {
        return ($text.Substring(0, $MaxLength) + "...")
    }
    return $text
}

function Add-Fb2PromptLine {
    param(
        [System.Collections.ArrayList]$Lines,
        [string]$Text = ""
    )

    [void]$Lines.Add($Text)
}

function New-Fb2HandoffPrompt {
    param(
        [object]$Refresh,
        [string]$SourcePath
    )

    $matrix = Get-Fb2PromptProperty $Refresh "completion_matrix"
    $gates = Get-Fb2PromptProperty $matrix "gates"
    $totals = Get-Fb2PromptProperty $matrix "totals"
    $commands = Get-Fb2PromptProperty $Refresh "next_commands"
    $blocking = Get-Fb2PromptProperty $Refresh "blocking_state"
    $ownerActions = Get-Fb2PromptProperty $Refresh "owner_next_actions"
    $freshness = Get-Fb2PromptProperty $Refresh "evidence_freshness"
    $freshnessArtifacts = @(Get-Fb2PromptProperty $freshness "artifacts" @())
    $gapBoard = Get-Fb2PromptProperty $Refresh "gap_action_board"
    $gapActions = @(Get-Fb2PromptProperty $gapBoard "actions" @())
    $requirements = @(Get-Fb2PromptProperty $matrix "requirements" @())
    $plannedCapabilities = @(Get-Fb2PromptProperty $Refresh "planned_capabilities" @())
    if (@($plannedCapabilities).Count -eq 0) {
        $plannedCapabilities = @(Get-Fb2PromptProperty $matrix "planned_capabilities" @())
    }
    if (@($plannedCapabilities).Count -eq 0) {
        $plannedCapabilities = @(Get-Fb2PromptProperty $gapBoard "planned_capabilities" @())
    }
    $exportedSamples = Get-Fb2PromptProperty $Refresh "exported_context_pack_sample_set_validation"
    $exportedSampleScenarios = @(Get-Fb2PromptProperty $exportedSamples "scenarios" @())
    $serverDeploy = Get-Fb2PromptProperty $Refresh "server_deploy_status"
    $server = Get-Fb2PromptProperty $serverDeploy "server"
    $lines = [System.Collections.ArrayList]::new()

    Add-Fb2PromptLine -Lines $lines -Text "# fb2 AI Center 下一轮执行提示"
    Add-Fb2PromptLine -Lines $lines -Text ""
    Add-Fb2PromptLine -Lines $lines -Text ('来源 refresh summary: `{0}`' -f $SourcePath)
    Add-Fb2PromptLine -Lines $lines -Text ('schema: `{0}` / matrix: `{1}`' -f [string]$Refresh.schema, [string]$matrix.schema)
    Add-Fb2PromptLine -Lines $lines -Text ""
    Add-Fb2PromptLine -Lines $lines -Text "## 当前闸门"
    Add-Fb2PromptLine -Lines $lines -Text ('- data_goal_complete: `{0}`' -f [bool]$gates.data_goal_complete)
    Add-Fb2PromptLine -Lines $lines -Text ('- full_final_complete: `{0}`' -f [bool]$gates.full_final_complete)
    Add-Fb2PromptLine -Lines $lines -Text ('- token_present: `{0}`' -f [bool]$gates.token_present)
    Add-Fb2PromptLine -Lines $lines -Text ('- protected_live_preflight_satisfied: `{0}`' -f [bool]$gates.protected_live_preflight_satisfied)
    Add-Fb2PromptLine -Lines $lines -Text ('- answer_source_validation_ready: `{0}`' -f [bool](Get-Fb2PromptProperty $Refresh 'answer_source_validation_ready' $false))
    Add-Fb2PromptLine -Lines $lines -Text ('- voice_deferred_by_user: `{0}`' -f [bool]$gates.voice_deferred_by_user)
    Add-Fb2PromptLine -Lines $lines -Text ('- next_minimum_action: `{0}`' -f [string]$gates.next_minimum_action)
    Add-Fb2PromptLine -Lines $lines -Text ('- totals: complete `{0}` / deferred `{1}` / incomplete `{2}` / total `{3}`' -f [int]$totals.complete, [int]$totals.deferred, [int]$totals.incomplete, [int]$totals.total)
    Add-Fb2PromptLine -Lines $lines -Text ""
    Add-Fb2PromptLine -Lines $lines -Text "## 线上主项目"
    Add-Fb2PromptLine -Lines $lines -Text ('- main_base: `{0}`' -f [string](Get-Fb2PromptProperty $serverDeploy 'main_base' ''))
    Add-Fb2PromptLine -Lines $lines -Text ('- health: `{0}`' -f [string](Get-Fb2PromptProperty $server 'health' ''))
    Add-Fb2PromptLine -Lines $lines -Text ('- versionName: `{0}`' -f [string](Get-Fb2PromptProperty $server 'versionName' ''))
    Add-Fb2PromptLine -Lines $lines -Text ('- deployed_git_sha: `{0}`' -f [string](Get-Fb2PromptProperty $server 'gitSha' ''))
    Add-Fb2PromptLine -Lines $lines -Text ('- latest_runtime_sha: `{0}`' -f [string](Get-Fb2PromptProperty $serverDeploy 'latest_runtime_sha' ''))
    Add-Fb2PromptLine -Lines $lines -Text ('- deployed_contains_latest_runtime_sha: `{0}`' -f [bool](Get-Fb2PromptProperty $serverDeploy 'deployed_contains_latest_runtime_sha' $false))
    Add-Fb2PromptLine -Lines $lines -Text ('- server_deploy_ready: `{0}`' -f [bool](Get-Fb2PromptProperty $Refresh 'server_deploy_ready' $false))
    Add-Fb2PromptLine -Lines $lines -Text ('- note: `{0}`' -f (Format-Fb2PromptCell (Get-Fb2PromptProperty $serverDeploy 'note' '') 220))
    Add-Fb2PromptLine -Lines $lines -Text ""
    Add-Fb2PromptLine -Lines $lines -Text "## Owner 下一步"
    Add-Fb2PromptLine -Lines $lines -Text ('- main_project: `{0}`' -f [string]$ownerActions.main_project)
    Add-Fb2PromptLine -Lines $lines -Text ('- fb2_project: `{0}`' -f [string]$ownerActions.fb2_project)
    Add-Fb2PromptLine -Lines $lines -Text ('- shared: `{0}`' -f [string]$ownerActions.shared)
    Add-Fb2PromptLine -Lines $lines -Text ""
    Add-Fb2PromptLine -Lines $lines -Text "## 计划能力 / 非生产边界"
    Add-Fb2PromptLine -Lines $lines -Text "| id | status | contract | source_enumerator | chunk_manifest | dry_run_status | production_grounding | blocks_data_goal | answer_time_vector_candidates_enabled | next |"
    Add-Fb2PromptLine -Lines $lines -Text "|---|---|---|---|---|---|---|---|---|---|"
    foreach ($capability in $plannedCapabilities) {
        $capId = Format-Fb2PromptCell (Get-Fb2PromptProperty $capability 'id' '') 80
        $capStatus = Format-Fb2PromptCell (Get-Fb2PromptProperty $capability 'status' '') 120
        $capContractVersion = [string](Get-Fb2PromptProperty $capability 'contract_version' '')
        $capReportVersion = [string](Get-Fb2PromptProperty $capability 'report_version' '')
        $capEmbeddingDryRunReportVersion = [string](Get-Fb2PromptProperty $capability 'embedding_build_dry_run_report_version' '')
        $capContract = Format-Fb2PromptCell (($capContractVersion, $capReportVersion, $capEmbeddingDryRunReportVersion | Where-Object { -not [string]::IsNullOrWhiteSpace([string]$_) }) -join ' / ') 180
        $capSourceEnumeratorReportVersion = [string](Get-Fb2PromptProperty $capability 'source_enumerator_report_version' '')
        $capSourceEnumeratorStatus = [string](Get-Fb2PromptProperty $capability 'source_enumerator_status' '')
        $capSourceEnumerator = Format-Fb2PromptCell (($capSourceEnumeratorReportVersion, $capSourceEnumeratorStatus | Where-Object { -not [string]::IsNullOrWhiteSpace([string]$_) }) -join ' / ') 140
        $capChunkManifestReportVersion = [string](Get-Fb2PromptProperty $capability 'chunk_manifest_report_version' '')
        $capChunkManifestStatus = [string](Get-Fb2PromptProperty $capability 'chunk_manifest_status' '')
        $capChunkManifest = Format-Fb2PromptCell (($capChunkManifestReportVersion, $capChunkManifestStatus | Where-Object { -not [string]::IsNullOrWhiteSpace([string]$_) }) -join ' / ') 140
        $capDryRunStatusValue = [string](Get-Fb2PromptProperty $capability 'dry_run_status' '')
        $capDryRunStatusCell = Format-Fb2PromptCell $capDryRunStatusValue 80
        $capProduction = Format-Fb2PromptCell (Get-Fb2PromptProperty $capability 'production_grounding' '') 40
        $capBlocks = Format-Fb2PromptCell (Get-Fb2PromptProperty $capability 'blocks_data_goal' '') 40
        $capAnswerTime = Format-Fb2PromptCell (Get-Fb2PromptProperty $capability 'answer_time_vector_candidates_enabled' '') 40
        $capNextRaw = [string](Get-Fb2PromptProperty $capability 'next_action' '')
        if (-not [string]::IsNullOrWhiteSpace($capDryRunStatusValue)) {
            $capNextRaw = "$capNextRaw writes_vector_store=$([bool](Get-Fb2PromptProperty $capability 'writes_vector_store' $false)); writes_public_group_messages=$([bool](Get-Fb2PromptProperty $capability 'writes_public_group_messages' $false)); writes_chunk_manifest_file=$([bool](Get-Fb2PromptProperty $capability 'writes_chunk_manifest_file' $false)); persists_manifest_rows=$([bool](Get-Fb2PromptProperty $capability 'persists_manifest_rows' $false)); ready_to_write_embeddings=$([bool](Get-Fb2PromptProperty $capability 'ready_to_write_embeddings' $false)); candidate_rows_require_live_hydration=$([bool](Get-Fb2PromptProperty $capability 'candidate_rows_require_live_hydration' $false));".Trim()
        }
        $capNext = Format-Fb2PromptCell $capNextRaw 260
        Add-Fb2PromptLine -Lines $lines -Text "| $capId | $capStatus | $capContract | $capSourceEnumerator | $capChunkManifest | $capDryRunStatusCell | $capProduction | $capBlocks | $capAnswerTime | $capNext |"
    }
    $p4SourceSafety = @($plannedCapabilities | Where-Object {
            [string](Get-Fb2PromptProperty $_ 'source_enumerator_report_version' '') -eq 'fb2_p4_source_enumerator_v1'
        } | Select-Object -First 1)
    if (@($p4SourceSafety).Count -gt 0) {
        Add-Fb2PromptLine -Lines $lines -Text ("- p4_source_enumerator_safety: writes_public_group_messages={0}; ready_to_write_embeddings={1}; writes_feedback_or_adoption={2}; writes_opinion_index_rows={3};" -f `
                [bool](Get-Fb2PromptProperty $p4SourceSafety[0] 'writes_public_group_messages' $false), `
                [bool](Get-Fb2PromptProperty $p4SourceSafety[0] 'ready_to_write_embeddings' $false), `
                [bool](Get-Fb2PromptProperty $p4SourceSafety[0] 'writes_feedback_or_adoption' $false), `
                [bool](Get-Fb2PromptProperty $p4SourceSafety[0] 'writes_opinion_index_rows' $false))
        Add-Fb2PromptLine -Lines $lines -Text ("- p4_chunk_manifest_safety: writes_chunk_manifest_file={0}; persists_manifest_rows={1}; source_payload_included={2}; embedding_text_included={3}; ready_for_shadow_eval={4};" -f `
                [bool](Get-Fb2PromptProperty $p4SourceSafety[0] 'writes_chunk_manifest_file' $false), `
                [bool](Get-Fb2PromptProperty $p4SourceSafety[0] 'persists_manifest_rows' $false), `
                [bool](Get-Fb2PromptProperty $p4SourceSafety[0] 'source_payload_included' $false), `
                [bool](Get-Fb2PromptProperty $p4SourceSafety[0] 'embedding_text_included' $false), `
                [bool](Get-Fb2PromptProperty $p4SourceSafety[0] 'ready_for_shadow_eval' $false))
    }
    Add-Fb2PromptLine -Lines $lines -Text ""
    Add-Fb2PromptLine -Lines $lines -Text "## fb2 导出样本"
    Add-Fb2PromptLine -Lines $lines -Text ('- attempted: `{0}` / complete: `{1}` / passed: `{2}` / failed: `{3}`' -f [bool](Get-Fb2PromptProperty $exportedSamples 'attempted' $false), [bool](Get-Fb2PromptProperty $exportedSamples 'complete' $false), [int](Get-Fb2PromptProperty $exportedSamples 'passed_count' 0), [int](Get-Fb2PromptProperty $exportedSamples 'failed_count' 0))
    if (@($exportedSampleScenarios).Count -gt 0) {
        $pipe = [char]124
        Add-Fb2PromptLine -Lines $lines -Text ('{0} scenario {0} audit {0} sources {0} business {0} quality_history {0} sha256 {0}' -f $pipe)
        Add-Fb2PromptLine -Lines $lines -Text ('{0}---{0}---{0}---:{0}---{0}---{0}---{0}' -f $pipe)
        foreach ($scenario in $exportedSampleScenarios) {
            $scenarioId = Format-Fb2PromptCell $scenario.scenario 80
            $auditId = Format-Fb2PromptCell $scenario.context_audit_id 80
            $sourceCount = Format-Fb2PromptCell $scenario.citation_source_count 30
            $sourceKinds = @((Get-Fb2PromptProperty $scenario 'source_kinds' @()) | ForEach-Object { [string]$_ })
            $splitKinds = Split-Fb2PromptSourceKinds -SourceKinds $sourceKinds
            $businessKinds = @((Get-Fb2PromptProperty $scenario 'business_source_kinds' @()) | ForEach-Object { [string]$_ } | Where-Object { -not [string]::IsNullOrWhiteSpace([string]$_) } | Select-Object -Unique)
            if ($businessKinds.Count -eq 0) {
                $businessKinds = @($splitKinds["business"])
            }
            $qualityKinds = @((Get-Fb2PromptProperty $scenario 'quality_history_source_kinds' @()) | ForEach-Object { [string]$_ } | Where-Object { -not [string]::IsNullOrWhiteSpace([string]$_) } | Select-Object -Unique)
            if ($qualityKinds.Count -eq 0) {
                $qualityKinds = @($splitKinds["quality_history"])
            }
            $business = Format-Fb2PromptCell (@($businessKinds) -join ', ') 180
            $quality = Format-Fb2PromptCell (@($qualityKinds) -join ', ') 180
            $sha = Format-Fb2PromptCell $scenario.context_pack_sha256 80
            Add-Fb2PromptLine -Lines $lines -Text ('{0} {1} {0} {2} {0} {3} {0} {4} {0} {5} {0} {6} {0}' -f $pipe, $scenarioId, $auditId, $sourceCount, $business, $quality, $sha)
        }
    }
    Add-Fb2PromptLine -Lines $lines -Text ""
    Add-Fb2PromptLine -Lines $lines -Text "## 可执行命令"
    foreach ($name in @("refresh_status", "read_status_refresh", "generate_context_pack_sample_request", "validate_context_pack_sample_set", "validate_exported_context_pack_sample_set", "validate_context_projection_log", "validate_user_scenario_audit", "validate_current_state", "validate_public_contract_status", "validate_server_deploy_status", "validate_project_direct_network_policy", "validate_context_format_route", "validate_read_only_direct_read", "validate_gap_action_board", "validate_evidence_freshness", "validate_evidence_privacy", "validate_completion_matrix", "validate_handoff_prompt", "validate_visible_answer_policy", "validate_live_preflight_request", "validate_tokenless_continuation", "no_write_direct_read", "data_only_preflight", "data_only_preflight_via_fb2_server_token_bridge", "visible_regression_requires_authorization")) {
        $value = Protect-Fb2PromptSecret -Text ([string](Get-Fb2PromptProperty $commands $name ""))
        if (-not [string]::IsNullOrWhiteSpace($value)) {
            Add-Fb2PromptLine -Lines $lines -Text ('- `{0}`: `{1}`' -f $name, $value)
        }
    }
    Add-Fb2PromptLine -Lines $lines -Text ""
    Add-Fb2PromptLine -Lines $lines -Text "## 阻塞与边界"
    Add-Fb2PromptLine -Lines $lines -Text ('- external_secret: `{0}`' -f [string]$blocking.external_secret)
    Add-Fb2PromptLine -Lines $lines -Text ('- blocked_by_external_secret: `{0}`' -f [bool]$blocking.blocked_by_external_secret)
    Add-Fb2PromptLine -Lines $lines -Text ('- safe_to_continue_without_secret: `{0}`' -f (@($blocking.safe_to_continue_without_secret) -join ', '))
    Add-Fb2PromptLine -Lines $lines -Text ('- requires_secret: `{0}`' -f (@($blocking.requires_secret) -join ', '))
    Add-Fb2PromptLine -Lines $lines -Text ""
    Add-Fb2PromptLine -Lines $lines -Text "## 缺口行动板"
    Add-Fb2PromptLine -Lines $lines -Text ('- gap_schema: `{0}`' -f [string](Get-Fb2PromptProperty $gapBoard 'schema' ''))
    Add-Fb2PromptLine -Lines $lines -Text ('- action_count: `{0}`' -f [int](Get-Fb2PromptProperty $gapBoard 'action_count' 0))
    foreach ($action in $gapActions) {
        $actionId = Format-Fb2PromptCell $action.id 120
        $actionStatus = Format-Fb2PromptCell $action.status 100
        $actionOwner = Format-Fb2PromptCell $action.owner 80
        $actionEvidence = Format-Fb2PromptCell $action.evidence_needed 220
        $actionCommand = Format-Fb2PromptCell $action.command 220
        $actionNotes = Format-Fb2PromptCell $action.notes 220
        Add-Fb2PromptLine -Lines $lines -Text ("- gap {0}: status={1}; owner={2}; evidence={3}; command={4}; notes={5}" -f $actionId, $actionStatus, $actionOwner, $actionEvidence, $actionCommand, $actionNotes)
    }
    Add-Fb2PromptLine -Lines $lines -Text ""
    Add-Fb2PromptLine -Lines $lines -Text "## 证据新鲜度"
    Add-Fb2PromptLine -Lines $lines -Text ('- freshness_schema: `{0}`' -f [string](Get-Fb2PromptProperty $freshness 'schema' ''))
    Add-Fb2PromptLine -Lines $lines -Text ('- generated_at_utc: `{0}`' -f [string](Get-Fb2PromptProperty $freshness 'generated_at_utc' ''))
    Add-Fb2PromptLine -Lines $lines -Text ('- note: `{0}`' -f [string](Get-Fb2PromptProperty $freshness 'note' ''))
    Add-Fb2PromptLine -Lines $lines -Text ('- current_output_artifact_count: `{0}`' -f [int](Get-Fb2PromptProperty $freshness 'current_output_artifact_count' 0))
    Add-Fb2PromptLine -Lines $lines -Text ('- history_artifact_count: `{0}`' -f [int](Get-Fb2PromptProperty $freshness 'history_artifact_count' 0))
    $pipe = [char]124
    Add-Fb2PromptLine -Lines $lines -Text ('{0} artifact {0} source {0} age_minutes {0} path {0}' -f $pipe)
    Add-Fb2PromptLine -Lines $lines -Text ('{0}---{0}---{0}---:{0}---{0}' -f $pipe)
    foreach ($artifact in $freshnessArtifacts) {
        $name = Format-Fb2PromptCell $artifact.name 80
        $source = Format-Fb2PromptCell $artifact.source_scope 80
        $age = Format-Fb2PromptCell $artifact.age_minutes 40
        $path = Format-Fb2PromptCell $artifact.path 180
        Add-Fb2PromptLine -Lines $lines -Text ('{0} {1} {0} {2} {0} {3} {0} {4} {0}' -f $pipe, $name, $source, $age, $path)
    }
    Add-Fb2PromptLine -Lines $lines -Text ""
    Add-Fb2PromptLine -Lines $lines -Text "## 完成矩阵"
    Add-Fb2PromptLine -Lines $lines -Text ('{0} group {0} owner {0} id {0} status {0} evidence {0} missing {0}' -f $pipe)
    Add-Fb2PromptLine -Lines $lines -Text ('{0}---{0}---{0}---{0}---{0}---{0}---{0}' -f $pipe)
    foreach ($requirement in $requirements) {
        $group = Format-Fb2PromptCell $requirement.group 80
        $owner = Format-Fb2PromptCell $requirement.owner 80
        $id = Format-Fb2PromptCell $requirement.id 80
        $status = Format-Fb2PromptCell $requirement.status 80
        $evidence = Format-Fb2PromptCell $requirement.evidence 220
        $missing = Format-Fb2PromptCell $requirement.missing 160
        Add-Fb2PromptLine -Lines $lines -Text ('{0} {1} {0} {2} {0} {3} {0} {4} {0} {5} {0} {6} {0}' -f $pipe, $group, $owner, $id, $status, $evidence, $missing)
    }
    Add-Fb2PromptLine -Lines $lines -Text ""
    Add-Fb2PromptLine -Lines $lines -Text "## 接手规则"
    Add-Fb2PromptLine -Lines $lines -Text '- 先运行 `refresh_status`，再读取 `status-refresh-current.json`。'
    Add-Fb2PromptLine -Lines $lines -Text '- 没有 `FB2_AI_CENTER_TOKEN` 时，只做公开契约、离线样本、无写群直读和文档/脚本回归。'
    Add-Fb2PromptLine -Lines $lines -Text '- 有 token 或可用 fb2 服务器 SSH 权限后，先跑 `data_only_preflight` 或 `data_only_preflight_via_fb2_server_token_bridge`，刷新 live Context Pack、本人订单、平台摘要、权限和质量证据。'
    Add-Fb2PromptLine -Lines $lines -Text '- 真实群聊可见写入必须另有明确授权；截图不能替代 API 直读 summary。'
    Add-Fb2PromptLine -Lines $lines -Text '- ASR/TTS final evidence 仍按用户要求暂停，不能把 `full_final_complete=false` 改成完成。'

    return (($lines -join [Environment]::NewLine) + [Environment]::NewLine)
}

function Assert-Fb2PromptSelfTest {
    param(
        [bool]$Condition,
        [string]$Message
    )

    if (-not $Condition) {
        throw "SelfTest failed: $Message"
    }
}

function Invoke-Fb2PromptSelfTest {
    $tempRoot = Join-Path ([System.IO.Path]::GetTempPath()) ("fb2-handoff-prompt-selftest-" + [guid]::NewGuid().ToString("N"))
    $refreshPath = Join-Path $tempRoot "status-refresh-current.json"
    $promptPath = Join-Path $tempRoot "handoff-prompt-current.md"
    try {
        New-Item -ItemType Directory -Force -Path $tempRoot | Out-Null
        $fixture = [pscustomobject]@{
            schema = "fb2.main_project.status_refresh.v1"
            server_deploy_ready = $true
            server_deploy_status = [ordered]@{
                schema = "fb2.main_project.server_deploy_status.v1"
                main_base = "http://43.139.149.158:8080"
                server = [ordered]@{
                    health = "OK"
                    versionName = "0.3.755"
                    gitSha = "1c14bde6cd12e7af87ec7feb2cb7dc412138c2c5"
                }
                latest_runtime_sha = "12368e2ba39b6ed8071a5e43b4c4e56091a0c18c"
                deployed_contains_latest_runtime_sha = $true
                note = "This verifies the deployed main-project server contains the latest runtime commit."
            }
            owner_next_actions = [ordered]@{
                main_project = "keep_contract_and_status_regressions_green_until_FB2_AI_CENTER_TOKEN_is_available"
                fb2_project = "provide_FB2_AI_CENTER_TOKEN_or_export_equivalent_live_Context_Pack_permission_quality_evidence"
                shared = "run_DataOnlyAcceptance_PreflightOnly_with_token_then_refresh_status_refresh_current_json"
            }
            blocking_state = [ordered]@{
                blocked_by_external_secret = $true
                external_secret = "FB2_AI_CENTER_TOKEN"
                safe_to_continue_without_secret = @("status_refresh_selftest")
                requires_secret = @("live_context_pack_permission_quality_refresh")
            }
            planned_capabilities = @(
                [ordered]@{
                    id = "p4_vector"
                    report_version = "fb2_p4_vector_readiness_plan_v1"
                    contract_version = "fb2_p4_vector_contract_v1"
                    source_enumerator_report_version = "fb2_p4_source_enumerator_v1"
                    source_enumerator_status = "source_specific_no_write_sample_available"
                    chunk_manifest_report_version = "fb2_p4_chunk_manifest_v1"
                    chunk_manifest_status = "id_only_no_write_manifest_available"
                    embedding_build_dry_run_report_version = "fb2_p4_embedding_build_dry_run_v1"
                    status = "contract_design_committed_embedding_not_started"
                    dry_run_status = "dry_run_available_no_writes"
                    writes_chunk_manifest_file = $false
                    persists_manifest_rows = $false
                    source_payload_included = $false
                    embedding_text_included = $false
                    writes_vector_store = $false
                    writes_public_group_messages = $false
                    ready_to_write_embeddings = $false
                    ready_for_shadow_eval = $false
                    candidate_rows_require_live_hydration = $true
                    blocks_data_goal = $false
                    production_grounding = $false
                    answer_time_vector_candidates_enabled = $false
                    next_action = "Keep Context Pack and structured tools as production grounding."
                }
            )
            exported_context_pack_sample_set_validation = [ordered]@{
                attempted = $true
                complete = $true
                passed_count = 4
                failed_count = 0
                scenarios = @(
                    [ordered]@{ scenario = "today_matches_context_pack"; context_audit_id = "audit-today"; citation_source_count = 23; source_kinds = @("match", "odds", "context_audit", "opinion_result_review_summary"); context_pack_sha256 = ("a" * 64) },
                    [ordered]@{ scenario = "my_ticket_context_pack"; context_audit_id = "audit-ticket"; citation_source_count = 43; source_kinds = @("user_order", "ticket", "context_audit"); context_pack_sha256 = ("b" * 64) },
                    [ordered]@{ scenario = "platform_order_context_pack"; context_audit_id = "audit-platform"; citation_source_count = 24; source_kinds = @("platform_order_summary", "context_audit"); context_pack_sha256 = ("c" * 64) },
                    [ordered]@{ scenario = "group_opinion_context_pack"; context_audit_id = "audit-opinion"; citation_source_count = 24; source_kinds = @("group_message", "opinion_memory", "context_audit"); context_pack_sha256 = ("d" * 64) }
                )
            }
            next_commands = [ordered]@{
                refresh_status = "pwsh -NoProfile -ExecutionPolicy Bypass -File scripts\fb2-ai-center-refresh-current-status.ps1"
                read_status_refresh = "Get-Content -Raw -LiteralPath target\fb2-ai-center\status-refresh-current.json | ConvertFrom-Json"
                generate_context_pack_sample_request = "pwsh -NoProfile -ExecutionPolicy Bypass -File scripts\validate-fb2-context-pack.ps1 -PrintExportRequest -ExternalUserId 6fe5aa17-0403-427a-8e91-7f414beca35d -OutputPath target\fb2-ai-center\context-pack-sample-request-current.json"
                validate_context_pack_sample_set = "pwsh -NoProfile -ExecutionPolicy Bypass -File scripts\validate-fb2-context-pack.ps1 -ValidateSampleSet -SamplesDir target\fb2-ai-center\samples -OutputPath target\fb2-ai-center\context-pack-samples-validation-current.json"
                validate_exported_context_pack_sample_set = "pwsh -NoProfile -ExecutionPolicy Bypass -File scripts\validate-fb2-context-pack.ps1 -ValidateSampleSet -SamplesDir <fb2_repo>\target\fb2-ai-center\samples -OutputPath target\fb2-ai-center\fb2-repo-context-pack-samples-validation-current.json"
                validate_context_projection_log = "pwsh -NoProfile -ExecutionPolicy Bypass -File scripts\validate-fb2-context-projection-log.ps1 -StatusPath target\fb2-ai-center\status-current.json -OutputPath target\fb2-ai-center\context-projection-log-validation-current.json"
                validate_user_scenario_audit = "pwsh -NoProfile -ExecutionPolicy Bypass -File scripts\validate-fb2-user-scenario-audit.ps1 -StatusPath target\fb2-ai-center\status-current.json -OutputPath target\fb2-ai-center\user-scenario-audit-validation-current.json"
                validate_current_state = "pwsh -NoProfile -ExecutionPolicy Bypass -File scripts\validate-fb2-ai-center-current-state.ps1"
                validate_public_contract_status = "pwsh -NoProfile -ExecutionPolicy Bypass -File scripts\fb2-public-contract-status.ps1 -OutputPath target\fb2-ai-center\public-contract-status-current.json"
                validate_server_deploy_status = "pwsh -NoProfile -ExecutionPolicy Bypass -File scripts\validate-fb2-main-server-deploy-status.ps1"
                validate_project_direct_network_policy = "pwsh -NoProfile -ExecutionPolicy Bypass -File scripts\validate-project-direct-network-policy.ps1 -OutputPath target\fb2-ai-center\project-direct-network-policy-validation-current.json"
                validate_context_format_route = "pwsh -NoProfile -ExecutionPolicy Bypass -File scripts\validate-fb2-context-format-route.ps1 -OutputPath target\fb2-ai-center\context-format-route-validation-current.json"
                validate_read_only_direct_read = "pwsh -NoProfile -ExecutionPolicy Bypass -File scripts\validate-fb2-visible-readonly-summary.ps1 -SummaryPath target\fb2-ai-center\read-only-direct-read-current.json"
                validate_gap_action_board = "pwsh -NoProfile -ExecutionPolicy Bypass -File scripts\validate-fb2-ai-center-gap-action-board.ps1"
                validate_evidence_freshness = "pwsh -NoProfile -ExecutionPolicy Bypass -File scripts\validate-fb2-ai-center-evidence-freshness.ps1"
                validate_evidence_privacy = "pwsh -NoProfile -ExecutionPolicy Bypass -File scripts\validate-fb2-evidence-privacy.ps1"
                validate_completion_matrix = "pwsh -NoProfile -ExecutionPolicy Bypass -File scripts\validate-fb2-ai-center-completion-matrix.ps1"
                validate_handoff_prompt = "pwsh -NoProfile -ExecutionPolicy Bypass -File scripts\validate-fb2-ai-center-handoff-prompt.ps1"
                validate_visible_answer_policy = "pwsh -NoProfile -ExecutionPolicy Bypass -File scripts\validate-fb2-visible-answer-policy.ps1 -SummaryPath <DATA_ONLY_ACCEPTANCE_JSON>"
                validate_live_preflight_request = "pwsh -NoProfile -ExecutionPolicy Bypass -File scripts\validate-fb2-live-preflight-request.ps1 -StatusPath target\fb2-ai-center\status-current.json"
                validate_tokenless_continuation = "pwsh -NoProfile -ExecutionPolicy Bypass -File scripts\validate-fb2-tokenless-continuation.ps1 -OutputPath target\fb2-ai-center\tokenless-continuation-validation-current.json"
                no_write_direct_read = "pwsh -NoProfile -ExecutionPolicy Bypass -File scripts\smoke-fb2-visible-chat.ps1 -ReadOnlyDirectRead -Fb2Password secret-real-password"
                data_only_preflight = '$env:FB2_AI_CENTER_TOKEN="secret-real-value"; pwsh -NoProfile -ExecutionPolicy Bypass -File scripts\smoke-fb2-final-acceptance.ps1 -DataOnlyAcceptance -PreflightOnly -Fb2Token secret-real-value'
                data_only_preflight_via_fb2_server_token_bridge = "pwsh -NoProfile -ExecutionPolicy Bypass -File scripts\run-fb2-ai-center-token-bridge.ps1 -RunDataOnlyPreflight"
                visible_regression_requires_authorization = "pwsh -NoProfile -ExecutionPolicy Bypass -File scripts\smoke-fb2-visible-chat.ps1 -AllowVisibleMessages"
            }
            completion_matrix = [ordered]@{
                schema = "fb2.main_project.completion_matrix.v1"
                totals = [ordered]@{ total = 2; complete = 1; deferred = 1; incomplete = 0 }
                gates = [ordered]@{
                    data_goal_complete = $true
                    full_final_complete = $false
                    token_present = $false
                    protected_live_preflight_satisfied = $true
                    voice_deferred_by_user = $true
                    next_minimum_action = "set_FB2_AI_CENTER_TOKEN_then_run_DataOnlyAcceptance_PreflightOnly"
                }
                requirements = @(
                    [ordered]@{ id = "today_matches_analysis"; group = "user_scenarios"; owner = "shared"; title = "today"; status = "complete"; complete = $true; deferred = $false; evidence = "sample"; missing = "" },
                    [ordered]@{ id = "voice_final_evidence"; group = "voice_deferred_by_user"; owner = "paused_by_user"; title = "voice"; status = "deferred"; complete = $false; deferred = $true; evidence = ""; missing = "ASR/TTS is intentionally deferred by user" }
                )
                planned_capabilities = @(
                    [ordered]@{
                        id = "p4_vector"
                        report_version = "fb2_p4_vector_readiness_plan_v1"
                        contract_version = "fb2_p4_vector_contract_v1"
                        source_enumerator_report_version = "fb2_p4_source_enumerator_v1"
                        source_enumerator_status = "source_specific_no_write_sample_available"
                        chunk_manifest_report_version = "fb2_p4_chunk_manifest_v1"
                        chunk_manifest_status = "id_only_no_write_manifest_available"
                        embedding_build_dry_run_report_version = "fb2_p4_embedding_build_dry_run_v1"
                        status = "contract_design_committed_embedding_not_started"
                        dry_run_status = "dry_run_available_no_writes"
                        writes_chunk_manifest_file = $false
                        persists_manifest_rows = $false
                        source_payload_included = $false
                        embedding_text_included = $false
                        writes_vector_store = $false
                        writes_public_group_messages = $false
                        ready_to_write_embeddings = $false
                        ready_for_shadow_eval = $false
                        candidate_rows_require_live_hydration = $true
                        blocks_data_goal = $false
                        production_grounding = $false
                        answer_time_vector_candidates_enabled = $false
                        next_action = "Keep Context Pack and structured tools as production grounding."
                    }
                )
            }
            evidence_freshness = [ordered]@{
                schema = "fb2.main_project.evidence_freshness.v1"
                generated_at_utc = "2026-06-23T00:00:00.0000000Z"
                note = "artifact freshness only; protected live fb2 data still requires FB2_AI_CENTER_TOKEN"
                current_output_artifact_count = 2
                history_artifact_count = 0
                artifacts = @(
                    [ordered]@{ name = "status"; source_scope = "current_output_dir"; age_minutes = 0; path = "target\fb2-ai-center\status-current.json" },
                    [ordered]@{ name = "goal_audit"; source_scope = "current_output_dir"; age_minutes = 0; path = "target\fb2-ai-center\goal-audit-current.json" }
                )
            }
            gap_action_board = [ordered]@{
                schema = "fb2.main_project.gap_action_board.v1"
                action_count = 2
                planned_capabilities = @(
                    [ordered]@{
                        id = "p4_vector"
                        report_version = "fb2_p4_vector_readiness_plan_v1"
                        contract_version = "fb2_p4_vector_contract_v1"
                        source_enumerator_report_version = "fb2_p4_source_enumerator_v1"
                        source_enumerator_status = "source_specific_no_write_sample_available"
                        chunk_manifest_report_version = "fb2_p4_chunk_manifest_v1"
                        chunk_manifest_status = "id_only_no_write_manifest_available"
                        embedding_build_dry_run_report_version = "fb2_p4_embedding_build_dry_run_v1"
                        status = "contract_design_committed_embedding_not_started"
                        dry_run_status = "dry_run_available_no_writes"
                        writes_chunk_manifest_file = $false
                        persists_manifest_rows = $false
                        source_payload_included = $false
                        embedding_text_included = $false
                        writes_vector_store = $false
                        writes_public_group_messages = $false
                        ready_to_write_embeddings = $false
                        ready_for_shadow_eval = $false
                        candidate_rows_require_live_hydration = $true
                        blocks_data_goal = $false
                        production_grounding = $false
                        answer_time_vector_candidates_enabled = $false
                        next_action = "Keep Context Pack and structured tools as production grounding."
                    }
                )
                actions = @(
                    [ordered]@{
                        id = "FB2_AI_CENTER_TOKEN_live_permission_quality_refresh"
                        status = "blocked_by_external_secret"
                        owner = "fb2_project_and_shared"
                        evidence_needed = "FB2_AI_CENTER_TOKEN or equivalent exported live Context Pack / permission / quality evidence"
                        command = '$env:FB2_AI_CENTER_TOKEN="secret-real-value"; pwsh -NoProfile -ExecutionPolicy Bypass -File scripts\smoke-fb2-final-acceptance.ps1 -DataOnlyAcceptance -PreflightOnly -Fb2Token secret-real-value'
                        notes = "Run no-write DataOnlyAcceptance preflight after token is available."
                    },
                    [ordered]@{
                        id = "voice_final_evidence"
                        status = "deferred_by_user"
                        owner = "paused_by_user"
                        evidence_needed = "real device ASR/TTS evidence"
                        command = ""
                        notes = "ASR/TTS is paused."
                    }
                )
            }
        }
        $fixture | Add-Member -NotePropertyName "answer_source_validation_ready" -NotePropertyValue $true -Force
        $fixture | ConvertTo-Json -Depth 8 | Set-Content -LiteralPath $refreshPath -Encoding UTF8
        & $PSCommandPath -RefreshPath $refreshPath -OutputPath $promptPath | Out-Null
        $content = Get-Content -LiteralPath $promptPath -Raw
        Assert-Fb2PromptSelfTest (Test-Path -LiteralPath $promptPath) "prompt file exists"
        Assert-Fb2PromptSelfTest ($content -match "fb2 AI Center") "prompt title"
        Assert-Fb2PromptSelfTest ($content -match "protected_live_preflight_satisfied") "protected live preflight gate"
        Assert-Fb2PromptSelfTest ($content -match "answer_source_validation_ready") "answer source validation gate"
        Assert-Fb2PromptSelfTest ($content -match "today_matches_analysis") "matrix item"
        Assert-Fb2PromptSelfTest ($content -match "证据新鲜度") "freshness section"
        Assert-Fb2PromptSelfTest ($content -match "fb2.main_project.evidence_freshness.v1") "freshness schema"
        Assert-Fb2PromptSelfTest ($content -match "缺口行动板") "gap action section"
        Assert-Fb2PromptSelfTest ($content -match "fb2.main_project.gap_action_board.v1") "gap action schema"
        Assert-Fb2PromptSelfTest ($content -match "计划能力 / 非生产边界") "planned capability section"
        Assert-Fb2PromptSelfTest ($content -match "fb2_p4_vector_contract_v1") "planned vector contract"
        Assert-Fb2PromptSelfTest ($content -match "fb2_p4_vector_readiness_plan_v1") "planned vector report"
        Assert-Fb2PromptSelfTest ($content -match "fb2_p4_source_enumerator_v1") "planned source enumerator report"
        Assert-Fb2PromptSelfTest ($content -match "source_specific_no_write_sample_available") "planned source enumerator status"
        Assert-Fb2PromptSelfTest ($content -match "fb2_p4_chunk_manifest_v1") "planned chunk manifest report"
        Assert-Fb2PromptSelfTest ($content -match "id_only_no_write_manifest_available") "planned chunk manifest status"
        Assert-Fb2PromptSelfTest ($content -match "fb2_p4_embedding_build_dry_run_v1") "planned embedding dry-run report"
        Assert-Fb2PromptSelfTest ($content -match "contract_design_committed_embedding_not_started") "planned vector status"
        Assert-Fb2PromptSelfTest ($content -match "production_grounding") "planned vector production boundary"
        Assert-Fb2PromptSelfTest ($content -match "blocks_data_goal") "planned vector non-blocking boundary"
        Assert-Fb2PromptSelfTest ($content -match "answer_time_vector_candidates_enabled") "planned vector answer-time disabled boundary"
        Assert-Fb2PromptSelfTest ($content -match "writes_public_group_messages") "planned source enumerator no public group write boundary"
        Assert-Fb2PromptSelfTest ($content -match "ready_to_write_embeddings") "planned source enumerator no embedding write boundary"
        Assert-Fb2PromptSelfTest ($content -match "writes_chunk_manifest_file") "planned chunk manifest no file write boundary"
        Assert-Fb2PromptSelfTest ($content -match "persists_manifest_rows") "planned chunk manifest no row persistence boundary"
        Assert-Fb2PromptSelfTest ($content -match "source_payload_included") "planned chunk manifest no source payload boundary"
        Assert-Fb2PromptSelfTest ($content -match "embedding_text_included") "planned chunk manifest no embedding text boundary"
        Assert-Fb2PromptSelfTest ($content -match "fb2 导出样本") "exported sample section"
        Assert-Fb2PromptSelfTest ($content -match "today_matches_context_pack") "exported sample row"
        Assert-Fb2PromptSelfTest ($content -match "\|\s*scenario\s*\|\s*audit\s*\|\s*sources\s*\|\s*business\s*\|\s*quality_history\s*\|\s*sha256\s*\|") "exported sample table classifies sources"
        Assert-Fb2PromptSelfTest ($content -match "\|\s*today_matches_context_pack\s*\|[^\r\n]+\|\s*match, odds, context_audit\s*\|\s*opinion_result_review_summary\s*\|") "review summary shown as quality history"
        Assert-Fb2PromptSelfTest ($content -match "generate_context_pack_sample_request") "context pack sample request command"
        Assert-Fb2PromptSelfTest ($content -match "validate_context_pack_sample_set") "context pack sample set validation command"
        Assert-Fb2PromptSelfTest ($content -match "validate_exported_context_pack_sample_set") "exported context pack sample set validation command"
        Assert-Fb2PromptSelfTest ($content -match "validate_context_projection_log") "context projection log validation command"
        Assert-Fb2PromptSelfTest ($content -match "validate_user_scenario_audit") "user scenario audit validation command"
        Assert-Fb2PromptSelfTest ($content -match "validate_current_state") "current state validation command"
        Assert-Fb2PromptSelfTest ($content -match "validate_public_contract_status") "public contract status validation command"
        Assert-Fb2PromptSelfTest ($content -match "validate_server_deploy_status") "server deploy status validation command"
        Assert-Fb2PromptSelfTest ($content -match "validate_project_direct_network_policy") "project direct network policy validation command"
        Assert-Fb2PromptSelfTest ($content -match "validate_context_format_route") "context format route validation command"
        Assert-Fb2PromptSelfTest ($content -match "validate_read_only_direct_read") "read-only direct read validation command"
        Assert-Fb2PromptSelfTest ($content -match "validate_evidence_freshness") "evidence freshness validation command"
        Assert-Fb2PromptSelfTest ($content -match "validate_evidence_privacy") "evidence privacy validation command"
        Assert-Fb2PromptSelfTest ($content -match "validate_completion_matrix") "completion matrix validation command"
        Assert-Fb2PromptSelfTest ($content -match "validate_handoff_prompt") "handoff prompt validation command"
        Assert-Fb2PromptSelfTest ($content -match "validate_live_preflight_request") "live preflight request validation command"
        Assert-Fb2PromptSelfTest ($content -match "validate_tokenless_continuation") "tokenless continuation validation command"
        Assert-Fb2PromptSelfTest ($content -match "<FB2_AI_CENTER_TOKEN>") "token placeholder"
        Assert-Fb2PromptSelfTest ($content -match "<FB2_PASSWORD>") "password placeholder"
        Assert-Fb2PromptSelfTest ($content -notmatch "secret-real-value") "token redacted"
        Assert-Fb2PromptSelfTest ($content -notmatch "secret-real-password") "password redacted"
        "== SelfTest Summary =="
        "failed=0"
    } finally {
        if (Test-Path -LiteralPath $tempRoot) {
            Remove-Item -LiteralPath $tempRoot -Recurse -Force
        }
    }
}

if ($SelfTest) {
    Invoke-Fb2PromptSelfTest
    exit 0
}

$root = Get-Fb2PromptRepoRoot
if ([string]::IsNullOrWhiteSpace($RefreshPath)) {
    $RefreshPath = Join-Path $root "target\fb2-ai-center\status-refresh-current.json"
} else {
    $RefreshPath = Resolve-Fb2PromptPath -Path $RefreshPath -Root $root
}
if ([string]::IsNullOrWhiteSpace($OutputPath)) {
    $OutputPath = Join-Path $root "target\fb2-ai-center\handoff-prompt-current.md"
} else {
    $OutputPath = Resolve-Fb2PromptPath -Path $OutputPath -Root $root
}

$parent = Split-Path -Parent $OutputPath
if (-not [string]::IsNullOrWhiteSpace($parent)) {
    New-Item -ItemType Directory -Force -Path $parent | Out-Null
}

$refresh = Read-Fb2PromptJson -Path $RefreshPath
$prompt = New-Fb2HandoffPrompt -Refresh $refresh -SourcePath $RefreshPath
Set-Content -LiteralPath $OutputPath -Value $prompt -Encoding UTF8

[pscustomobject]@{
    schema = "fb2.main_project.handoff_prompt_result.v1"
    source_refresh = $RefreshPath
    output_path = $OutputPath
    requirement_count = @($refresh.completion_matrix.requirements).Count
} | ConvertTo-Json -Depth 4
