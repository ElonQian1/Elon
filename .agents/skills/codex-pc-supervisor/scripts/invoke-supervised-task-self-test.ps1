function Invoke-SupervisionSelfTest {
    function ConvertFrom-Utf8Base64([string]$Value) {
        return [System.Text.Encoding]::UTF8.GetString([Convert]::FromBase64String($Value))
    }
    $testWorkspace = ConvertFrom-Utf8Base64 'Qzpc5LiA6b6Z6aG555uuXOS4reaWh+W3peS9nOWMug=='
    $testExecutionWorkspace = 'C:\conversation-worktrees\temporary-generation'
    $testPrompt = ConvertFrom-Utf8Base64 '5qOA5p+l55uR552j6ZO+6Lev77yM5L+d5oyB5Lit5paH6Lev5b6E5a6M5pW0'
    $testCriteria = @(
        ConvertFrom-Utf8Base64 '5o+Q5Lqk5LiO5qOA5p+l5L+d55WZ5Lit5paH'
        ConvertFrom-Utf8Base64 '562J5b6F5LiO6aqM5pS25pGY6KaB5peg5Lmx56CB'
    )
    $testSummary = ConvertFrom-Utf8Base64 '54us56uL5aSN5qC477ya6Lev5b6E44CB5o+Q56S65ZKM6aqM5pS25p2h5Lu25a6M5pW0'
    $testProject = ConvertFrom-Utf8Base64 '5Lit5paH6aG555uu'
    $testBody = New-SupervisedTaskBody $testProject $testWorkspace $testPrompt `
        'requirement' '' '' $testCriteria 'after_task_or_unblock'
    [byte[]]$requestBytes = Convert-ToUtf8JsonBytes $testBody
    $requestRoundTrip = Convert-JsonResponseBytes $requestBytes 'application/json'
    $ordinaryPostContext = New-OrdinaryPostRequestContext $testBody
    $ordinaryPostContextAgain = $ordinaryPostContext
    $ordinaryPostAttempts = New-Object System.Collections.Generic.List[object]
    $ordinaryPostRetry = Invoke-IdempotentNodePost `
        ([pscustomobject]@{ BaseUrl = 'http://127.0.0.1:7799' }) '/api/local-tasks' $testBody `
        $ordinaryPostContext `
        -RequestInvoker {
            param($Candidate, $EndpointPath, [byte[]]$Bytes, [string]$Key)
            $ordinaryPostAttempts.Add([pscustomobject]@{
                BaseUrl = $Candidate.BaseUrl; Path = $EndpointPath; Key = $Key; Bytes = $Bytes
            }) | Out-Null
            if ($ordinaryPostAttempts.Count -eq 1) {
                throw [System.Net.WebException]::new(
                    'fake timeout', [System.Net.WebExceptionStatus]::Timeout)
            }
            return [pscustomobject]@{ ok = $true; task_id = 'fake-retry-task' }
        } `
        -ConnectionResolver {
            [pscustomobject]@{ BaseUrl = 'http://127.0.0.1:7801' }
        }

    $parentJson = [ordered]@{
        record = [ordered]@{
            task_id = 'local-parent-task'
            project_id = $testProject
            workspace_path = $testExecutionWorkspace
            prompt = $testPrompt
            status = 'failed'
            finished_at_ms = 2
            workspace_status = [ordered]@{
                isolated = $true
                base_workspace_path = $testWorkspace
                active_workspace_path = ConvertFrom-Utf8Base64 'QzpcY29udmVyc2F0aW9uLXdvcmt0cmVlc1zkuK3mlofpobnnm65c5Lit5paH5Lya6K+d'
                branch = 'ai/session/中文项目/中文会话'
            }
        }
        supervision = [ordered]@{
            enabled = $true
            protocol = $script:SupervisionProtocol
            contract = [ordered]@{
                protocol = $script:SupervisionProtocol
                task_role = 'requirement'
                root_task_id = 'local-root-task'
            }
        }
        resume_workspace_status = [ordered]@{
            eligible = $true
            derivation = 'workspace_status'
            active_workspace_path = ConvertFrom-Utf8Base64 'QzpcY29udmVyc2F0aW9uLXdvcmt0cmVlc1zkuK3mlofpobnnm65c5Lit5paH5Lya6K+d'
            branch = 'ai/session/中文项目/中文会话'
            git_head = '0123456789abcdef0123456789abcdef01234567'
            occupied = $false
            requires_recreation = $false
        }
    }
    [byte[]]$responseBytes = Convert-ToUtf8JsonBytes $parentJson
    $decodedParent = Convert-JsonResponseBytes $responseBytes 'application/json; charset=utf-8'
    $invalidUtf8Rejected = $false
    try {
        $null = Convert-JsonResponseBytes ([byte[]](0xC3, 0x28)) 'application/json; charset=utf-8'
    } catch [System.Text.DecoderFallbackException] {
        $invalidUtf8Rejected = $true
    } catch {
        if ($_.Exception.Message -eq 'Node response is not valid in its declared/default UTF-8 encoding.') {
            $invalidUtf8Rejected = $true
        } else {
            throw
        }
    }
    $reviewCandidate = New-SupervisionReviewBody 'accepted' $testSummary @(
        ConvertFrom-Utf8Base64 '5ZCO57ut57un57ut6KeC5a+f5Lit5paH5pel5b+X'
    )
    $reviewCandidate.reviewed_by = 'legacy-caller'
    $reviewCandidate.review_source = 'legacy-helper'
    $reviewBody = Convert-ToPublicSupervisionReviewBody $reviewCandidate
    [byte[]]$reviewBytes = Convert-ToUtf8JsonBytes $reviewBody
    $reviewRoundTrip = Convert-JsonResponseBytes $reviewBytes 'application/json'
    $reviewPropertyNames = @($reviewRoundTrip.PSObject.Properties.Name)
    $improvementPrompt = ConvertFrom-Utf8Base64 '5L+u5aSN5Lit5paH57un5om/6Lev5b6E'
    $improvementBody = New-ImprovementTaskBody $decodedParent 'local-parent-task' $improvementPrompt `
        $testCriteria $true
    $resumeBody = New-ResumeTaskBody $decodedParent 'local-parent-task' $testCriteria 'after_task_or_unblock'
    $supersedeBody = New-SupersedeTaskBody $decodedParent 'local-parent-task' `
        'REVISED REQUIREMENT' $testCriteria 'user changed the requirement' 'after_task_or_unblock'
    $resumeParent = Convert-JsonResponseBytes (Convert-ToUtf8JsonBytes $parentJson) 'application/json'
    $resumeParent.supervision.contract.task_role = 'resume_original'
    $resumeParentBody = New-ResumeTaskBody $resumeParent 'local-parent-task' $testCriteria 'after_task_or_unblock'
    $rejectedParentRoles = @('missing', 'unknown', 'capability_repair', 'post_task_improvement')
    $rejectedParentRoleCount = 0
    foreach ($rejectedRole in $rejectedParentRoles) {
        $rejectedParent = Convert-JsonResponseBytes (Convert-ToUtf8JsonBytes $parentJson) 'application/json'
        if ($rejectedRole -eq 'missing') {
            $rejectedParent.supervision.contract.PSObject.Properties.Remove('task_role')
        } else {
            $rejectedParent.supervision.contract.task_role = $rejectedRole
        }
        try {
            $null = New-ResumeTaskBody $rejectedParent 'local-parent-task' $testCriteria 'after_task_or_unblock'
        } catch {
            if ($_.Exception.Message -eq 'Resume parent task_role must be requirement or resume_original.') {
                $rejectedParentRoleCount++
            } else {
                throw
            }
        }
    }
    $legacyParent = Convert-JsonResponseBytes (Convert-ToUtf8JsonBytes $parentJson) 'application/json'
    $legacyParent.record.workspace_status = $null
    $legacyParent | Add-Member -NotePropertyName resume_workspace_status -NotePropertyValue ([pscustomobject][ordered]@{
        eligible = $true
        derivation = 'legacy_started_cwd_git_registry'
        active_workspace_path = 'C:\conversation-worktrees\中文项目\中文会话'
        branch = 'ai/session/中文项目/中文会话'
        git_head = '0123456789abcdef0123456789abcdef01234567'
    }) -Force
    $legacyResumeBody = New-ResumeTaskBody $legacyParent 'local-parent-task' $testCriteria 'after_task_or_unblock'
    $recycledParent = Convert-JsonResponseBytes (Convert-ToUtf8JsonBytes $parentJson) 'application/json'
    $recycledParent.resume_workspace_status.derivation = 'platform_receipt_commit_rebuild_available'
    $recycledParent.resume_workspace_status.requires_recreation = $true
    $recycledResumeBody = New-ResumeTaskBody $recycledParent 'local-parent-task' $testCriteria 'after_task_or_unblock'
    $recoveryReadyParent = Convert-JsonResponseBytes (Convert-ToUtf8JsonBytes $parentJson) 'application/json'
    $recoveryReadyParent.resume_workspace_status.derivation = 'workspace_status_git_recovery_ready_legacy_branch_ref'
    $recoveryReadyParent.resume_workspace_status.requires_recreation = $true
    $recoveryReadyResumeBody = New-ResumeTaskBody $recoveryReadyParent 'local-parent-task' $testCriteria 'after_task_or_unblock'
    $inheritedParent = Convert-JsonResponseBytes (Convert-ToUtf8JsonBytes $parentJson) 'application/json'
    $inheritedParent.resume_workspace_status.derivation = 'inherited_workspace_status'
    $inheritedResumeBody = New-ResumeTaskBody $inheritedParent 'local-parent-task' $testCriteria 'after_task_or_unblock'
    $recordedHeadRecoveryParent = Convert-JsonResponseBytes (Convert-ToUtf8JsonBytes $parentJson) 'application/json'
    $recordedHeadRecoveryParent.resume_workspace_status.derivation = 'workspace_status_git_recovery_ready_recorded_head'
    $recordedHeadRecoveryParent.resume_workspace_status.requires_recreation = $true
    $recordedHeadRecoveryResumeBody = New-ResumeTaskBody $recordedHeadRecoveryParent 'local-parent-task' $testCriteria 'after_task_or_unblock'
    $occupiedRecoveryReadyParent = Convert-JsonResponseBytes (Convert-ToUtf8JsonBytes $recoveryReadyParent) 'application/json'
    $occupiedRecoveryReadyParent.resume_workspace_status.occupied = $true
    $occupiedRecoveryReadyRejected = $false
    try {
        $null = New-ResumeTaskBody $occupiedRecoveryReadyParent 'local-parent-task' $testCriteria 'after_task_or_unblock'
    } catch {
        $occupiedRecoveryReadyRejected = $true
    }
    $unsafeParent = [ordered]@{
        record = [ordered]@{
            task_id = 'local-running-task'
            project_id = $testProject
            workspace_path = $testWorkspace
            prompt = $testPrompt
            status = 'running'
            workspace_status = [ordered]@{ isolated = $false }
        }
        supervision = [ordered]@{ enabled = $false }
    }
    $unsafeResumeRejected = $false
    try {
        $null = New-ResumeTaskBody $unsafeParent 'local-running-task' $testCriteria 'after_task_or_unblock'
    } catch {
        $unsafeResumeRejected = $true
    }
    $testDetailPath = Get-TaskDetailPath 'local-test?id'
    $testEpochPath = Get-TaskDetailPath 'local-test?id' 25 42 'journal:a/b?c'
    $testProjectsPath = Get-CloudProjectsPath $true
    $testBindingBody = New-ProjectBindingBody 'elon-self' $testWorkspace
    $testGrantBody = New-FullAccessGrantBody 'elon-self' $testWorkspace
    $testGrantPathMatch =
        (Convert-ToComparableWorkspacePath $testWorkspace) -eq
        (Convert-ToComparableWorkspacePath ('\\?\' + $testWorkspace))
    $criteriaJson = ConvertFrom-Utf8Base64 'WyLmnaHku7bkuIAiLCLmnaHku7bkuowiXQ=='
    $jsonCriteria = @(Resolve-AcceptanceCriteria @() $criteriaJson '')
    $activeDetail = [ordered]@{
        record = [ordered]@{
            task_id = 'local-recovered-task'
            status = 'running'
            error = $null
            finished_at_ms = $null
        }
        runtime = [ordered]@{
            phase = 'verification'
            current_command = 'cargo test --bin elon-pc-node'
            last_progress = 1784361600000
            heartbeat = 1784361605000
            idle_duration = 5
            dispatch = [ordered]@{
                schema = 'elon.task_dispatch_progress.v1'
                stage = 'active'
                stages = @([ordered]@{
                    stage = 'build_admission_cache_telemetry'
                    duration_ms = 25
                    outcome = 'completed'
                })
            }
        }
        supervision = [ordered]@{ evidence = [ordered]@{
            event_count = 99
            tool_calls = 12
            tool_results = 11
            failed_tools = 1
            file_change_events = 4
            agent_messages = 2
            terminal_event_seen = $false
            changed_files = @('server/src/a.rs','server/src/b.rs')
            command_exit_codes = @([ordered]@{ command = 'cargo test'; exit_code = 0 })
            failure_summaries = @('old failure')
        } }
        events = @([ordered]@{
            seq = 42
            event = [ordered]@{ type = 'recovery_running'; phase = 'verification' }
        })
        last_event_seq = 42
        has_more = $false
    }
    $activeDetail = Convert-JsonResponseBytes (Convert-ToUtf8JsonBytes $activeDetail) 'application/json'
    $activeCompact = Convert-ToCompactTaskDetail $activeDetail
    $volatileDetail = Convert-JsonResponseBytes (Convert-ToUtf8JsonBytes $activeDetail) 'application/json'
    $volatileDetail.runtime.heartbeat = 1784361665000
    $volatileDetail.runtime.idle_duration = 65
    $volatileCompact = Convert-ToCompactTaskDetail $volatileDetail
    $unchangedCompact = Select-TaskDeltaChanges `
        (Convert-ToCompactTaskDetail $activeDetail) `
        $activeCompact.state_digest $activeCompact.evidence_digest
    $terminalCompact = Convert-ToCompactTaskDetail $activeDetail $null $true
    $unchangedTerminalCompact = Select-TaskDeltaChanges `
        $terminalCompact $terminalCompact.state_digest $terminalCompact.evidence_digest
    $dictionaryFieldAccess = (Get-ObjectField ([ordered]@{ state_digest = 'dictionary-digest' }) 'state_digest') -eq 'dictionary-digest'
    $deltaEvents = New-Object System.Collections.Generic.List[object]
    $deltaSeen = @{}
    $null = Merge-TaskDeltaEvents $deltaEvents $deltaSeen ([pscustomobject]@{
        cursor_epoch = 'epoch-a'; cursor_reset = $false
        events = @([pscustomobject]@{ seq = 1; event = [pscustomobject]@{ type = 'one' } })
    })
    $null = Merge-TaskDeltaEvents $deltaEvents $deltaSeen ([pscustomobject]@{
        cursor_epoch = 'epoch-a'; cursor_reset = $false
        events = @(
            [pscustomobject]@{ seq = 1; event = [pscustomobject]@{ type = 'duplicate' } },
            [pscustomobject]@{ seq = 2; event = [pscustomobject]@{ type = 'two' } }
        )
    })
    $deltaNoLoss = $deltaEvents.Count -eq 2 -and $deltaEvents[0].seq -eq 1 -and $deltaEvents[1].seq -eq 2
    $deltaReset = Merge-TaskDeltaEvents $deltaEvents $deltaSeen ([pscustomobject]@{
        cursor_epoch = 'epoch-b'; cursor_reset = $true
        events = @([pscustomobject]@{ seq = 1; event = [pscustomobject]@{ type = 'reset' } })
    })
    $emptyPageCursor = Resolve-MonotonicTaskCursor 42 0 $false 0
    $resetPageCursor = Resolve-MonotonicTaskCursor 42 0 $true 7
    $timeoutFailure = Get-WaitFailureCode ([System.Net.WebException]::new(
        'timeout', [System.Net.WebExceptionStatus]::Timeout))
    $unreachableFailure = Get-WaitFailureCode ([System.Net.WebException]::new(
        'connect', [System.Net.WebExceptionStatus]::ConnectFailure))
    $apiFailure = Get-WaitFailureCode ([System.Exception]::new(
        'Node API returned HTTP 500: structured failure'))
    $genericFailure = Get-WaitFailureCode ([System.Exception]::new('unexpected parse failure'))
    $noChangeOutcome = Resolve-WaitOutcome $false 'running' 0 $false $true
    $changedOutcome = Resolve-WaitOutcome $false 'running' 1 $false $false
    $invalidDeltaRejected = $false
    try {
        $null = Merge-TaskDeltaEvents $deltaEvents $deltaSeen ([pscustomobject]@{
            cursor_epoch = ''; cursor_reset = $false
            events = @([pscustomobject]@{ seq = 2; event = [pscustomobject]@{ type = 'invalid' } })
        })
    } catch {
        $invalidDeltaRejected = $true
    }
    $priorStateRoot = $env:ELON_DESKTOP_REVIEW_STATE_ROOT
    $priorInstallRoot = $env:ELON_DESKTOP_REVIEW_INSTALL_ROOT
    $desktopPathsRejected = $false
    $legacyDesktopCapabilityRejected = $false
    try {
        Assert-NodeSupervisionCapability ([pscustomobject]@{
            SupervisionProtocol = $script:SupervisionProtocol
            SupervisionCapabilities = @('desktop_review_ticket_v2', 'desktop_review_ticket_v1')
        }) $script:DesktopReviewCapability 'Desktop Review'
    } catch {
        $legacyDesktopCapabilityRejected = $true
    }
    try {
        $env:ELON_DESKTOP_REVIEW_STATE_ROOT = ''
        $env:ELON_DESKTOP_REVIEW_INSTALL_ROOT = ''
        $null = New-DesktopReviewTicket 'owner-self-test' 'local-self-test' 'POST' '/api/local-tasks/local-self-test/supervision/desktop-review' $reviewBytes
    } catch {
        $desktopPathsRejected = $_.Exception.Message -like 'desktop_review_paths_not_configured:*'
    } finally {
        $env:ELON_DESKTOP_REVIEW_STATE_ROOT = $priorStateRoot
        $env:ELON_DESKTOP_REVIEW_INSTALL_ROOT = $priorInstallRoot
    }
    $criteriaFile = Join-Path ([System.IO.Path]::GetTempPath()) "elon-supervision-criteria-$([guid]::NewGuid().ToString('N')).json"
    try {
        [System.IO.File]::WriteAllText(
            $criteriaFile,
            (ConvertFrom-Utf8Base64 'eyJhY2NlcHRhbmNlX2NyaXRlcmlhIjpbIuaWh+S7tuadoeS7tuS4gCIsIuaWh+S7tuadoeS7tuS6jCIsIuaWh+S7tuadoeS7tuS4iSJdfQ=='),
            $script:Utf8NoBom
        )
        $fileCriteria = @(Resolve-AcceptanceCriteria @() '' $criteriaFile)
    } finally {
        Remove-Item -LiteralPath $criteriaFile -Force -ErrorAction SilentlyContinue
    }
    $checks = [ordered]@{
        legacy_criteria = $testBody.supervision.acceptance_criteria.Count -eq 2
        task_role = $testBody.supervision.task_role -eq 'requirement'
        request_workspace = $requestRoundTrip.workspace_path -ceq $testWorkspace
        request_permission = $requestRoundTrip.runtime_permission -eq 'full_access'
        request_prompt = $requestRoundTrip.prompt -ceq $testPrompt
        request_criteria = $requestRoundTrip.supervision.acceptance_criteria[0] -ceq $testCriteria[0]
        ordinary_post_stable_idempotency =
            $ordinaryPostContext.IdempotencyKey -match '^desktop-[0-9a-f]{32}$' -and
            $ordinaryPostContextAgain.IdempotencyKey -ceq $ordinaryPostContext.IdempotencyKey -and
            [object]::ReferenceEquals($ordinaryPostContextAgain, $ordinaryPostContext) -and
            ([byte[]]$ordinaryPostContext.BodyBytes).Length -eq $requestBytes.Length
        ordinary_post_timeout_retry_reuses_binding = $ordinaryPostAttempts.Count -eq 2 -and
            $ordinaryPostAttempts[0].Key -ceq $ordinaryPostAttempts[1].Key -and
            $ordinaryPostAttempts[0].Key -ceq $ordinaryPostContext.IdempotencyKey -and
            $ordinaryPostAttempts[0].Path -ceq $ordinaryPostAttempts[1].Path -and
            $ordinaryPostAttempts[0].Bytes.Length -eq $ordinaryPostAttempts[1].Bytes.Length -and
            $ordinaryPostRetry.Connection.BaseUrl -eq 'http://127.0.0.1:7801'
        response_workspace = $decodedParent.record.workspace_path -ceq $testExecutionWorkspace
        invalid_utf8 = $invalidUtf8Rejected
        review_summary = $reviewRoundTrip.summary -ceq $testSummary
        review_public_dto = $reviewPropertyNames.Count -eq 3 -and
            $reviewPropertyNames -contains 'verdict' -and
            $reviewPropertyNames -contains 'summary' -and
            $reviewPropertyNames -contains 'improvements' -and
            $reviewPropertyNames -notcontains 'reviewed_by' -and
            $reviewPropertyNames -notcontains 'review_source'
        improve_workspace = $improvementBody.workspace_path -ceq $testWorkspace
        improve_role = $improvementBody.supervision.task_role -eq 'capability_repair'
        improve_parent = $improvementBody.supervision.parent_task_id -eq 'local-parent-task'
        improve_root = $improvementBody.supervision.root_task_id -eq 'local-root-task'
        resume_workspace = $resumeBody.workspace_path -ceq $testExecutionWorkspace
        resume_prompt = $resumeBody.prompt -eq 'Resolve elon.resume_context.v1 for parent_task_id=local-parent-task and root_task_id=local-root-task.' -and
            $resumeBody.prompt.IndexOf($testPrompt, [System.StringComparison]::Ordinal) -lt 0 -and
            $resumeBody.prompt.IndexOf('Resume the original task', [System.StringComparison]::Ordinal) -lt 0
        resume_role = $resumeBody.supervision.task_role -eq 'resume_original'
        resume_parent_role_requirement = $resumeBody.supervision.task_role -eq 'resume_original'
        resume_parent_role_resume_original = $resumeParentBody.supervision.task_role -eq 'resume_original'
        resume_parent_role_reject_matrix = $rejectedParentRoleCount -eq $rejectedParentRoles.Count
        resume_protocol = $resumeBody.supervision.protocol -eq $script:SupervisionProtocol
        resume_parent = $resumeBody.supervision.parent_task_id -eq 'local-parent-task'
        resume_root = $resumeBody.supervision.root_task_id -eq 'local-root-task'
        supersede_reuses_authorized_base = $supersedeBody.workspace_path -ceq $testWorkspace
        supersede_keeps_lineage = $supersedeBody.supervision.task_role -eq 'resume_original' -and
            $supersedeBody.supervision.parent_task_id -eq 'local-parent-task' -and
            $supersedeBody.supervision.root_task_id -eq 'local-root-task'
        supersede_records_explicit_revision = $supersedeBody.prompt -ceq 'REVISED REQUIREMENT' -and
            $supersedeBody.contract_revision.schema -eq 'elon.supervision.contract_revision.v1' -and
            $supersedeBody.contract_revision.reason -eq 'user changed the requirement'
        resume_legacy_started_cwd = $legacyResumeBody.supervision.task_role -eq 'resume_original'
        resume_receipt_rebuild = $recycledResumeBody.supervision.task_role -eq 'resume_original'
        resume_git_recovery_ready = $recoveryReadyResumeBody.supervision.task_role -eq 'resume_original'
        resume_inherited_workspace = $inheritedResumeBody.supervision.task_role -eq 'resume_original'
        resume_recorded_head_recovery = $recordedHeadRecoveryResumeBody.supervision.task_role -eq 'resume_original'
        resume_git_recovery_occupied_guard = $occupiedRecoveryReadyRejected
        resume_guard = $unsafeResumeRejected
        protocol = $script:SupervisionProtocol -eq 'elon.desktop_pc_supervision.v1'
        detail_path = $testDetailPath -eq '/api/local-tasks/local-test%3Fid?limit=200'
        expected_cursor_epoch_path = $testEpochPath -eq '/api/local-tasks/local-test%3Fid?since=42&limit=25&expected_cursor_epoch=journal%3Aa%2Fb%3Fc'
        desktop_review_paths_fail_closed = $desktopPathsRejected
        desktop_review_requires_v3_capability = $legacyDesktopCapabilityRejected -and
            $script:DesktopReviewCapability -eq 'desktop_review_ticket_v3'
        cloud_projects_path = $testProjectsPath -eq '/api/cloud-projects?include_system=true'
        project_binding_body = $testBindingBody.project_id -eq 'elon-self' -and
            $testBindingBody.workspace_path -ceq [System.IO.Path]::GetFullPath($testWorkspace) -and
            $null -eq (Get-ObjectField $testBindingBody 'token') -and
            $null -eq (Get-ObjectField $testBindingBody 'user_token')
        full_access_grant_body = $testGrantBody.project_id -eq 'elon-self' -and
            $testGrantBody.confirm_full_access -eq $true -and $testGrantPathMatch -and
            (Test-EquivalentProjectId 'elon-project' 'elon-self') -and
            $null -eq (Get-ObjectField $testGrantBody 'token') -and
            $null -eq (Get-ObjectField $testGrantBody 'user_token')
        inspect_wait_active = $activeCompact.record.status -eq 'running' -and
            $activeCompact.runtime.phase -eq 'verification' -and
            $activeCompact.runtime.dispatch.stage -eq 'active' -and
            $activeCompact.last_event_seq -eq 42 -and
            $activeCompact.events[0].type -eq 'recovery_running'
        compact_delta_omits_repeated_evidence_arrays = $null -eq $activeCompact.terminal_evidence -and
            $activeCompact.evidence_totals.event_count -eq 99 -and
            $activeCompact.evidence_digest -match '^[0-9a-f]{64}$' -and
            $activeCompact.state_digest -match '^[0-9a-f]{64}$'
        compact_delta_omits_unchanged_state_and_evidence = -not $unchangedCompact.state_changed -and
            -not $unchangedCompact.evidence_changed -and $null -eq $unchangedCompact.record -and
            $null -eq $unchangedCompact.runtime -and $null -eq $unchangedCompact.evidence_totals
        compact_delta_ignores_volatile_liveness = $volatileCompact.state_digest -eq $activeCompact.state_digest
        compact_delta_omits_unchanged_terminal_evidence = $null -eq $unchangedTerminalCompact.terminal_evidence
        compact_delta_dictionary_field_access = $dictionaryFieldAccess
        compact_delta_no_loss_or_duplicate = $deltaNoLoss -and $deltaReset -and
            $deltaEvents.Count -eq 1 -and $deltaEvents[0].event.type -eq 'reset'
        empty_page_cursor_monotonic = $emptyPageCursor -eq 42
        epoch_reset_uses_resume_cursor = $resetPageCursor -eq 7
        compact_wait_timeout_classified = $timeoutFailure -eq 'request_timeout'
        compact_wait_unreachable_classified = $unreachableFailure -eq 'node_unreachable'
        compact_wait_api_error_not_unreachable = $apiFailure -eq 'node_api_error' -and
            $genericFailure -eq 'request_failed'
        compact_wait_no_change_outcome = $noChangeOutcome -eq 'no_change_timeout' -and
            $changedOutcome -eq 'changed'
        compact_delta_invalid_epoch_rejected = $invalidDeltaRejected
        criteria_json = $jsonCriteria.Count -eq 2 -and
            $jsonCriteria[1] -ceq (ConvertFrom-Utf8Base64 '5p2h5Lu25LqM')
        criteria_file = $fileCriteria.Count -eq 3 -and
            $fileCriteria[2] -ceq (ConvertFrom-Utf8Base64 '5paH5Lu25p2h5Lu25LiJ')
    }
    $failedChecks = @($checks.Keys | Where-Object { -not $checks[$_] })
    if ($failedChecks.Count -gt 0) {
        throw "Supervised request construction self-test failed: $($failedChecks -join ', ')"
    }
    Convert-ToJsonResult ([ordered]@{
        ok = $true
        action = 'SelfTest'
        protocol = $script:SupervisionProtocol
        checks = @(
            'utf8_request_bytes', 'utf8_response_decode', 'invalid_utf8_rejected', 'non_ascii_workspace',
            'non_ascii_prompt', 'acceptance_criteria', 'ordinary_post_stable_idempotency',
            'ordinary_post_timeout_retry_reuses_binding',
            'review_summary', 'review_public_dto',
            'improve_inherited_path', 'resume_inherited_path', 'resume_parent_guard',
            'resume_parent_role_requirement', 'resume_parent_role_resume_original',
            'resume_parent_role_reject_matrix',
            'supersede_explicit_contract_revision',
            'resume_legacy_started_cwd', 'resume_receipt_rebuild', 'resume_git_recovery_ready',
            'resume_inherited_workspace', 'resume_recorded_head_recovery',
            'resume_git_recovery_occupied_guard', 'task_detail_path', 'cloud_projects_path',
            'project_binding_body_without_token',
            'authoritative_full_access_grant_without_token',
            'expected_cursor_epoch_handshake', 'desktop_review_paths_fail_closed',
            'desktop_review_requires_v3_capability',
            'inspect_wait_active_runtime', 'compact_delta_evidence_digest',
            'compact_delta_no_loss_or_duplicate', 'compact_delta_invalid_epoch_rejected',
            'compact_delta_omits_unchanged_state_and_evidence',
            'compact_delta_ignores_volatile_liveness', 'compact_delta_omits_unchanged_terminal_evidence',
            'compact_delta_dictionary_field_access',
            'compact_wait_timeout_classified', 'compact_wait_unreachable_classified',
            'compact_wait_api_error_not_unreachable', 'compact_wait_no_change_outcome',
            'criteria_json_array', 'criteria_utf8_file'
        )
    })
}
