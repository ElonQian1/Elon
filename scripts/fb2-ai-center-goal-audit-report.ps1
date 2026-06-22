#requires -Version 7.0

param(
    [string]$StatusPath = "",
    [string]$OutputPath = "",
    [string]$MarkdownPath = "",
    [switch]$SelfTest
)

$ErrorActionPreference = "Stop"

function Get-Fb2GoalAuditRepoRoot {
    Split-Path -Parent $PSScriptRoot
}

function Get-Fb2GoalAuditProperty {
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

function Test-Fb2GoalAuditTruthy {
    param([object]$Value)

    if ($null -eq $Value) {
        return $false
    }
    if ($Value -is [bool]) {
        return [bool]$Value
    }
    return ([string]$Value) -match "^(true|True|1)$"
}

function Test-Fb2GoalAuditTextPresent {
    param([object]$Value)

    return -not [string]::IsNullOrWhiteSpace([string]$Value)
}

function Test-Fb2GoalAuditZeroText {
    param([object]$Value)

    $text = [string]$Value
    if ([string]::IsNullOrWhiteSpace($text)) {
        return $false
    }
    return $text -match "\b(value|count)=0\b|^0$"
}

function Test-Fb2GoalAuditContains {
    param(
        [object]$Values,
        [string]$Expected
    )

    return (@($Values) -contains $Expected)
}

function Find-Fb2GoalAuditScenario {
    param(
        [object]$UserScenarioAudit,
        [string]$ScenarioId
    )

    foreach ($scenario in @((Get-Fb2GoalAuditProperty $UserScenarioAudit "scenarios" @()))) {
        if ([string](Get-Fb2GoalAuditProperty $scenario "id" "") -eq $ScenarioId) {
            return $scenario
        }
    }
    return $null
}

function ConvertTo-Fb2GoalAuditEvidenceText {
    param([object]$EvidenceObject)

    if ($null -eq $EvidenceObject) {
        return ""
    }

    $allowedNames = @(
        "context_audit_id",
        "citation_source_count",
        "context_pack_sha256",
        "selected_message_seed",
        "selected_message_reply",
        "summary_post",
        "summary_post_ready_for_mode",
        "feedback_complete",
        "context_projection_complete",
        "quality_unmatched_cited_sources"
    )
    $parts = [System.Collections.Generic.List[string]]::new()
    foreach ($property in @($EvidenceObject.PSObject.Properties)) {
        if (-not (@($allowedNames) -contains $property.Name)) {
            continue
        }
        $value = $property.Value
        if ($null -eq $value) {
            continue
        }

        $text = if ($value -is [string] -or $value -is [bool] -or $value -is [int] -or $value -is [long] -or $value -is [double]) {
            [string]$value
        } else {
            ($value | ConvertTo-Json -Compress -Depth 4)
        }

        if ($text.Length -gt 260) {
            $text = $text.Substring(0, 260) + "..."
        }
        [void]$parts.Add("$($property.Name)=$text")
    }

    return (@($parts) -join " ")
}

function New-Fb2GoalAuditRequirement {
    param(
        [string]$Id,
        [string]$Title,
        [bool]$Complete,
        [string]$Evidence,
        [string]$Missing = "",
        [bool]$Deferred = $false
    )

    $status = if ($Deferred) {
        "deferred"
    } elseif ($Complete) {
        "complete"
    } else {
        "missing"
    }

    [ordered]@{
        id = $Id
        title = $Title
        status = $status
        complete = $Complete
        deferred = $Deferred
        evidence = $Evidence
        missing = $Missing
    }
}

function New-Fb2GoalAuditScenarioRequirement {
    param(
        [object]$UserScenarioAudit,
        [string]$ScenarioId,
        [string]$Title
    )

    $scenario = Find-Fb2GoalAuditScenario -UserScenarioAudit $UserScenarioAudit -ScenarioId $ScenarioId
    $complete = Test-Fb2GoalAuditTruthy (Get-Fb2GoalAuditProperty $scenario "complete")
    $missing = @((Get-Fb2GoalAuditProperty $scenario "missing" @())) -join ","
    $evidenceObject = Get-Fb2GoalAuditProperty $scenario "evidence"
    $auditId = [string](Get-Fb2GoalAuditProperty $evidenceObject "context_audit_id")
    $sourceCount = [string](Get-Fb2GoalAuditProperty $evidenceObject "citation_source_count")
    $hash = [string](Get-Fb2GoalAuditProperty $evidenceObject "context_pack_sha256")
    $evidence = if ($complete) {
        if ((Test-Fb2GoalAuditTextPresent $auditId) -or (Test-Fb2GoalAuditTextPresent $sourceCount) -or (Test-Fb2GoalAuditTextPresent $hash)) {
            "scenario=$ScenarioId context_audit_id=$auditId citation_sources=$sourceCount context_pack_sha256=$hash"
        } else {
            $details = ConvertTo-Fb2GoalAuditEvidenceText -EvidenceObject $evidenceObject
            "scenario=$ScenarioId $details".Trim()
        }
    } else {
        "scenario=$ScenarioId incomplete"
    }

    New-Fb2GoalAuditRequirement `
        -Id $ScenarioId `
        -Title $Title `
        -Complete $complete `
        -Evidence $evidence `
        -Missing $missing
}

function Resolve-Fb2GoalAuditNextMinimumAction {
    param(
        [bool]$FullFinalComplete,
        [bool]$DataGoalComplete,
        [object]$Status,
        [object]$GoalGapAudit
    )

    if ($FullFinalComplete) {
        return "goal_complete"
    }
    if (-not $DataGoalComplete) {
        return "fix_missing_non_voice_requirements"
    }

    $livePreflight = Get-Fb2GoalAuditProperty $Status "live_preflight_request"
    $livePreflightMissing = @((Get-Fb2GoalAuditProperty $livePreflight "missing" @()))
    $tokenMissing = (
        (@($livePreflightMissing) -contains "FB2_AI_CENTER_TOKEN") `
            -or (@((Get-Fb2GoalAuditProperty $GoalGapAudit "missing" @())) -contains "FB2_AI_CENTER_TOKEN_live_permission_quality_refresh")
    )
    if ($tokenMissing) {
        return "set_FB2_AI_CENTER_TOKEN_then_run_DataOnlyAcceptance_PreflightOnly"
    }

    $gapNext = [string](Get-Fb2GoalAuditProperty $GoalGapAudit "next_smallest_action")
    if (-not [string]::IsNullOrWhiteSpace($gapNext)) {
        return $gapNext
    }

    return "keep_non_voice_regression_green_resume_ASR_TTS_only_when_user_unpauses"
}

