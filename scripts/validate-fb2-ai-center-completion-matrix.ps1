#requires -Version 7.0

param(
    [string]$RefreshPath = "",
    [string]$OutputPath = "",
    [switch]$SelfTest
)

$ErrorActionPreference = "Stop"

function Get-Fb2MatrixRepoRoot {
    Split-Path -Parent $PSScriptRoot
}

function Resolve-Fb2MatrixPath {
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

function Get-Fb2MatrixProperty {
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

function Read-Fb2MatrixJson {
    param([string]$Path)

    if ([string]::IsNullOrWhiteSpace($Path) -or -not (Test-Path -LiteralPath $Path)) {
        throw "Refresh summary not found: $Path. Run scripts\fb2-ai-center-refresh-current-status.ps1 first."
    }
    Get-Content -LiteralPath $Path -Raw | ConvertFrom-Json
}

function Read-Fb2MatrixJsonOrNull {
    param([string]$Path)

    if ([string]::IsNullOrWhiteSpace($Path) -or -not (Test-Path -LiteralPath $Path)) {
        return $null
    }
    try {
        Get-Content -LiteralPath $Path -Raw | ConvertFrom-Json
    } catch {
        $null
    }
}

function Add-Fb2MatrixCheck {
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

function Test-Fb2MatrixSecretSafe {
    param([string]$Text)

    if ([string]::IsNullOrWhiteSpace($Text)) {
        return $true
    }
    if ($Text -match '(?i)FB2_AI_CENTER_TOKEN\s*=\s*["''][^<]') {
        return $false
    }
    if ($Text -match '(?i)-Fb2(AiCenter)?Token\s+(?!<FB2_AI_CENTER_TOKEN>)[^\s]+') {
        return $false
    }
    if ($Text -match '(?i)-Fb2Password\s+(?!<FB2_PASSWORD>)[^\s]+') {
        return $false
    }
    if ($Text -match '(?i)(bearer|token|password|secret)[=:]\s*(?!<)[A-Za-z0-9_\-\.]{12,}') {
        return $false
    }
    return $true
}

function Find-Fb2MatrixRequirement {
    param(
        [object[]]$Requirements,
        [string]$Id
    )

    @($Requirements | Where-Object { [string]$_.id -eq $Id } | Select-Object -First 1)
}

function Find-Fb2MatrixScenario {
    param(
        [object[]]$Scenarios,
        [string]$Field,
        [string]$Value
    )

    @($Scenarios | Where-Object { [string](Get-Fb2MatrixProperty $_ $Field "") -eq $Value } | Select-Object -First 1)
}

function Normalize-Fb2MatrixPath {
    param([string]$Path)

    if ([string]::IsNullOrWhiteSpace($Path)) {
        return ""
    }
    try {
        return [System.IO.Path]::GetFullPath($Path).TrimEnd('\', '/').ToLowerInvariant()
    } catch {
        return $Path.TrimEnd('\', '/').ToLowerInvariant()
    }
}

function Get-Fb2MatrixExpectedGroup {
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

function Get-Fb2MatrixExpectedOwner {
    param([string]$Group)

    switch ($Group) {
        "main_project_contract" { return "main_project" }
        "voice_deferred_by_user" { return "paused_by_user" }
        default { return "shared" }
    }
}

function New-Fb2MatrixValidation {
    param(
        [object]$Refresh,
        [string]$SourcePath
    )

    $checks = [System.Collections.ArrayList]::new()
    $matrix = Get-Fb2MatrixProperty $Refresh "completion_matrix"
    $totals = Get-Fb2MatrixProperty $matrix "totals"
    $gates = Get-Fb2MatrixProperty $matrix "gates"
    $groups = Get-Fb2MatrixProperty $matrix "groups"
    $requirements = @(Get-Fb2MatrixProperty $matrix "requirements" @())

    $requiredIds = @(
        "context_pack_contract",
        "main_project_contract_smoke",
        "domain_context_index_contract",
        "today_matches_analysis",
        "my_ticket_analysis",
        "platform_order_risk",
        "group_opinion_summary",
        "selected_message_review",
        "group_discussion_summary_post",
        "source_reference_audit",
        "permission_safety",
        "feedback_quality_loop",
        "direct_group_chat_read",
        "voice_final_evidence"
    )

    Add-Fb2MatrixCheck $checks "matrix schema" ([string](Get-Fb2MatrixProperty $matrix "schema" "") -eq "fb2.main_project.completion_matrix.v1")
    Add-Fb2MatrixCheck $checks "requirement count matches declared total" ([int](Get-Fb2MatrixProperty $totals "total" 0) -eq @($requirements).Count) ("declared=$([int](Get-Fb2MatrixProperty $totals 'total' 0)) actual=$(@($requirements).Count)")
    Add-Fb2MatrixCheck $checks "expected requirement count" (@($requirements).Count -eq @($requiredIds).Count) ("expected=$(@($requiredIds).Count) actual=$(@($requirements).Count)")

    $ids = @($requirements | ForEach-Object { [string]$_.id })
    $duplicateIds = @($ids | Group-Object | Where-Object { $_.Count -gt 1 } | ForEach-Object { $_.Name })
    Add-Fb2MatrixCheck $checks "no duplicate requirement ids" (@($duplicateIds).Count -eq 0) (@($duplicateIds) -join ",")

    foreach ($id in $requiredIds) {
        $item = @(Find-Fb2MatrixRequirement -Requirements $requirements -Id $id)
        Add-Fb2MatrixCheck $checks "has requirement $id" (@($item).Count -gt 0)
        if (@($item).Count -eq 0) {
            continue
        }
        $expectedGroup = Get-Fb2MatrixExpectedGroup -Id $id
        $expectedOwner = Get-Fb2MatrixExpectedOwner -Group $expectedGroup
        Add-Fb2MatrixCheck $checks "$id group" ([string](Get-Fb2MatrixProperty $item[0] "group" "") -eq $expectedGroup) ("expected=$expectedGroup actual=$([string](Get-Fb2MatrixProperty $item[0] 'group' ''))")
        Add-Fb2MatrixCheck $checks "$id owner" ([string](Get-Fb2MatrixProperty $item[0] "owner" "") -eq $expectedOwner) ("expected=$expectedOwner actual=$([string](Get-Fb2MatrixProperty $item[0] 'owner' ''))")
        Add-Fb2MatrixCheck $checks "$id status present" (-not [string]::IsNullOrWhiteSpace([string](Get-Fb2MatrixProperty $item[0] "status" "")))
        Add-Fb2MatrixCheck $checks "$id evidence secret safe" (Test-Fb2MatrixSecretSafe -Text ([string](Get-Fb2MatrixProperty $item[0] "evidence" "")))
        Add-Fb2MatrixCheck $checks "$id missing secret safe" (Test-Fb2MatrixSecretSafe -Text ([string](Get-Fb2MatrixProperty $item[0] "missing" "")))
    }

    $completeCount = @($requirements | Where-Object { [bool]$_.complete }).Count
    $deferredCount = @($requirements | Where-Object { [bool]$_.deferred }).Count
    $incompleteCount = @($requirements | Where-Object { -not [bool]$_.complete -and -not [bool]$_.deferred }).Count
    Add-Fb2MatrixCheck $checks "complete count matches" ([int](Get-Fb2MatrixProperty $totals "complete" -1) -eq $completeCount) ("declared=$([int](Get-Fb2MatrixProperty $totals 'complete' -1)) actual=$completeCount")
    Add-Fb2MatrixCheck $checks "deferred count matches" ([int](Get-Fb2MatrixProperty $totals "deferred" -1) -eq $deferredCount) ("declared=$([int](Get-Fb2MatrixProperty $totals 'deferred' -1)) actual=$deferredCount")
    Add-Fb2MatrixCheck $checks "incomplete count matches" ([int](Get-Fb2MatrixProperty $totals "incomplete" -1) -eq $incompleteCount) ("declared=$([int](Get-Fb2MatrixProperty $totals 'incomplete' -1)) actual=$incompleteCount")

    foreach ($groupName in @("main_project_contract", "user_scenarios", "permission_and_quality", "group_chat_direct_read", "voice_deferred_by_user", "other")) {
        $actual = @($requirements | Where-Object { [string]$_.group -eq $groupName }).Count
        $declared = [int](Get-Fb2MatrixProperty $groups $groupName -1)
        Add-Fb2MatrixCheck $checks "group count $groupName" ($declared -eq $actual) ("declared=$declared actual=$actual")
    }

    $nonVoiceIncomplete = @($requirements | Where-Object { [string]$_.id -ne "voice_final_evidence" -and -not [bool]$_.complete })
    $voice = @(Find-Fb2MatrixRequirement -Requirements $requirements -Id "voice_final_evidence")
    $voiceDeferred = (@($voice).Count -gt 0 -and [bool](Get-Fb2MatrixProperty $voice[0] "deferred" $false) -and -not [bool](Get-Fb2MatrixProperty $voice[0] "complete" $false))
    $dataGoalComplete = [bool](Get-Fb2MatrixProperty $gates "data_goal_complete" $false)
    $fullFinalComplete = [bool](Get-Fb2MatrixProperty $gates "full_final_complete" $false)
    $gateVoiceDeferred = [bool](Get-Fb2MatrixProperty $gates "voice_deferred_by_user" $false)
    $tokenPresent = [bool](Get-Fb2MatrixProperty $gates "token_present" $false)
    $nextAction = [string](Get-Fb2MatrixProperty $gates "next_minimum_action" "")
    $protectedLivePreflightSatisfied = [bool](Get-Fb2MatrixProperty $Refresh "protected_live_preflight_satisfied" $false)
    $expectedTokenlessReadyNextAction = if ($protectedLivePreflightSatisfied) {
        "keep_non_voice_regression_green_resume_ASR_TTS_only_when_user_unpauses"
    } else {
        "set_FB2_AI_CENTER_TOKEN_then_run_DataOnlyAcceptance_PreflightOnly"
    }

    Add-Fb2MatrixCheck $checks "data goal gate matches non-voice requirements" ($dataGoalComplete -eq (@($nonVoiceIncomplete).Count -eq 0)) ("non_voice_incomplete=$(@($nonVoiceIncomplete | ForEach-Object { $_.id }) -join ',')")
    Add-Fb2MatrixCheck $checks "voice deferred gate matches voice requirement" ($gateVoiceDeferred -eq $voiceDeferred)
    Add-Fb2MatrixCheck $checks "full final implies data goal complete" ((-not $fullFinalComplete) -or $dataGoalComplete)
    Add-Fb2MatrixCheck $checks "full final implies voice complete" ((-not $fullFinalComplete) -or (@($voice).Count -gt 0 -and [bool](Get-Fb2MatrixProperty $voice[0] "complete" $false) -and -not [bool](Get-Fb2MatrixProperty $voice[0] "deferred" $false)))
    Add-Fb2MatrixCheck $checks "tokenless non-voice ready next action matches protected preflight state" (($tokenPresent -or -not $dataGoalComplete -or $fullFinalComplete) -or $nextAction -eq $expectedTokenlessReadyNextAction) ("expected=$expectedTokenlessReadyNextAction next=$nextAction")
    Add-Fb2MatrixCheck $checks "next action secret safe" (Test-Fb2MatrixSecretSafe -Text $nextAction)

    $refreshMissing = @((Get-Fb2MatrixProperty $Refresh "missing_non_voice_requirements" @()))
    $refreshDeferred = @((Get-Fb2MatrixProperty $Refresh "deferred_requirements" @()))
    Add-Fb2MatrixCheck $checks "refresh missing matches matrix non-voice incomplete" ((@($refreshMissing).Count -eq @($nonVoiceIncomplete).Count) -and -not (@($nonVoiceIncomplete | Where-Object { -not (@($refreshMissing) -contains [string]$_.id) }))) ("refresh=$(@($refreshMissing) -join ',') matrix=$(@($nonVoiceIncomplete | ForEach-Object { $_.id }) -join ',')")
    Add-Fb2MatrixCheck $checks "refresh deferred includes voice if deferred" ((-not $voiceDeferred) -or (@($refreshDeferred) -contains "voice_final_evidence"))

    $files = Get-Fb2MatrixProperty $Refresh "files"
    $statusPath = [string](Get-Fb2MatrixProperty $files "status" "")
    $exportedSamplePath = [string](Get-Fb2MatrixProperty $files "exported_context_pack_sample_set_validation" "")
    $statusSummary = Read-Fb2MatrixJsonOrNull -Path $statusPath
    $exportedSampleSet = Read-Fb2MatrixJsonOrNull -Path $exportedSamplePath
    if ($null -ne $statusSummary -and $null -ne $exportedSampleSet) {
        $selectedSamplePath = [string](Get-Fb2MatrixProperty (Get-Fb2MatrixProperty $statusSummary "latest_context_pack_sample_set") "path" "")
        Add-Fb2MatrixCheck $checks "exported sample set selected by status" `
            ((Normalize-Fb2MatrixPath -Path $selectedSamplePath) -eq (Normalize-Fb2MatrixPath -Path $exportedSamplePath)) `
            ("selected=$selectedSamplePath exported=$exportedSamplePath")

        $scenarioMap = @(
            [ordered]@{ sample = "today_matches_context_pack"; answer = "today_matches_analysis" },
            [ordered]@{ sample = "my_ticket_context_pack"; answer = "my_ticket_analysis" },
            [ordered]@{ sample = "platform_order_context_pack"; answer = "platform_order_risk" },
            [ordered]@{ sample = "group_opinion_context_pack"; answer = "group_opinion_summary" }
        )
        $selectedScenarios = @((Get-Fb2MatrixProperty (Get-Fb2MatrixProperty $statusSummary "latest_context_pack_sample_set") "scenarios" @()))
        $answerScenarios = @((Get-Fb2MatrixProperty (Get-Fb2MatrixProperty $statusSummary "latest_context_answer_readiness") "scenarios" @()))
        foreach ($mapping in $scenarioMap) {
            $sampleId = [string]$mapping.sample
            $answerId = [string]$mapping.answer
            $exportedScenario = @(Find-Fb2MatrixScenario -Scenarios @($exportedSampleSet.scenarios) -Field "scenario" -Value $sampleId)
            $selectedScenario = @(Find-Fb2MatrixScenario -Scenarios $selectedScenarios -Field "scenario" -Value $sampleId)
            $answerScenario = @(Find-Fb2MatrixScenario -Scenarios $answerScenarios -Field "id" -Value $answerId)
            $requirement = @(Find-Fb2MatrixRequirement -Requirements $requirements -Id $answerId)
            $auditId = if (@($exportedScenario).Count -gt 0) { [string](Get-Fb2MatrixProperty $exportedScenario[0] "context_audit_id" "") } else { "" }
            $sha = if (@($exportedScenario).Count -gt 0) { [string](Get-Fb2MatrixProperty $exportedScenario[0] "context_pack_sha256" "") } else { "" }
            $evidence = if (@($requirement).Count -gt 0) { [string](Get-Fb2MatrixProperty $requirement[0] "evidence" "") } else { "" }
            Add-Fb2MatrixCheck $checks "exported scenario present $sampleId" (-not [string]::IsNullOrWhiteSpace($auditId))
            Add-Fb2MatrixCheck $checks "selected sample audit matches exported $sampleId" `
                (@($selectedScenario).Count -gt 0 -and [string](Get-Fb2MatrixProperty $selectedScenario[0] "context_audit_id" "") -eq $auditId)
            Add-Fb2MatrixCheck $checks "answer readiness audit matches exported $answerId" `
                (@($answerScenario).Count -gt 0 -and [string](Get-Fb2MatrixProperty $answerScenario[0] "context_audit_id" "") -eq $auditId -and [string](Get-Fb2MatrixProperty $answerScenario[0] "context_pack_sha256" "") -eq $sha)
            Add-Fb2MatrixCheck $checks "completion matrix evidence matches exported $answerId" `
                ((-not [string]::IsNullOrWhiteSpace($auditId)) -and $evidence.Contains($auditId) -and ([string]::IsNullOrWhiteSpace($sha) -or $evidence.Contains($sha)))
        }
    }

    $failed = @($checks | Where-Object { -not [bool]$_.passed })
    [ordered]@{
        schema = "fb2.main_project.completion_matrix_validation.v1"
        source_refresh = $SourcePath
        success = (@($failed).Count -eq 0)
        check_count = @($checks).Count
        failed_count = @($failed).Count
        failed = @($failed)
        checks = @($checks)
    }
}

function New-Fb2MatrixRequirementFixture {
    param(
        [string]$Id,
        [bool]$Complete,
        [bool]$Deferred = $false
    )

    $group = Get-Fb2MatrixExpectedGroup -Id $Id
    [ordered]@{
        id = $Id
        group = $group
        owner = Get-Fb2MatrixExpectedOwner -Group $group
        title = "fixture $Id"
        status = if ($Deferred) { "deferred" } elseif ($Complete) { "complete" } else { "missing" }
        complete = $Complete
        deferred = $Deferred
        evidence = if ($Complete) { "fixture evidence" } else { "" }
        missing = if ($Complete) { "" } elseif ($Deferred) { "ASR/TTS is intentionally deferred by user" } else { "missing fixture evidence" }
    }
}

function New-Fb2MatrixFixtureRefresh {
    $ids = @(
        "context_pack_contract",
        "main_project_contract_smoke",
        "domain_context_index_contract",
        "today_matches_analysis",
        "my_ticket_analysis",
        "platform_order_risk",
        "group_opinion_summary",
        "selected_message_review",
        "group_discussion_summary_post",
        "source_reference_audit",
        "permission_safety",
        "feedback_quality_loop",
        "direct_group_chat_read"
    )
    $requirements = @($ids | ForEach-Object { New-Fb2MatrixRequirementFixture -Id $_ -Complete $true })
    $requirements += New-Fb2MatrixRequirementFixture -Id "voice_final_evidence" -Complete $false -Deferred $true
    [pscustomobject]@{
        completion_matrix = [ordered]@{
            schema = "fb2.main_project.completion_matrix.v1"
            totals = [ordered]@{ total = 14; complete = 13; deferred = 1; incomplete = 0 }
            gates = [ordered]@{
                data_goal_complete = $true
                full_final_complete = $false
                token_present = $false
                voice_deferred_by_user = $true
                next_minimum_action = "set_FB2_AI_CENTER_TOKEN_then_run_DataOnlyAcceptance_PreflightOnly"
            }
            groups = [ordered]@{
                main_project_contract = 3
                user_scenarios = 7
                permission_and_quality = 2
                group_chat_direct_read = 1
                voice_deferred_by_user = 1
                other = 0
            }
            requirements = $requirements
        }
        missing_non_voice_requirements = @()
        deferred_requirements = @("voice_final_evidence")
    }
}

function Invoke-Fb2MatrixSelfTest {
    $tempRoot = Join-Path ([System.IO.Path]::GetTempPath()) ("fb2-completion-matrix-selftest-" + [guid]::NewGuid().ToString("N"))
    try {
        New-Item -ItemType Directory -Force -Path $tempRoot | Out-Null
        $failed = 0
        $valid = New-Fb2MatrixFixtureRefresh
        $validResult = New-Fb2MatrixValidation -Refresh $valid -SourcePath "selftest-valid.json"
        if (-not [bool]$validResult.success) {
            $validResult | ConvertTo-Json -Depth 8
            $failed++
        }

        $scenarioAuditMap = @(
            [ordered]@{ sample = "today_matches_context_pack"; answer = "today_matches_analysis"; audit = "audit-live-today"; sha = "sha-live-today" },
            [ordered]@{ sample = "my_ticket_context_pack"; answer = "my_ticket_analysis"; audit = "audit-live-ticket"; sha = "sha-live-ticket" },
            [ordered]@{ sample = "platform_order_context_pack"; answer = "platform_order_risk"; audit = "audit-live-platform"; sha = "sha-live-platform" },
            [ordered]@{ sample = "group_opinion_context_pack"; answer = "group_opinion_summary"; audit = "audit-live-opinion"; sha = "sha-live-opinion" }
        )
        $exportedPath = Join-Path $tempRoot "fb2-repo-context-pack-samples-validation-current.json"
        $statusPath = Join-Path $tempRoot "status-current.json"
        $sampleScenarios = @(
            foreach ($item in $scenarioAuditMap) {
                [ordered]@{
                    scenario = [string]$item.sample
                    passed = $true
                    context_audit_id = [string]$item.audit
                    citation_source_count = 10
                    source_kinds = @("context_audit")
                    context_pack_sha256 = [string]$item.sha
                }
            }
        )
        [ordered]@{
            schema = "fb2.main_project.context_pack_sample_set_validation.v1"
            complete = $true
            scenario_count = 4
            passed_count = 4
            failed_count = 0
            scenarios = $sampleScenarios
        } | ConvertTo-Json -Depth 8 | Set-Content -LiteralPath $exportedPath -Encoding UTF8
        [ordered]@{
            latest_context_pack_sample_set = [ordered]@{
                path = $exportedPath
                complete = $true
                scenarios = $sampleScenarios
            }
            latest_context_answer_readiness = [ordered]@{
                complete = $true
                scenarios = @(
                    foreach ($item in $scenarioAuditMap) {
                        [ordered]@{
                            id = [string]$item.answer
                            context_audit_id = [string]$item.audit
                            context_pack_sha256 = [string]$item.sha
                        }
                    }
                )
            }
        } | ConvertTo-Json -Depth 8 | Set-Content -LiteralPath $statusPath -Encoding UTF8
        $consistent = New-Fb2MatrixFixtureRefresh
        Add-Member -InputObject $consistent -NotePropertyName "files" -NotePropertyValue ([ordered]@{
            status = $statusPath
            exported_context_pack_sample_set_validation = $exportedPath
        })
        foreach ($item in $scenarioAuditMap) {
            $requirement = @(Find-Fb2MatrixRequirement -Requirements @($consistent.completion_matrix.requirements) -Id ([string]$item.answer))
            $requirement[0].evidence = "scenario=$($item.answer) context_audit_id=$($item.audit) context_pack_sha256=$($item.sha)"
        }
        $consistentResult = New-Fb2MatrixValidation -Refresh $consistent -SourcePath "selftest-consistent-exported.json"
        if (-not [bool]$consistentResult.success) {
            $consistentResult | ConvertTo-Json -Depth 8
            $failed++
        }
        $staleMatrix = $consistent | ConvertTo-Json -Depth 12 | ConvertFrom-Json
        $staleRequirement = @(Find-Fb2MatrixRequirement -Requirements @($staleMatrix.completion_matrix.requirements) -Id "today_matches_analysis")
        $staleRequirement[0].evidence = "scenario=today_matches_analysis context_audit_id=old-audit context_pack_sha256=old-sha"
        $staleMatrixResult = New-Fb2MatrixValidation -Refresh $staleMatrix -SourcePath "selftest-stale-matrix.json"
        if ([bool]$staleMatrixResult.success) { $failed++ }

        $missingRequirement = $valid | ConvertTo-Json -Depth 12 | ConvertFrom-Json
        $missingRequirement.completion_matrix.requirements = @($missingRequirement.completion_matrix.requirements | Where-Object { [string]$_.id -ne "my_ticket_analysis" })
        $missingRequirement.completion_matrix.totals.total = 13
        $missingRequirement.completion_matrix.totals.complete = 12
        $missingRequirement.completion_matrix.groups.user_scenarios = 6
        $missingRequirementResult = New-Fb2MatrixValidation -Refresh $missingRequirement -SourcePath "selftest-missing-requirement.json"
        if ([bool]$missingRequirementResult.success) { $failed++ }

        $badTotals = $valid | ConvertTo-Json -Depth 12 | ConvertFrom-Json
        $badTotals.completion_matrix.totals.complete = 14
        $badTotalsResult = New-Fb2MatrixValidation -Refresh $badTotals -SourcePath "selftest-bad-totals.json"
        if ([bool]$badTotalsResult.success) { $failed++ }

        $badDataGoal = $valid | ConvertTo-Json -Depth 12 | ConvertFrom-Json
        $ticket = @(Find-Fb2MatrixRequirement -Requirements @($badDataGoal.completion_matrix.requirements) -Id "my_ticket_analysis")
        $ticket[0].complete = $false
        $ticket[0].status = "missing"
        $ticket[0].missing = "missing user orders"
        $badDataGoal.completion_matrix.totals.complete = 12
        $badDataGoal.completion_matrix.totals.incomplete = 1
        $badDataGoal.missing_non_voice_requirements = @("my_ticket_analysis")
        $badDataGoalResult = New-Fb2MatrixValidation -Refresh $badDataGoal -SourcePath "selftest-bad-data-goal.json"
        if ([bool]$badDataGoalResult.success) { $failed++ }

        $badFullFinal = $valid | ConvertTo-Json -Depth 12 | ConvertFrom-Json
        $badFullFinal.completion_matrix.gates.full_final_complete = $true
        $badFullFinalResult = New-Fb2MatrixValidation -Refresh $badFullFinal -SourcePath "selftest-bad-full-final.json"
        if ([bool]$badFullFinalResult.success) { $failed++ }

        $leaky = $valid | ConvertTo-Json -Depth 12 | ConvertFrom-Json
        $leaky.completion_matrix.requirements[0].evidence = "token=real-secret-token-1234567890"
        $leakyResult = New-Fb2MatrixValidation -Refresh $leaky -SourcePath "selftest-leaky.json"
        if ([bool]$leakyResult.success) { $failed++ }

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
    Invoke-Fb2MatrixSelfTest
    exit 0
}

$root = Get-Fb2MatrixRepoRoot
if ([string]::IsNullOrWhiteSpace($RefreshPath)) {
    $RefreshPath = Join-Path $root "target\fb2-ai-center\status-refresh-current.json"
} else {
    $RefreshPath = Resolve-Fb2MatrixPath -Path $RefreshPath -Root $root
}
if ([string]::IsNullOrWhiteSpace($OutputPath)) {
    $OutputPath = Join-Path $root "target\fb2-ai-center\completion-matrix-validation-current.json"
} else {
    $OutputPath = Resolve-Fb2MatrixPath -Path $OutputPath -Root $root
}

$parent = Split-Path -Parent $OutputPath
if (-not [string]::IsNullOrWhiteSpace($parent)) {
    New-Item -ItemType Directory -Force -Path $parent | Out-Null
}

$refresh = Read-Fb2MatrixJson -Path $RefreshPath
$result = New-Fb2MatrixValidation -Refresh $refresh -SourcePath $RefreshPath
$json = $result | ConvertTo-Json -Depth 8
Set-Content -LiteralPath $OutputPath -Value $json -Encoding UTF8
$json

if (-not [bool]$result.success) {
    exit 1
}
