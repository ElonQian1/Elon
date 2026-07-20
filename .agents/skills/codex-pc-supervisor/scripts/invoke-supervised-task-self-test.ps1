function Invoke-SupervisionSelfTest {
    function ConvertFrom-Utf8Base64([string]$Value) {
        return [System.Text.Encoding]::UTF8.GetString([Convert]::FromBase64String($Value))
    }
    $testWorkspace = ConvertFrom-Utf8Base64 'Qzpc5LiA6b6Z6aG555uuXOS4reaWh+W3peS9nOWMug=='
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

    $parentJson = [ordered]@{
        record = [ordered]@{
            task_id = 'local-parent-task'
            project_id = $testProject
            workspace_path = $testWorkspace
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
    $reviewBody = New-SupervisionReviewBody 'accepted' $testSummary @(
        ConvertFrom-Utf8Base64 '5ZCO57ut57un57ut6KeC5a+f5Lit5paH5pel5b+X'
    )
    [byte[]]$reviewBytes = Convert-ToUtf8JsonBytes $reviewBody
    $reviewRoundTrip = Convert-JsonResponseBytes $reviewBytes 'application/json'
    $improvementPrompt = ConvertFrom-Utf8Base64 '5L+u5aSN5Lit5paH57un5om/6Lev5b6E'
    $improvementBody = New-ImprovementTaskBody $decodedParent 'local-parent-task' $improvementPrompt `
        $testCriteria $true
    $resumeBody = New-ResumeTaskBody $decodedParent 'local-parent-task' $testCriteria 'after_task_or_unblock'
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
        }
        supervision = [ordered]@{ evidence = [ordered]@{ terminal_event_seen = $false } }
        events = @([ordered]@{
            seq = 42
            event = [ordered]@{ type = 'recovery_running'; phase = 'verification' }
        })
        last_event_seq = 42
        has_more = $false
    }
    $activeDetail = Convert-JsonResponseBytes (Convert-ToUtf8JsonBytes $activeDetail) 'application/json'
    $activeCompact = Convert-ToCompactTaskDetail $activeDetail
    $priorDesktopCredential = [Environment]::GetEnvironmentVariable('ELON_DESKTOP_REVIEW_CREDENTIAL')
    try {
        $env:ELON_DESKTOP_REVIEW_CREDENTIAL = 'desktop-self-test-credential-at-least-32-bytes'
        $desktopTicket = New-DesktopReviewTicket 'owner-self-test' 'local-self-test'
    } finally {
        [Environment]::SetEnvironmentVariable('ELON_DESKTOP_REVIEW_CREDENTIAL', $priorDesktopCredential)
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
        request_prompt = $requestRoundTrip.prompt -ceq $testPrompt
        request_criteria = $requestRoundTrip.supervision.acceptance_criteria[0] -ceq $testCriteria[0]
        response_workspace = $decodedParent.record.workspace_path -ceq $testWorkspace
        invalid_utf8 = $invalidUtf8Rejected
        review_summary = $reviewRoundTrip.summary -ceq $testSummary
        improve_workspace = $improvementBody.workspace_path -ceq $testWorkspace
        improve_role = $improvementBody.supervision.task_role -eq 'capability_repair'
        improve_parent = $improvementBody.supervision.parent_task_id -eq 'local-parent-task'
        improve_root = $improvementBody.supervision.root_task_id -eq 'local-root-task'
        resume_workspace = $resumeBody.workspace_path -ceq $testWorkspace
        resume_prompt = $resumeBody.prompt.IndexOf($testPrompt, [System.StringComparison]::Ordinal) -ge 0
        resume_role = $resumeBody.supervision.task_role -eq 'resume_original'
        resume_protocol = $resumeBody.supervision.protocol -eq $script:SupervisionProtocol
        resume_parent = $resumeBody.supervision.parent_task_id -eq 'local-parent-task'
        resume_root = $resumeBody.supervision.root_task_id -eq 'local-root-task'
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
        desktop_review_ticket = $desktopTicket -match '^v1\.[0-9]+\.[0-9a-f]{32}\.[0-9a-f]{64}$'
        cloud_projects_path = $testProjectsPath -eq '/api/cloud-projects?include_system=true'
        inspect_wait_active = $activeCompact.record.status -eq 'running' -and
            $activeCompact.runtime.phase -eq 'verification' -and
            $activeCompact.last_event_seq -eq 42 -and
            $activeCompact.events[0].type -eq 'recovery_running'
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
            'non_ascii_prompt', 'acceptance_criteria', 'review_summary',
            'improve_inherited_path', 'resume_inherited_path', 'resume_parent_guard',
            'resume_legacy_started_cwd', 'resume_receipt_rebuild', 'resume_git_recovery_ready',
            'resume_inherited_workspace', 'resume_recorded_head_recovery',
            'resume_git_recovery_occupied_guard', 'task_detail_path', 'cloud_projects_path',
            'expected_cursor_epoch_handshake', 'desktop_review_short_lived_ticket',
            'inspect_wait_active_runtime', 'criteria_json_array', 'criteria_utf8_file'
        )
    })
}