function Read-Fb2GoalAuditStatus {
    param([string]$Path)

    if ([string]::IsNullOrWhiteSpace($Path) -or -not (Test-Path -LiteralPath $Path)) {
        throw "Status file not found: $Path. Run scripts\smoke-fb2-ai-center-status.ps1 first."
    }
    Get-Content -LiteralPath $Path -Raw | ConvertFrom-Json
}

function New-Fb2GoalAuditReport {
    param(
        [object]$Status,
        [string]$SourcePath
    )

    $public = Get-Fb2GoalAuditProperty $Status "latest_public_contract_status"
    $data = Get-Fb2GoalAuditProperty $Status "latest_data_only_acceptance"
    $fullAcceptance = Get-Fb2GoalAuditProperty $Status "latest_final_acceptance"
    $fullAcceptanceExists = Test-Fb2GoalAuditTruthy (Get-Fb2GoalAuditProperty $fullAcceptance "exists")
    $fullSource = if ($fullAcceptanceExists) { $fullAcceptance } else { $data }
    $goal = Get-Fb2GoalAuditProperty $Status "goal_completion"
    $requirements = Get-Fb2GoalAuditProperty $goal "requirements"
    $scenarioAudit = Get-Fb2GoalAuditProperty $Status "latest_user_scenario_audit"
    $answerReadiness = Get-Fb2GoalAuditProperty $Status "latest_context_answer_readiness"
    $sampleSet = Get-Fb2GoalAuditProperty $Status "latest_context_pack_sample_set"
    $contractSmoke = Get-Fb2GoalAuditProperty $Status "latest_contract_smoke_summary"
    $contractSmokeGates = Get-Fb2GoalAuditProperty $contractSmoke "gates"
    $gap = Get-Fb2GoalAuditProperty $Status "goal_gap_audit"
    $deferred = @((Get-Fb2GoalAuditProperty $gap "deferred_by_user" @()))
    $gapMissing = @((Get-Fb2GoalAuditProperty $gap "missing" @()))

    $publicComplete = Test-Fb2GoalAuditTruthy (Get-Fb2GoalAuditProperty $public "complete")
    $templateReady = ([string](Get-Fb2GoalAuditProperty $public "context_pack_template_schema") -eq "fb2.context_pack_template.v1")
    $domainIndexSchema = [string](Get-Fb2GoalAuditProperty $public "domain_context_index_schema")
    $domainIndexCount = [int](Get-Fb2GoalAuditProperty $public "domain_context_index_count" 0)
    $domainIndexIds = @((Get-Fb2GoalAuditProperty $public "domain_context_index_ids" @()))
    $domainIndexMissing = @((Get-Fb2GoalAuditProperty $public "missing" @()))
    $requiredDomainIndexes = @(
        "match_index",
        "odds_snapshot_index",
        "current_user_ticket_index",
        "platform_order_risk_index",
        "group_opinion_index",
        "opinion_memory_index",
        "context_audit_index",
        "feedback_quality_index"
    )
    $missingDomainIndexes = @($requiredDomainIndexes | Where-Object { -not (Test-Fb2GoalAuditContains $domainIndexIds $_) })
    # 领域索引契约是“fb2 数据如何长期高效服务主项目 AI”的机器边界；缺它时不能只靠 Context Pack 样本宣称数据目标完整。
    $domainIndexReady = (
        $publicComplete `
            -and $domainIndexSchema -eq "fb2.main_project.domain_context_index.v1" `
            -and $domainIndexCount -ge 8 `
            -and @($missingDomainIndexes).Count -eq 0 `
            -and @($domainIndexMissing).Count -eq 0
    )
    $sampleSetComplete = Test-Fb2GoalAuditTruthy (Get-Fb2GoalAuditProperty $sampleSet "complete")
    $answerReady = Test-Fb2GoalAuditTruthy (Get-Fb2GoalAuditProperty $answerReadiness "complete")
    $contextContractComplete = ($publicComplete -and $templateReady -and $sampleSetComplete -and $answerReady)
    # 这个门槛只证明主项目侧无写群契约仍可用；fb2 受保护 live 数据仍由 token preflight 单独证明。
    $contractSmokeComplete = Test-Fb2GoalAuditTruthy (Get-Fb2GoalAuditProperty $contractSmoke "complete")
    $contractSmokeFailedCount = [int](Get-Fb2GoalAuditProperty $contractSmoke "failed_count" 0)
    $contractSmokeReady = (
        $contractSmokeComplete `
            -and $contractSmokeFailedCount -eq 0 `
            -and (Test-Fb2GoalAuditTruthy (Get-Fb2GoalAuditProperty $contractSmokeGates "chat_bootstrap_ready")) `
            -and (Test-Fb2GoalAuditTruthy (Get-Fb2GoalAuditProperty $contractSmokeGates "ai_billing_policy_ready")) `
            -and (Test-Fb2GoalAuditTruthy (Get-Fb2GoalAuditProperty $contractSmokeGates "live_manifest_ready")) `
            -and (Test-Fb2GoalAuditTruthy (Get-Fb2GoalAuditProperty $contractSmokeGates "domain_contract_ready")) `
            -and (Test-Fb2GoalAuditTruthy (Get-Fb2GoalAuditProperty $contractSmokeGates "dynamic_discovery_ready")) `
            -and (Test-Fb2GoalAuditTruthy (Get-Fb2GoalAuditProperty $contractSmokeGates "protected_service_token_boundary_ready"))
    )
    $contractSmokeMissing = @()
    if (-not $contractSmokeComplete) { $contractSmokeMissing += "contract_smoke_summary" }
    if ($contractSmokeFailedCount -ne 0) { $contractSmokeMissing += "failed_checks" }
    foreach ($gateName in @(
            "chat_bootstrap_ready",
            "ai_billing_policy_ready",
            "live_manifest_ready",
            "domain_contract_ready",
            "dynamic_discovery_ready",
            "protected_service_token_boundary_ready"
        )) {
        if (-not (Test-Fb2GoalAuditTruthy (Get-Fb2GoalAuditProperty $contractSmokeGates $gateName))) {
            $contractSmokeMissing += $gateName
        }
    }

    $permissionBlocks = Get-Fb2GoalAuditProperty $data "permission_total_blocks"
    $qualityUnmatched = Get-Fb2GoalAuditProperty $data "quality_unmatched_cited_sources"
    $dataSuccess = Test-Fb2GoalAuditTruthy (Get-Fb2GoalAuditProperty $data "success")
    $fullSummarySuccess = Test-Fb2GoalAuditTruthy (Get-Fb2GoalAuditProperty $fullSource "success")
    $dataMode = [string](Get-Fb2GoalAuditProperty $fullSource "mode")
    $acceptanceScope = [string](Get-Fb2GoalAuditProperty $fullSource "acceptance_scope")
    $voiceStatus = [string](Get-Fb2GoalAuditProperty $fullSource "voice_status")
    $visibleExitZero = ([string](Get-Fb2GoalAuditProperty $fullSource "visible_chat_exit_code") -eq "0")
    $centerExitZero = ([string](Get-Fb2GoalAuditProperty $fullSource "final_acceptance_exit_code") -eq "0")
    $feedbackComplete = Test-Fb2GoalAuditTruthy (Get-Fb2GoalAuditProperty $data "feedback_complete")
    $directReadComplete = Test-Fb2GoalAuditTruthy (Get-Fb2GoalAuditProperty $data "direct_read_evidence_complete")
    $fullFeedbackComplete = Test-Fb2GoalAuditTruthy (Get-Fb2GoalAuditProperty $fullSource "feedback_complete")
    $fullDirectReadComplete = Test-Fb2GoalAuditTruthy (Get-Fb2GoalAuditProperty $fullSource "direct_read_evidence_complete")
    $voiceEvidencePresent = Test-Fb2GoalAuditTruthy (Get-Fb2GoalAuditProperty $requirements "voice_final_evidence_path_present")
    $voiceDeferred = ((@($deferred) -contains "ASR_TTS_final_evidence") -and -not $voiceEvidencePresent)
    $sameBatchFullFinalMissing = @($gapMissing) -contains "full_final_acceptance_same_batch_voice_and_visible_chat"
    $fullFinalSummaryComplete = (
        $fullAcceptanceExists `
            -and $fullSummarySuccess `
            -and $dataMode -eq "visible_final_acceptance" `
            -and $acceptanceScope -eq "full_final_acceptance" `
            -and $voiceStatus -eq "required" `
            -and $visibleExitZero `
            -and $centerExitZero `
            -and $fullFeedbackComplete `
            -and $fullDirectReadComplete `
            -and -not $sameBatchFullFinalMissing
    )

    $items = @()
    $items += New-Fb2GoalAuditRequirement `
        -Id "context_pack_contract" `
        -Title "fb2 uses XML-wrapped Markdown Context Pack plus JSON metadata, not raw DB/MCP-first" `
        -Complete $contextContractComplete `
        -Evidence "public_contract=$publicComplete template=$($public.context_pack_template_schema) sample_set=$sampleSetComplete answer_readiness=$answerReady"

    $items += New-Fb2GoalAuditRequirement `
        -Id "main_project_contract_smoke" `
        -Title "主项目默认 smoke 证明 chat-bootstrap、AI计费、live manifest、动态发现和 service-token 边界仍可用" `
        -Complete $contractSmokeReady `
        -Evidence "complete=$contractSmokeComplete failed=$contractSmokeFailedCount checks=$($contractSmoke.check_count) live_data=$($contractSmokeGates.fb2_live_data_status)" `
        -Missing (@($contractSmokeMissing | Select-Object -Unique) -join ",")

    $items += New-Fb2GoalAuditRequirement `
        -Id "domain_context_index_contract" `
        -Title "fb2 exposes the long-term domain index contract for matches, odds, tickets, platform risk, group opinions and feedback quality" `
        -Complete $domainIndexReady `
        -Evidence "schema=$domainIndexSchema count=$domainIndexCount indexes=$(@($domainIndexIds) -join ',')" `
        -Missing ($missingDomainIndexes -join ",")

    $items += New-Fb2GoalAuditScenarioRequirement -UserScenarioAudit $scenarioAudit -ScenarioId "today_matches_analysis" -Title "用户问今天比赛怎么看时可读取比赛事实和赔率"
    $items += New-Fb2GoalAuditScenarioRequirement -UserScenarioAudit $scenarioAudit -ScenarioId "my_ticket_analysis" -Title "用户问帮我分析我的票时只读取本人订单/票据"
    $items += New-Fb2GoalAuditScenarioRequirement -UserScenarioAudit $scenarioAudit -ScenarioId "platform_order_risk" -Title "用户问平台今天订单风险时只读取匿名平台汇总"
    $items += New-Fb2GoalAuditScenarioRequirement -UserScenarioAudit $scenarioAudit -ScenarioId "group_opinion_summary" -Title "用户问群里大家怎么看时读取群观点和观点记忆"
    $items += New-Fb2GoalAuditScenarioRequirement -UserScenarioAudit $scenarioAudit -ScenarioId "selected_message_review" -Title "用户长按消息 AI回复时复核被选中消息"
    $items += New-Fb2GoalAuditScenarioRequirement -UserScenarioAudit $scenarioAudit -ScenarioId "group_discussion_summary_post" -Title "群聊总结帖可读取讨论并写回质量反馈"
    $items += New-Fb2GoalAuditScenarioRequirement -UserScenarioAudit $scenarioAudit -ScenarioId "source_reference_audit" -Title "AI 可说明引用了哪些比赛、订单、群消息和审计来源"

    $items += New-Fb2GoalAuditRequirement `
        -Id "permission_safety" `
        -Title "权限边界：本人订单、平台匿名汇总、未授权请求拒绝并审计" `
        -Complete (Test-Fb2GoalAuditTruthy (Get-Fb2GoalAuditProperty $requirements "permission_quality_feedback")) `
        -Evidence "permission_total_blocks=$permissionBlocks quality_unmatched_cited_sources=$qualityUnmatched"

    $items += New-Fb2GoalAuditRequirement `
        -Id "feedback_quality_loop" `
        -Title "回答后写回 feedback/quality，引用来源 unmatched 为 0" `
        -Complete ($feedbackComplete -and (Test-Fb2GoalAuditZeroText $qualityUnmatched)) `
        -Evidence "feedback_complete=$feedbackComplete quality_unmatched_cited_sources=$qualityUnmatched"

    $items += New-Fb2GoalAuditRequirement `
        -Id "direct_group_chat_read" `
        -Title "测试 fb2 对话时用群聊 API 直读，不用截图作为验收证据" `
        -Complete $directReadComplete `
        -Evidence "direct_read_evidence_complete=$directReadComplete read_only_path=$($Status.latest_read_only_direct_read.path)"

    $items += New-Fb2GoalAuditRequirement `
        -Id "voice_final_evidence" `
        -Title "ASR/TTS final-ready 真机证据" `
        -Complete $voiceEvidencePresent `
        -Evidence "voice_final_evidence_path_present=$($requirements.voice_final_evidence_path_present)" `
        -Missing "ASR/TTS is intentionally deferred by user" `
        -Deferred $voiceDeferred

    $nonVoiceItems = @($items | Where-Object { $_.id -ne "voice_final_evidence" })
    $missingNonVoice = @($nonVoiceItems | Where-Object { -not [bool]$_.complete })
    $dataGoalComplete = ($missingNonVoice.Count -eq 0)
    $fullFinalComplete = ($dataGoalComplete -and $voiceEvidencePresent -and $fullFinalSummaryComplete)

    [ordered]@{
        schema = "fb2.main_project.goal_audit_report.v1"
        generated_at = (Get-Date).ToUniversalTime().ToString("o")
        source_status_path = $SourcePath
        source_summary_dirs = @((Get-Fb2GoalAuditProperty $Status "summary_dirs" @()))
        data_goal_complete = $dataGoalComplete
        full_final_complete = $fullFinalComplete
        stage = [string](Get-Fb2GoalAuditProperty $goal "stage")
        non_voice_ready = Test-Fb2GoalAuditTruthy (Get-Fb2GoalAuditProperty $goal "non_voice_ready")
        full_final_ready = Test-Fb2GoalAuditTruthy (Get-Fb2GoalAuditProperty $goal "full_final_ready")
        requirements = @($items)
        missing_non_voice_requirements = @($missingNonVoice | ForEach-Object { $_.id })
        deferred_requirements = @($items | Where-Object { [bool]$_.deferred } | ForEach-Object { $_.id })
        evidence_summary = [ordered]@{
            data_only_summary = [string](Get-Fb2GoalAuditProperty $data "path")
            visible_chat_exit_code = [string](Get-Fb2GoalAuditProperty $data "visible_chat_exit_code")
            final_acceptance_exit_code = [string](Get-Fb2GoalAuditProperty $data "final_acceptance_exit_code")
            scenario_my_ticket_orders = [string](Get-Fb2GoalAuditProperty $data "scenario_my_ticket_orders")
            platform_order_summary = [string](Get-Fb2GoalAuditProperty $data "platform_order_summary")
            permission_total_blocks = [string]$permissionBlocks
            quality_unmatched_cited_sources = [string]$qualityUnmatched
            direct_read_evidence_complete = $directReadComplete
            contract_smoke_summary = [string](Get-Fb2GoalAuditProperty $contractSmoke "path")
            contract_smoke_check_count = [string](Get-Fb2GoalAuditProperty $contractSmoke "check_count")
            contract_smoke_live_data_status = [string](Get-Fb2GoalAuditProperty $contractSmokeGates "fb2_live_data_status")
        }
        full_final_completion_evidence = [ordered]@{
            path = [string](Get-Fb2GoalAuditProperty $fullSource "path")
            exists = $fullAcceptanceExists
            mode = $dataMode
            acceptance_scope = $acceptanceScope
            voice_status = $voiceStatus
            success = $fullSummarySuccess
            visible_chat_exit_code_zero = $visibleExitZero
            final_acceptance_exit_code_zero = $centerExitZero
            feedback_complete = $fullFeedbackComplete
            direct_read_evidence_complete = $fullDirectReadComplete
            voice_final_evidence_path_present = $voiceEvidencePresent
            missing_same_batch_full_final = $sameBatchFullFinalMissing
        }
        next_minimum_action = Resolve-Fb2GoalAuditNextMinimumAction `
            -FullFinalComplete $fullFinalComplete `
            -DataGoalComplete $dataGoalComplete `
            -Status $Status `
            -GoalGapAudit $gap
    }
}

