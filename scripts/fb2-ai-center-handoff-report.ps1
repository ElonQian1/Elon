#requires -Version 7.0

param(
    [string]$StatusPath = "",
    [string]$OutputPath = "",
    [string]$MarkdownPath = "",
    [switch]$SelfTest
)

$ErrorActionPreference = "Stop"

function Get-Fb2HandoffRepoRoot {
    Split-Path -Parent $PSScriptRoot
}

function Get-Fb2HandoffProperty {
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

function Test-Fb2HandoffTruthy {
    param([object]$Value)

    if ($null -eq $Value) {
        return $false
    }
    if ($Value -is [bool]) {
        return [bool]$Value
    }
    return ([string]$Value) -match "^(true|True|1)$"
}

function ConvertTo-Fb2HandoffText {
    param([object]$Value)

    if ($null -eq $Value) {
        return ""
    }
    return [string]$Value
}

function Read-Fb2HandoffJson {
    param([string]$Path)

    if ([string]::IsNullOrWhiteSpace($Path) -or -not (Test-Path -LiteralPath $Path)) {
        throw "Status file not found: $Path. Run scripts\smoke-fb2-ai-center-status.ps1 first."
    }
    Get-Content -LiteralPath $Path -Raw | ConvertFrom-Json
}

function New-Fb2HandoffReport {
    param(
        [object]$Status,
        [string]$SourcePath
    )

    $validation = Get-Fb2HandoffProperty $Status "validation_scope"
    $public = Get-Fb2HandoffProperty $Status "latest_public_contract_status"
    $contractSmoke = Get-Fb2HandoffProperty $Status "latest_contract_smoke_summary"
    $contractSmokeGates = Get-Fb2HandoffProperty $contractSmoke "gates"
    $readOnly = Get-Fb2HandoffProperty $Status "latest_read_only_direct_read"
    $readOnlyEvidence = Get-Fb2HandoffProperty $readOnly "evidence"
    $answerReadiness = Get-Fb2HandoffProperty $Status "latest_context_answer_readiness"
    $scenarioAudit = Get-Fb2HandoffProperty $Status "latest_user_scenario_audit"
    $goalGap = Get-Fb2HandoffProperty $Status "goal_gap_audit"
    $goalFlags = Get-Fb2HandoffProperty $goalGap "current_flags"
    $livePreflight = Get-Fb2HandoffProperty $Status "live_preflight_request"
    $coordination = Get-Fb2HandoffProperty $Status "coordination"
    $coordinationNext = Get-Fb2HandoffProperty $coordination "next_action_by_owner"
    $commands = Get-Fb2HandoffProperty $livePreflight "commands"
    $summaryDirs = @((Get-Fb2HandoffProperty $Status "summary_dirs" @()))

    $publicComplete = Test-Fb2HandoffTruthy (Get-Fb2HandoffProperty $public "complete")
    $contractSmokeComplete = Test-Fb2HandoffTruthy (Get-Fb2HandoffProperty $contractSmoke "complete")
    $directReadComplete = Test-Fb2HandoffTruthy (Get-Fb2HandoffProperty $readOnly "complete")
    $answerReady = Test-Fb2HandoffTruthy (Get-Fb2HandoffProperty $answerReadiness "complete")
    $scenarioComplete = Test-Fb2HandoffTruthy (Get-Fb2HandoffProperty $scenarioAudit "complete")
    $nonVoiceReady = Test-Fb2HandoffTruthy (Get-Fb2HandoffProperty $goalFlags "non_voice_ready")
    $fullFinalReady = Test-Fb2HandoffTruthy (Get-Fb2HandoffProperty $goalFlags "full_final_ready")
    $tokenPresent = Test-Fb2HandoffTruthy (Get-Fb2HandoffProperty $goalFlags "token_present")
    $voiceEvidencePresent = Test-Fb2HandoffTruthy (Get-Fb2HandoffProperty $goalFlags "voice_evidence_path_present")

    $stage = if ($fullFinalReady) {
        "full_final_ready"
    } elseif ($nonVoiceReady) {
        "non_voice_ready_voice_deferred"
    } elseif ($answerReady -and $directReadComplete -and $publicComplete) {
        "contract_direct_read_and_offline_context_ready_needs_live_refresh"
    } else {
        "needs_more_evidence"
    }

    $mainNext = ConvertTo-Fb2HandoffText (Get-Fb2HandoffProperty $coordinationNext "main_project")
    $fb2Next = ConvertTo-Fb2HandoffText (Get-Fb2HandoffProperty $coordinationNext "fb2_project")
    $sharedNext = ConvertTo-Fb2HandoffText (Get-Fb2HandoffProperty $coordinationNext "shared")
    if ([string]::IsNullOrWhiteSpace($mainNext)) {
        $mainNext = ConvertTo-Fb2HandoffText (Get-Fb2HandoffProperty $goalGap "next_smallest_action")
    }
    if ([string]::IsNullOrWhiteSpace($fb2Next)) {
        $fb2Next = if ($tokenPresent) {
            "fix any live context, permission, quality, or feedback failures surfaced by DataOnlyAcceptance"
        } else {
            "provide FB2_AI_CENTER_TOKEN for live permission quality refresh, or keep exported Context Pack samples current"
        }
    }
    if ([string]::IsNullOrWhiteSpace($sharedNext)) {
        $sharedNext = if ($voiceEvidencePresent) { "run full final acceptance" } else { "ASR/TTS final evidence remains deferred" }
    }

    [ordered]@{
        schema = "fb2.main_project.handoff_report.v1"
        generated_at = (Get-Date).ToUniversalTime().ToString("o")
        source_status_path = $SourcePath
        source_status_generated_at = ConvertTo-Fb2HandoffText (Get-Fb2HandoffProperty $Status "generated_at")
        source_summary_dirs = @($summaryDirs)
        stage = $stage
        verdict = [ordered]@{
            public_contract_ready = $publicComplete
            contract_smoke_ready = $contractSmokeComplete
            group_chat_direct_read_ready = $directReadComplete
            context_answer_readiness_ready = $answerReady
            user_scenario_audit_ready = $scenarioComplete
            non_voice_ready = $nonVoiceReady
            full_final_ready = $fullFinalReady
            token_present = $tokenPresent
            voice_evidence_path_present = $voiceEvidencePresent
        }
        evidence_policy = [ordered]@{
            group_chat_test_method = ConvertTo-Fb2HandoffText (Get-Fb2HandoffProperty $public "group_chat_test_method" (Get-Fb2HandoffProperty $validation "group_chat_evidence"))
            screenshots_accepted = Test-Fb2HandoffTruthy (Get-Fb2HandoffProperty $validation "screenshots_accepted_for_group_chat")
            writes_group_messages = Test-Fb2HandoffTruthy (Get-Fb2HandoffProperty $validation "writes_group_messages")
            stores_message_body = Test-Fb2HandoffTruthy (Get-Fb2HandoffProperty $validation "stores_message_body")
            required_group_message_fields = @((Get-Fb2HandoffProperty $public "required_group_message_fields" @("message_id", "text_len", "text_sha256")))
        }
        public_contract = [ordered]@{
            complete = $publicComplete
            server_version = ConvertTo-Fb2HandoffText (Get-Fb2HandoffProperty $public "server_version")
            server_git_sha = ConvertTo-Fb2HandoffText (Get-Fb2HandoffProperty $public "server_git_sha")
            live_tool_count = [int](Get-Fb2HandoffProperty $public "live_tool_count" 0)
            limitations = @((Get-Fb2HandoffProperty $public "limitations" @()))
        }
        contract_smoke = [ordered]@{
            complete = $contractSmokeComplete
            path = ConvertTo-Fb2HandoffText (Get-Fb2HandoffProperty $contractSmoke "path")
            failed_count = [int](Get-Fb2HandoffProperty $contractSmoke "failed_count" 0)
            skipped_count = [int](Get-Fb2HandoffProperty $contractSmoke "skipped_count" 0)
            check_count = [int](Get-Fb2HandoffProperty $contractSmoke "check_count" 0)
            chat_bootstrap_ready = Test-Fb2HandoffTruthy (Get-Fb2HandoffProperty $contractSmokeGates "chat_bootstrap_ready")
            ai_billing_policy_ready = Test-Fb2HandoffTruthy (Get-Fb2HandoffProperty $contractSmokeGates "ai_billing_policy_ready")
            live_manifest_ready = Test-Fb2HandoffTruthy (Get-Fb2HandoffProperty $contractSmokeGates "live_manifest_ready")
            protected_service_token_boundary_ready = Test-Fb2HandoffTruthy (Get-Fb2HandoffProperty $contractSmokeGates "protected_service_token_boundary_ready")
            fb2_live_data_status = ConvertTo-Fb2HandoffText (Get-Fb2HandoffProperty $contractSmokeGates "fb2_live_data_status")
            missing = @((Get-Fb2HandoffProperty $contractSmoke "missing" @()))
        }
        group_chat_direct_read = [ordered]@{
            complete = $directReadComplete
            path = ConvertTo-Fb2HandoffText (Get-Fb2HandoffProperty $readOnly "path")
            group_id = ConvertTo-Fb2HandoffText (Get-Fb2HandoffProperty $readOnlyEvidence "group_id")
            message_count = [int](Get-Fb2HandoffProperty $readOnlyEvidence "message_count" 0)
            sample_message_id = ConvertTo-Fb2HandoffText (Get-Fb2HandoffProperty $readOnlyEvidence "sample_message_id")
            sample_text_sha256 = ConvertTo-Fb2HandoffText (Get-Fb2HandoffProperty $readOnlyEvidence "sample_text_sha256")
            writes = Test-Fb2HandoffTruthy (Get-Fb2HandoffProperty $readOnlyEvidence "writes")
        }
        answer_readiness = [ordered]@{
            complete = $answerReady
            scenario_count = [int](Get-Fb2HandoffProperty $answerReadiness "scenario_count" 0)
            passed_count = [int](Get-Fb2HandoffProperty $answerReadiness "passed_count" 0)
            failed_count = [int](Get-Fb2HandoffProperty $answerReadiness "failed_count" 0)
            note = ConvertTo-Fb2HandoffText (Get-Fb2HandoffProperty $answerReadiness "note")
        }
        user_scenario_audit = [ordered]@{
            complete = $scenarioComplete
            scenario_count = [int](Get-Fb2HandoffProperty $scenarioAudit "scenario_count" 0)
            complete_count = [int](Get-Fb2HandoffProperty $scenarioAudit "complete_count" 0)
            failed_count = [int](Get-Fb2HandoffProperty $scenarioAudit "failed_count" 0)
            missing = @((Get-Fb2HandoffProperty $scenarioAudit "missing" @()))
        }
        blockers = [ordered]@{
            missing = @((Get-Fb2HandoffProperty $goalGap "missing" @()))
            refresh_gaps = @((Get-Fb2HandoffProperty $Status "refresh_gaps" @()))
            deferred_by_user = @((Get-Fb2HandoffProperty $goalGap "deferred_by_user" @()))
            blocked_by_external_secret = Test-Fb2HandoffTruthy (Get-Fb2HandoffProperty $goalGap "blocked_by_external_secret")
        }
        next_action_by_owner = [ordered]@{
            main_project = $mainNext
            fb2_project = $fb2Next
            shared = $sharedNext
        }
        safe_commands = [ordered]@{
            refresh_public_contract = 'pwsh -NoProfile -ExecutionPolicy Bypass -File scripts\fb2-public-contract-status.ps1 -OutputPath target\fb2-ai-center\public-contract-status-current.json'
            refresh_status = 'pwsh -NoProfile -ExecutionPolicy Bypass -File scripts\smoke-fb2-ai-center-status.ps1 -OutputPath target\fb2-ai-center\status-current.json'
            refresh_status_with_extra_evidence = 'pwsh -NoProfile -ExecutionPolicy Bypass -File scripts\smoke-fb2-ai-center-status.ps1 -EvidenceDirs "D:\rust\active-projects\elon cli\target\fb2-ai-center" -OutputPath target\fb2-ai-center\status-current.json'
            no_write_direct_read = ConvertTo-Fb2HandoffText (Get-Fb2HandoffProperty $commands "no_write_direct_read")
            data_only_preflight = ConvertTo-Fb2HandoffText (Get-Fb2HandoffProperty $commands "data_only_preflight")
            visible_regression_requires_authorization = ConvertTo-Fb2HandoffText (Get-Fb2HandoffProperty $commands "visible_regression_requires_authorization")
            generate_handoff_report = 'pwsh -NoProfile -ExecutionPolicy Bypass -File scripts\fb2-ai-center-handoff-report.ps1 -StatusPath target\fb2-ai-center\status-current.json -OutputPath target\fb2-ai-center\handoff-current.json -MarkdownPath target\fb2-ai-center\handoff-current.md'
        }
    }
}

function ConvertTo-Fb2HandoffMarkdown {
    param([object]$Report)

    $lines = [System.Collections.Generic.List[string]]::new()
    [void]$lines.Add("# fb2 AI Center Handoff")
    [void]$lines.Add("")
    [void]$lines.Add("- schema: $($Report.schema)")
    [void]$lines.Add("- generated_at: $($Report.generated_at)")
    [void]$lines.Add("- stage: $($Report.stage)")
    [void]$lines.Add("- source_status: $($Report.source_status_path)")
    if (@($Report.source_summary_dirs).Count -gt 0) {
        [void]$lines.Add("- source_summary_dirs: $((@($Report.source_summary_dirs) -join " | "))")
    }
    [void]$lines.Add("")
    [void]$lines.Add("## Verdict")
    [void]$lines.Add("")
    [void]$lines.Add("| item | value |")
    [void]$lines.Add("|---|---|")
    foreach ($name in @(
        "public_contract_ready",
        "contract_smoke_ready",
        "group_chat_direct_read_ready",
        "context_answer_readiness_ready",
        "user_scenario_audit_ready",
        "non_voice_ready",
        "full_final_ready",
        "token_present",
        "voice_evidence_path_present"
    )) {
        [void]$lines.Add("| $name | $($Report.verdict.$name) |")
    }
    [void]$lines.Add("")
    [void]$lines.Add("## Evidence Policy")
    [void]$lines.Add("")
    [void]$lines.Add("- group_chat_test_method: $($Report.evidence_policy.group_chat_test_method)")
    [void]$lines.Add("- screenshots_accepted: $($Report.evidence_policy.screenshots_accepted)")
    [void]$lines.Add("- writes_group_messages: $($Report.evidence_policy.writes_group_messages)")
    [void]$lines.Add("- required_group_message_fields: $(@($Report.evidence_policy.required_group_message_fields) -join ', ')")
    [void]$lines.Add("")
    [void]$lines.Add("## Current Evidence")
    [void]$lines.Add("")
    [void]$lines.Add("- public_contract: complete=$($Report.public_contract.complete), server=$($Report.public_contract.server_version), sha=$($Report.public_contract.server_git_sha), live_tool_count=$($Report.public_contract.live_tool_count)")
    [void]$lines.Add("- contract_smoke: complete=$($Report.contract_smoke.complete), failed=$($Report.contract_smoke.failed_count), skipped=$($Report.contract_smoke.skipped_count), checks=$($Report.contract_smoke.check_count), chat_bootstrap=$($Report.contract_smoke.chat_bootstrap_ready), ai_billing=$($Report.contract_smoke.ai_billing_policy_ready), live_data=$($Report.contract_smoke.fb2_live_data_status)")
    [void]$lines.Add("- direct_group_read: complete=$($Report.group_chat_direct_read.complete), group=$($Report.group_chat_direct_read.group_id), count=$($Report.group_chat_direct_read.message_count), sample=$($Report.group_chat_direct_read.sample_message_id), sha=$($Report.group_chat_direct_read.sample_text_sha256), writes=$($Report.group_chat_direct_read.writes)")
    [void]$lines.Add("- answer_readiness: complete=$($Report.answer_readiness.complete), passed=$($Report.answer_readiness.passed_count)/$($Report.answer_readiness.scenario_count)")
    [void]$lines.Add("- user_scenario_audit: complete=$($Report.user_scenario_audit.complete), complete_count=$($Report.user_scenario_audit.complete_count)/$($Report.user_scenario_audit.scenario_count), missing=$(@($Report.user_scenario_audit.missing) -join ', ')")
    [void]$lines.Add("")
    [void]$lines.Add("## Blockers")
    [void]$lines.Add("")
    [void]$lines.Add("- missing: $(@($Report.blockers.missing) -join ', ')")
    [void]$lines.Add("- refresh_gaps: $(@($Report.blockers.refresh_gaps) -join ', ')")
    [void]$lines.Add("- deferred_by_user: $(@($Report.blockers.deferred_by_user) -join ', ')")
    [void]$lines.Add("")
    [void]$lines.Add("## Next Actions")
    [void]$lines.Add("")
    [void]$lines.Add("- main_project: $($Report.next_action_by_owner.main_project)")
    [void]$lines.Add("- fb2_project: $($Report.next_action_by_owner.fb2_project)")
    [void]$lines.Add("- shared: $($Report.next_action_by_owner.shared)")
    [void]$lines.Add("")
    [void]$lines.Add("## Safe Commands")
    [void]$lines.Add("")
    foreach ($name in @(
        "refresh_public_contract",
        "refresh_status",
        "refresh_status_with_extra_evidence",
        "no_write_direct_read",
        "data_only_preflight",
        "visible_regression_requires_authorization",
        "generate_handoff_report"
    )) {
        [void]$lines.Add("- ${name}: ``$($Report.safe_commands.$name)``")
    }

    $lines -join [Environment]::NewLine
}

function Invoke-Fb2HandoffSelfTest {
    $status = [pscustomobject]@{
        schema = "fb2.main_project.status_snapshot.v1"
        generated_at = "2026-01-01T00:00:00Z"
        validation_scope = [pscustomobject]@{
            group_chat_evidence = "api_direct_read_summary_only"
            screenshots_accepted_for_group_chat = $false
            writes_group_messages = $false
            stores_message_body = $false
        }
        latest_public_contract_status = [pscustomobject]@{
            complete = $true
            group_chat_test_method = "direct_api_read"
            required_group_message_fields = @("message_id", "text_len", "text_sha256")
            server_version = "selftest"
            server_git_sha = "abc123"
            live_tool_count = 34
            limitations = @("does_not_verify_fb2_live_context_pack_or_orders")
        }
        latest_contract_smoke_summary = [pscustomobject]@{
            complete = $true
            path = "contract-smoke-summary.json"
            failed_count = 0
            skipped_count = 1
            check_count = 12
            gates = [pscustomobject]@{
                chat_bootstrap_ready = $true
                ai_billing_policy_ready = $true
                live_manifest_ready = $true
                protected_service_token_boundary_ready = $true
                fb2_live_data_status = "skipped_missing_FB2_AI_CENTER_TOKEN"
            }
            missing = @()
        }
        latest_read_only_direct_read = [pscustomobject]@{
            complete = $true
            path = "read-only.json"
            evidence = [pscustomobject]@{
                group_id = "ext_fb2_official"
                message_count = 80
                sample_message_id = "gai_sample"
                sample_text_sha256 = "abcdef"
                writes = $false
            }
        }
        latest_context_answer_readiness = [pscustomobject]@{
            complete = $true
            scenario_count = 4
            passed_count = 4
            failed_count = 0
            note = "offline"
        }
        latest_user_scenario_audit = [pscustomobject]@{
            complete = $false
            scenario_count = 7
            complete_count = 4
            failed_count = 3
            missing = @("selected_message_review")
        }
        goal_gap_audit = [pscustomobject]@{
            missing = @("FB2_AI_CENTER_TOKEN_live_permission_quality_refresh")
            refresh_gaps = @()
            deferred_by_user = @("ASR_TTS_final_evidence")
            blocked_by_external_secret = $true
            next_smallest_action = "set_FB2_AI_CENTER_TOKEN_then_run_DataOnlyAcceptance_PreflightOnly"
            current_flags = [pscustomobject]@{
                non_voice_ready = $false
                full_final_ready = $false
                token_present = $false
                voice_evidence_path_present = $false
            }
        }
        refresh_gaps = @("context_answer_readiness_validated_offline")
        live_preflight_request = [pscustomobject]@{
            commands = [pscustomobject]@{
                no_write_direct_read = "read"
                data_only_preflight = "preflight"
                visible_regression_requires_authorization = "visible"
            }
        }
    }

    $report = New-Fb2HandoffReport -Status $status -SourcePath "selftest-status.json"
    $md = ConvertTo-Fb2HandoffMarkdown -Report $report
    $failed = 0
    if ($report.schema -ne "fb2.main_project.handoff_report.v1") { $failed++ }
    if ($report.stage -ne "contract_direct_read_and_offline_context_ready_needs_live_refresh") { $failed++ }
    if (-not [bool]$report.verdict.public_contract_ready) { $failed++ }
    if (-not [bool]$report.verdict.contract_smoke_ready) { $failed++ }
    if (-not [bool]$report.contract_smoke.chat_bootstrap_ready) { $failed++ }
    if ([string]$report.contract_smoke.fb2_live_data_status -ne "skipped_missing_FB2_AI_CENTER_TOKEN") { $failed++ }
    if (-not [bool]$report.verdict.group_chat_direct_read_ready) { $failed++ }
    if ([bool]$report.verdict.user_scenario_audit_ready) { $failed++ }
    if ([bool]$report.evidence_policy.screenshots_accepted) { $failed++ }
    if (-not (@($report.evidence_policy.required_group_message_fields) -contains "text_sha256")) { $failed++ }
    if (-not ([string]$md -match "direct_api_read")) { $failed++ }
    if (-not ([string]$md -match "selected_message_review")) { $failed++ }
    Write-Output "== SelfTest Summary =="
    Write-Output "failed=$failed"
    if ($failed -gt 0) {
        exit 1
    }
}

if ($SelfTest) {
    Invoke-Fb2HandoffSelfTest
    exit 0
}

$root = Get-Fb2HandoffRepoRoot
if ([string]::IsNullOrWhiteSpace($StatusPath)) {
    $StatusPath = Join-Path $root "target\fb2-ai-center\status-current.json"
}

$status = Read-Fb2HandoffJson -Path $StatusPath
$report = New-Fb2HandoffReport -Status $status -SourcePath $StatusPath
$json = $report | ConvertTo-Json -Depth 10

if (-not [string]::IsNullOrWhiteSpace($OutputPath)) {
    $dir = Split-Path -Parent $OutputPath
    if ($dir -and -not (Test-Path -LiteralPath $dir)) {
        New-Item -ItemType Directory -Path $dir -Force | Out-Null
    }
    Set-Content -Path $OutputPath -Value $json -Encoding UTF8
}

if (-not [string]::IsNullOrWhiteSpace($MarkdownPath)) {
    $dir = Split-Path -Parent $MarkdownPath
    if ($dir -and -not (Test-Path -LiteralPath $dir)) {
        New-Item -ItemType Directory -Path $dir -Force | Out-Null
    }
    Set-Content -Path $MarkdownPath -Value (ConvertTo-Fb2HandoffMarkdown -Report $report) -Encoding UTF8
}

Write-Output $json