function ConvertTo-Fb2GoalAuditMarkdown {
    param([object]$Report)

    $lines = [System.Collections.Generic.List[string]]::new()
    [void]$lines.Add("# fb2 AI Center Goal Audit")
    [void]$lines.Add("")
    [void]$lines.Add("- schema: $($Report.schema)")
    [void]$lines.Add("- generated_at: $($Report.generated_at)")
    [void]$lines.Add("- stage: $($Report.stage)")
    [void]$lines.Add("- data_goal_complete: $($Report.data_goal_complete)")
    [void]$lines.Add("- full_final_complete: $($Report.full_final_complete)")
    [void]$lines.Add("- next_minimum_action: $($Report.next_minimum_action)")
    [void]$lines.Add("")
    [void]$lines.Add("## Requirements")
    [void]$lines.Add("")
    [void]$lines.Add("| id | status | evidence |")
    [void]$lines.Add("|---|---|---|")
    foreach ($item in @($Report.requirements)) {
        [void]$lines.Add("| $($item.id) | $($item.status) | $($item.evidence) |")
    }
    [void]$lines.Add("")
    [void]$lines.Add("## Evidence Summary")
    [void]$lines.Add("")
    $summaryEntries = if ($Report.evidence_summary -is [System.Collections.IDictionary]) {
        $Report.evidence_summary.GetEnumerator()
    } else {
        $Report.evidence_summary.PSObject.Properties
    }
    foreach ($property in $summaryEntries) {
        $name = if ($property.PSObject.Properties["Name"]) { $property.Name } else { $property.Key }
        [void]$lines.Add("- $($name): $($property.Value)")
    }
    [void]$lines.Add("")
    [void]$lines.Add("## Full Final Completion Evidence")
    [void]$lines.Add("")
    $fullEntries = if ($Report.full_final_completion_evidence -is [System.Collections.IDictionary]) {
        $Report.full_final_completion_evidence.GetEnumerator()
    } else {
        $Report.full_final_completion_evidence.PSObject.Properties
    }
    foreach ($property in $fullEntries) {
        $name = if ($property.PSObject.Properties["Name"]) { $property.Name } else { $property.Key }
        [void]$lines.Add("- $($name): $($property.Value)")
    }

    $lines -join [Environment]::NewLine
}

function Invoke-Fb2GoalAuditSelfTest {
    $scenarioIds = @(
        "today_matches_analysis",
        "my_ticket_analysis",
        "platform_order_risk",
        "group_opinion_summary",
        "selected_message_review",
        "group_discussion_summary_post",
        "source_reference_audit"
    )
    $scenarios = @($scenarioIds | ForEach-Object {
            [pscustomobject]@{
                id = $_
                complete = $true
                missing = @()
                evidence = [pscustomobject]@{
                    context_audit_id = "audit-$_"
                    citation_source_count = 3
                    context_pack_sha256 = "hash-$_"
                }
            }
        })
    $status = [pscustomobject]@{
        summary_dirs = @("target/fb2-ai-center")
        latest_public_contract_status = [pscustomobject]@{
            complete = $true
            context_pack_template_schema = "fb2.context_pack_template.v1"
            domain_context_index_schema = "fb2.main_project.domain_context_index.v1"
            domain_context_index_count = 8
            domain_context_index_ids = @(
                "match_index",
                "odds_snapshot_index",
                "current_user_ticket_index",
                "platform_order_risk_index",
                "group_opinion_index",
                "opinion_memory_index",
                "context_audit_index",
                "feedback_quality_index"
            )
            missing = @()
        }
        latest_context_pack_sample_set = [pscustomobject]@{ complete = $true }
        latest_context_answer_readiness = [pscustomobject]@{ complete = $true }
        latest_contract_smoke_summary = [pscustomobject]@{
            complete = $true
            path = "contract-smoke-summary.json"
            failed_count = 0
            skipped_count = 1
            check_count = 255
            gates = [pscustomobject]@{
                chat_bootstrap_ready = $true
                voice_contract_ready = $true
                ai_billing_policy_ready = $true
                live_manifest_ready = $true
                domain_contract_ready = $true
                dynamic_discovery_ready = $true
                protected_service_token_boundary_ready = $true
                fb2_live_data_status = "skipped_missing_FB2_AI_CENTER_TOKEN"
            }
            missing = @()
        }
        latest_user_scenario_audit = [pscustomobject]@{ scenarios = $scenarios }
        latest_read_only_direct_read = [pscustomobject]@{ path = "read-only.json" }
        latest_data_only_acceptance = [pscustomobject]@{
            path = "data-only.json"
            success = $true
            mode = "visible_data_only_acceptance"
            acceptance_scope = "data_permission_quality_visible_chat_without_voice"
            voice_status = "deferred_by_user"
            feedback_complete = $true
            direct_read_evidence_complete = $true
            visible_chat_exit_code = "0"
            final_acceptance_exit_code = "0"
            scenario_my_ticket_orders = "count=10 min=1"
            platform_order_summary = "count=1 min=1"
            permission_total_blocks = "count=4"
            quality_unmatched_cited_sources = "value=0"
        }
        latest_final_acceptance = [pscustomobject]@{
            exists = $false
            path = ""
        }
        goal_completion = [pscustomobject]@{
            stage = "non_voice_data_chat_permission_quality_ready_voice_deferred"
            non_voice_ready = $true
            full_final_ready = $false
            requirements = [pscustomobject]@{
                permission_quality_feedback = $true
                voice_final_evidence_path_present = $false
            }
        }
        goal_gap_audit = [pscustomobject]@{
            missing = @("FB2_AI_CENTER_TOKEN_live_permission_quality_refresh", "full_final_acceptance_same_batch_voice_and_visible_chat")
            deferred_by_user = @("ASR_TTS_final_evidence")
            next_smallest_action = "set_FB2_AI_CENTER_TOKEN_then_run_DataOnlyAcceptance_PreflightOnly"
        }
        live_preflight_request = [pscustomobject]@{
            missing = @("FB2_AI_CENTER_TOKEN")
            ready_without_token = $true
        }
    }

    $report = New-Fb2GoalAuditReport -Status $status -SourcePath "selftest.json"
    $failed = 0
    if ($report.schema -ne "fb2.main_project.goal_audit_report.v1") { $failed++ }
    if (-not [bool]$report.data_goal_complete) { $failed++ }
    if ([bool]$report.full_final_complete) { $failed++ }
    if ($report.next_minimum_action -ne "set_FB2_AI_CENTER_TOKEN_then_run_DataOnlyAcceptance_PreflightOnly") { $failed++ }
    if (-not (@($report.deferred_requirements) -contains "voice_final_evidence")) { $failed++ }
    if (@($report.missing_non_voice_requirements).Count -ne 0) { $failed++ }
    if (-not (@($report.requirements | ForEach-Object { $_.id }) -contains "main_project_contract_smoke")) { $failed++ }
    if (-not (Test-Fb2GoalAuditTextPresent (Get-Fb2GoalAuditProperty $report.evidence_summary "contract_smoke_summary"))) { $failed++ }
    if (-not (@($report.requirements | ForEach-Object { $_.id }) -contains "domain_context_index_contract")) { $failed++ }

    $badScenario = $status | ConvertTo-Json -Depth 12 | ConvertFrom-Json
    $badScenario.latest_user_scenario_audit.scenarios[1].complete = $false
    $badReport = New-Fb2GoalAuditReport -Status $badScenario -SourcePath "selftest-bad.json"
    if ([bool]$badReport.data_goal_complete) { $failed++ }
    if (-not (@($badReport.missing_non_voice_requirements) -contains "my_ticket_analysis")) { $failed++ }

    $badDomainIndex = $status | ConvertTo-Json -Depth 12 | ConvertFrom-Json
    $badDomainIndex.latest_public_contract_status.domain_context_index_ids = @("match_index")
    $badDomainIndexReport = New-Fb2GoalAuditReport -Status $badDomainIndex -SourcePath "selftest-bad-domain-index.json"
    if ([bool]$badDomainIndexReport.data_goal_complete) { $failed++ }
    if (-not (@($badDomainIndexReport.missing_non_voice_requirements) -contains "domain_context_index_contract")) { $failed++ }

    $badContractSmoke = $status | ConvertTo-Json -Depth 12 | ConvertFrom-Json
    $badContractSmoke.latest_contract_smoke_summary.gates.chat_bootstrap_ready = $false
    $badContractSmokeReport = New-Fb2GoalAuditReport -Status $badContractSmoke -SourcePath "selftest-bad-contract-smoke.json"
    if ([bool]$badContractSmokeReport.data_goal_complete) { $failed++ }
    if (-not (@($badContractSmokeReport.missing_non_voice_requirements) -contains "main_project_contract_smoke")) { $failed++ }

    $voiceOnly = $status | ConvertTo-Json -Depth 12 | ConvertFrom-Json
    $voiceOnly.goal_completion.requirements.voice_final_evidence_path_present = $true
    $voiceOnly.goal_gap_audit.deferred_by_user = @()
    $voiceOnlyReport = New-Fb2GoalAuditReport -Status $voiceOnly -SourcePath "selftest-voice-only.json"
    if (-not [bool]$voiceOnlyReport.data_goal_complete) { $failed++ }
    if ([bool]$voiceOnlyReport.full_final_complete) { $failed++ }

    $fullFinal = $voiceOnly | ConvertTo-Json -Depth 12 | ConvertFrom-Json
    $fullFinal.latest_final_acceptance = [pscustomobject]@{
        exists = $true
        path = "final-acceptance.json"
        success = $true
        mode = "visible_final_acceptance"
        acceptance_scope = "full_final_acceptance"
        voice_status = "required"
        feedback_complete = $true
        direct_read_evidence_complete = $true
        visible_chat_exit_code = "0"
        final_acceptance_exit_code = "0"
    }
    $fullFinal.goal_gap_audit.missing = @()
    $fullFinalReport = New-Fb2GoalAuditReport -Status $fullFinal -SourcePath "selftest-full-final.json"
    if (-not [bool]$fullFinalReport.full_final_complete) { $failed++ }

    Write-Output "== SelfTest Summary =="
    Write-Output "failed=$failed"
    if ($failed -gt 0) {
        exit 1
    }
}

if ($SelfTest) {
    Invoke-Fb2GoalAuditSelfTest
    exit 0
}

if ([string]::IsNullOrWhiteSpace($StatusPath)) {
    $StatusPath = Join-Path (Get-Fb2GoalAuditRepoRoot) "target\fb2-ai-center\status-current.json"
}

$statusObject = Read-Fb2GoalAuditStatus -Path $StatusPath
$reportObject = New-Fb2GoalAuditReport -Status $statusObject -SourcePath $StatusPath
$json = $reportObject | ConvertTo-Json -Depth 10

if (-not [string]::IsNullOrWhiteSpace($OutputPath)) {
    $outputDir = Split-Path -Parent $OutputPath
    if ($outputDir -and -not (Test-Path -LiteralPath $outputDir)) {
        New-Item -ItemType Directory -Path $outputDir -Force | Out-Null
    }
    Set-Content -Path $OutputPath -Value $json -Encoding UTF8
}

if (-not [string]::IsNullOrWhiteSpace($MarkdownPath)) {
    $markdownDir = Split-Path -Parent $MarkdownPath
    if ($markdownDir -and -not (Test-Path -LiteralPath $markdownDir)) {
        New-Item -ItemType Directory -Path $markdownDir -Force | Out-Null
    }
    Set-Content -Path $MarkdownPath -Value (ConvertTo-Fb2GoalAuditMarkdown -Report $reportObject) -Encoding UTF8
}

Write-Output $json
