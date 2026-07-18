[CmdletBinding()]
param(
    [Parameter(Mandatory = $true)]
    [ValidateSet('Probe', 'Submit', 'Inspect', 'Wait', 'Review', 'Improve', 'Resume', 'SelfTest')]
    [string]$Action,
    [string]$ProjectId = 'elon-project',
    [string]$WorkspacePath,
    [string]$Prompt,
    [string[]]$AcceptanceCriteria = @(),
    [string]$TaskId,
    [ValidateSet('observing', 'accepted', 'needs_follow_up', 'blocked_capability', 'rejected')]
    [string]$Verdict = 'observing',
    [string]$Summary = '',
    [string[]]$Improvements = @(),
    [ValidateSet('requirement', 'capability_repair', 'resume_original', 'post_task_improvement')]
    [string]$TaskRole = 'requirement',
    [ValidateSet('after_task_or_unblock', 'after_task_only', 'observe_only')]
    [string]$ImprovementPolicy = 'after_task_or_unblock',
    [string]$ParentTaskId,
    [string]$RootTaskId,
    [switch]$BlockingImprovement,
    [ValidateRange(1, 55)]
    [int]$WaitSeconds = 55
)

Set-StrictMode -Version 2.0
$ErrorActionPreference = 'Stop'
$script:SupervisionProtocol = 'elon.desktop_pc_supervision.v1'
$script:Utf8NoBom = [System.Text.UTF8Encoding]::new($false)
$script:Utf8NoBomStrict = [System.Text.UTF8Encoding]::new($false, $true)
$OutputEncoding = $script:Utf8NoBom
try { [Console]::OutputEncoding = $script:Utf8NoBom } catch {}

function Get-ObjectField {
    param([object]$InputObject, [string]$Name)
    if ($null -eq $InputObject) { return $null }
    $property = $InputObject.PSObject.Properties[$Name]
    if ($null -eq $property) { return $null }
    return $property.Value
}

function Convert-ToJsonResult {
    param([System.Collections.IDictionary]$Value)
    $Value | ConvertTo-Json -Depth 20 -Compress
}

function Convert-ToUtf8JsonBytes {
    param([object]$Value)
    $json = $Value | ConvertTo-Json -Depth 20 -Compress
    return ,([byte[]]$script:Utf8NoBomStrict.GetBytes($json))
}

function Convert-ResponseBytesToText {
    param(
        [byte[]]$Bytes,
        [string]$ContentType = ''
    )
    if ($null -eq $Bytes -or $Bytes.Length -eq 0) { return '' }

    $encoding = $null
    $offset = 0
    if ($Bytes.Length -ge 3 -and $Bytes[0] -eq 0xEF -and $Bytes[1] -eq 0xBB -and $Bytes[2] -eq 0xBF) {
        $encoding = $script:Utf8NoBomStrict
        $offset = 3
    } elseif ($Bytes.Length -ge 2 -and $Bytes[0] -eq 0xFF -and $Bytes[1] -eq 0xFE) {
        $encoding = [System.Text.UnicodeEncoding]::new($false, $true, $true)
        $offset = 2
    } elseif ($Bytes.Length -ge 2 -and $Bytes[0] -eq 0xFE -and $Bytes[1] -eq 0xFF) {
        $encoding = [System.Text.UnicodeEncoding]::new($true, $true, $true)
        $offset = 2
    } elseif ($ContentType -match '(?i)charset\s*=\s*["'']?([^;\s"'']+)') {
        try {
            $declaredEncoding = [System.Text.Encoding]::GetEncoding($Matches[1])
            $encoding = $declaredEncoding.Clone()
            $encoding.DecoderFallback = [System.Text.DecoderFallback]::ExceptionFallback
        } catch {
            throw "Node response declares an unsupported charset: $($Matches[1])"
        }
    } else {
        # RFC 8259 JSON exchanged between systems is UTF-8. Windows PowerShell
        # 5.1 otherwise falls back to a legacy code page when charset is absent.
        $encoding = $script:Utf8NoBomStrict
    }

    try {
        return $encoding.GetString($Bytes, $offset, $Bytes.Length - $offset)
    } catch [System.Text.DecoderFallbackException] {
        throw 'Node response is not valid in its declared/default UTF-8 encoding.'
    }
}

function Read-ResponseBytes {
    param([System.Net.WebResponse]$Response)
    $stream = $Response.GetResponseStream()
    if ($null -eq $stream) { return ,([byte[]]@()) }
    $buffer = New-Object System.IO.MemoryStream
    try {
        $stream.CopyTo($buffer)
        return ,([byte[]]$buffer.ToArray())
    } finally {
        $stream.Dispose()
        $buffer.Dispose()
    }
}

function Convert-JsonResponseBytes {
    param(
        [byte[]]$Bytes,
        [string]$ContentType = ''
    )
    $text = Convert-ResponseBytesToText $Bytes $ContentType
    if ([string]::IsNullOrWhiteSpace($text)) { return $null }
    return $text | ConvertFrom-Json
}

function Invoke-Utf8JsonRequest {
    param(
        [string]$Method,
        [string]$Uri,
        [System.Collections.IDictionary]$Headers,
        [object]$Body = $null,
        [int]$TimeoutSec = 15
    )
    $request = [System.Net.HttpWebRequest]::Create($Uri)
    $request.Method = $Method.ToUpperInvariant()
    $request.Timeout = $TimeoutSec * 1000
    $request.ReadWriteTimeout = $TimeoutSec * 1000
    $request.AutomaticDecompression = [System.Net.DecompressionMethods]::GZip -bor `
        [System.Net.DecompressionMethods]::Deflate
    foreach ($name in $Headers.Keys) {
        $request.Headers[[string]$name] = [string]$Headers[$name]
    }
    if ($null -ne $Body) {
        [byte[]]$requestBytes = Convert-ToUtf8JsonBytes $Body
        $request.ContentType = 'application/json; charset=utf-8'
        $request.ContentLength = $requestBytes.Length
        $requestStream = $request.GetRequestStream()
        try {
            $requestStream.Write($requestBytes, 0, $requestBytes.Length)
        } finally {
            $requestStream.Dispose()
        }
    }

    $response = $null
    try {
        $response = $request.GetResponse()
        [byte[]]$responseBytes = Read-ResponseBytes $response
        return Convert-JsonResponseBytes $responseBytes ([string]$response.ContentType)
    } catch [System.Net.WebException] {
        $errorResponse = $_.Exception.Response
        if ($null -ne $errorResponse) {
            try {
                [byte[]]$errorBytes = Read-ResponseBytes $errorResponse
                $errorText = Convert-ResponseBytesToText $errorBytes ([string]$errorResponse.ContentType)
                $statusCode = [int]([System.Net.HttpWebResponse]$errorResponse).StatusCode
                throw "Node API returned HTTP ${statusCode}: $errorText"
            } finally {
                $errorResponse.Dispose()
            }
        }
        throw
    } finally {
        if ($null -ne $response) { $response.Dispose() }
    }
}

function Get-NodeConnection {
    param([int]$RetrySeconds = 0)
    $candidateUrls = New-Object System.Collections.Generic.List[string]
    if (-not [string]::IsNullOrWhiteSpace($env:ELON_NODE_ADMIN_URL)) {
        $candidateUrls.Add($env:ELON_NODE_ADMIN_URL.TrimEnd('/'))
    }
    foreach ($port in 7799..7819) {
        $candidateUrls.Add("http://127.0.0.1:$port")
    }

    $timer = [System.Diagnostics.Stopwatch]::StartNew()
    do {
        foreach ($candidateUrl in ($candidateUrls | Select-Object -Unique)) {
            try {
                $statusHeaders = @{ Origin = $candidateUrl }
                $status = Invoke-Utf8JsonRequest -Method Get -Uri "$candidateUrl/api/status" `
                    -Headers $statusHeaders -TimeoutSec 2
                $token = [string](Get-ObjectField $status 'local_admin_token')
                $header = [string](Get-ObjectField $status 'local_admin_token_header')
                $version = [string](Get-ObjectField $status 'version')
                if (-not [string]::IsNullOrWhiteSpace($token) -and
                    $header.ToLowerInvariant() -eq 'x-elon-local-admin-token' -and
                    -not [string]::IsNullOrWhiteSpace($version)) {
                    return [pscustomobject]@{
                        BaseUrl = $candidateUrl
                        Header = $header
                        Token = $token
                        Version = $version
                    }
                }
            } catch {
                continue
            }
        }
        if ($timer.Elapsed.TotalSeconds -lt $RetrySeconds) { Start-Sleep -Seconds 2 }
    } while ($timer.Elapsed.TotalSeconds -lt $RetrySeconds)
    throw 'No authorized Yilong PC node found on 127.0.0.1 ports 7799-7819.'
}

function Invoke-NodeApi {
    param(
        [object]$Connection,
        [string]$Method,
        [string]$Path,
        [object]$Body = $null
    )
    $requestHeaders = @{ Origin = $Connection.BaseUrl }
    $requestHeaders[$Connection.Header] = $Connection.Token
    Invoke-Utf8JsonRequest -Method $Method -Uri "$($Connection.BaseUrl)$Path" `
        -Headers $requestHeaders -Body $Body -TimeoutSec 15
}

function New-SupervisedTaskBody {
    param(
        [string]$BodyProjectId,
        [string]$BodyWorkspacePath,
        [string]$BodyPrompt,
        [string]$BodyRole,
        [string]$BodyParentTaskId,
        [string]$BodyRootTaskId,
        [string[]]$BodyCriteria,
        [string]$BodyImprovementPolicy
    )
    if ([string]::IsNullOrWhiteSpace($BodyProjectId) -or
        [string]::IsNullOrWhiteSpace($BodyWorkspacePath) -or
        [string]::IsNullOrWhiteSpace($BodyPrompt)) {
        throw 'Submit requires ProjectId, WorkspacePath, and Prompt.'
    }
    $resolvedWorkspace = [System.IO.Path]::GetFullPath($BodyWorkspacePath)
    if (-not [System.IO.Path]::IsPathRooted($resolvedWorkspace)) {
        throw 'WorkspacePath must be absolute.'
    }
    $supervision = [ordered]@{
        protocol = $script:SupervisionProtocol
        supervisor = 'codex_desktop'
        task_role = $BodyRole
        acceptance_criteria = @($BodyCriteria | Where-Object { -not [string]::IsNullOrWhiteSpace($_) })
        improvement_policy = $BodyImprovementPolicy
    }
    if (-not [string]::IsNullOrWhiteSpace($BodyParentTaskId)) {
        $supervision.parent_task_id = $BodyParentTaskId.Trim()
    }
    if (-not [string]::IsNullOrWhiteSpace($BodyRootTaskId)) {
        $supervision.root_task_id = $BodyRootTaskId.Trim()
    }
    return [ordered]@{
        project_id = $BodyProjectId.Trim()
        conversation_id = "desktop-supervised-$([guid]::NewGuid().ToString('N'))"
        workspace_path = $resolvedWorkspace
        prompt = $BodyPrompt.Trim()
        runtime_permission = 'full_access'
        supervision = $supervision
    }
}

function Get-TaskDetailPath {
    param([string]$RequestedTaskId, [int]$Limit = 1000)
    if ([string]::IsNullOrWhiteSpace($RequestedTaskId)) { throw "$Action requires TaskId." }
    $encodedTaskId = [uri]::EscapeDataString($RequestedTaskId.Trim())
    return "/api/local-tasks/${encodedTaskId}?limit=$Limit"
}

function Get-TaskDetail {
    param([object]$Connection, [string]$RequestedTaskId, [int]$Limit = 1000)
    $detailPath = Get-TaskDetailPath $RequestedTaskId $Limit
    Invoke-NodeApi $Connection 'Get' $detailPath
}

function Get-RecordFromDetail {
    param([object]$Detail)
    $record = Get-ObjectField $Detail 'record'
    if ($null -eq $record) { $record = Get-ObjectField $Detail 'task' }
    if ($null -eq $record) { throw 'Node task detail does not contain a record.' }
    return $record
}

function Get-RootTaskFromDetail {
    param([object]$Detail, [string]$FallbackTaskId)
    $supervision = Get-ObjectField $Detail 'supervision'
    $contract = Get-ObjectField $supervision 'contract'
    $root = [string](Get-ObjectField $contract 'root_task_id')
    if ([string]::IsNullOrWhiteSpace($root)) { return $FallbackTaskId }
    return $root
}

function Assert-SafeResumeParentDetail {
    param([object]$ParentDetail, [string]$RequestedParentTaskId)
    if ([string]::IsNullOrWhiteSpace($RequestedParentTaskId)) {
        throw 'Resume requires TaskId.'
    }
    $record = Get-RecordFromDetail $ParentDetail
    $recordTaskId = [string](Get-ObjectField $record 'task_id')
    if (-not [string]::IsNullOrWhiteSpace($recordTaskId) -and $recordTaskId -ne $RequestedParentTaskId) {
        throw 'Resume task detail does not match the requested parent task.'
    }
    $status = ([string](Get-ObjectField $record 'status')).Trim().ToLowerInvariant()
    $terminalStatuses = @('done', 'failed', 'canceled', 'interrupted')
    if ($terminalStatuses -notcontains $status -or $null -eq (Get-ObjectField $record 'finished_at_ms')) {
        throw 'Resume requires a parent task with a reliable terminal status.'
    }
    $supervision = Get-ObjectField $ParentDetail 'supervision'
    $contract = Get-ObjectField $supervision 'contract'
    if ((Get-ObjectField $supervision 'enabled') -ne $true -or
        [string](Get-ObjectField $supervision 'protocol') -ne $script:SupervisionProtocol -or
        [string](Get-ObjectField $contract 'protocol') -ne $script:SupervisionProtocol) {
        throw 'Resume requires a parent task with the current desktop supervision protocol.'
    }
    $workspaceStatus = Get-ObjectField $record 'workspace_status'
    if ((Get-ObjectField $workspaceStatus 'isolated') -ne $true -or
        [string]::IsNullOrWhiteSpace([string](Get-ObjectField $workspaceStatus 'base_workspace_path')) -or
        [string]::IsNullOrWhiteSpace([string](Get-ObjectField $workspaceStatus 'active_workspace_path')) -or
        [string]::IsNullOrWhiteSpace([string](Get-ObjectField $workspaceStatus 'branch'))) {
        throw 'Resume requires a platform-recorded isolated parent worktree.'
    }
}

function New-SupervisionReviewBody {
    param(
        [string]$BodyVerdict,
        [string]$BodySummary,
        [string[]]$BodyImprovements
    )
    return [ordered]@{
        verdict = $BodyVerdict
        summary = $BodySummary
        improvements = @($BodyImprovements | Where-Object { -not [string]::IsNullOrWhiteSpace($_) })
        reviewed_by = 'codex_desktop'
    }
}

function New-ImprovementTaskBody {
    param(
        [object]$ParentDetail,
        [string]$RequestedParentTaskId,
        [string]$ImprovementPrompt,
        [string[]]$BodyCriteria,
        [bool]$IsBlocking
    )
    if ([string]::IsNullOrWhiteSpace($ImprovementPrompt)) {
        throw 'Improve requires a capability improvement Prompt.'
    }
    $parentRecord = Get-RecordFromDetail $ParentDetail
    $parentProjectId = [string](Get-ObjectField $parentRecord 'project_id')
    $parentWorkspace = [string](Get-ObjectField $parentRecord 'workspace_path')
    $rootTask = Get-RootTaskFromDetail $ParentDetail $RequestedParentTaskId
    $role = if ($IsBlocking) { 'capability_repair' } else { 'post_task_improvement' }
    $prefix = if ($IsBlocking) { 'Repair the Yilong PC capability blocking the original task, then return verification evidence:' } else { 'Improve the Yilong PC executor after the user task is complete:' }
    return New-SupervisedTaskBody $parentProjectId $parentWorkspace "$prefix`n$ImprovementPrompt" `
        $role $RequestedParentTaskId $rootTask $BodyCriteria 'after_task_only'
}

function New-ResumeTaskBody {
    param(
        [object]$ParentDetail,
        [string]$RequestedParentTaskId,
        [string[]]$BodyCriteria,
        [string]$BodyImprovementPolicy
    )
    Assert-SafeResumeParentDetail $ParentDetail $RequestedParentTaskId
    $parentRecord = Get-RecordFromDetail $ParentDetail
    $parentProjectId = [string](Get-ObjectField $parentRecord 'project_id')
    $parentWorkspace = [string](Get-ObjectField $parentRecord 'workspace_path')
    $parentPrompt = [string](Get-ObjectField $parentRecord 'prompt')
    $rootTask = Get-RootTaskFromDetail $ParentDetail $RequestedParentTaskId
    $resumePrompt = "The capability repair is complete. Resume the original task. Inspect the current workspace and prior failure evidence before repeating work:`n$parentPrompt"
    return New-SupervisedTaskBody $parentProjectId $parentWorkspace $resumePrompt `
        'resume_original' $RequestedParentTaskId $rootTask $BodyCriteria $BodyImprovementPolicy
}

function Submit-Body {
    param([object]$Connection, [object]$Body, [string]$ResultAction)
    $response = Invoke-NodeApi $Connection 'Post' '/api/local-tasks' $Body
    $responseTaskId = [string](Get-ObjectField $response 'task_id')
    Convert-ToJsonResult ([ordered]@{
        ok = $true
        action = $ResultAction
        protocol = $script:SupervisionProtocol
        node_url = $Connection.BaseUrl
        node_version = $Connection.Version
        task_id = $responseTaskId
        response = $response
    })
}

if ($Action -eq 'SelfTest') {
    $testWorkspace = 'C:\一龙项目\中文工作区'
    $testPrompt = '检查监督链路，保持中文路径完整'
    $testCriteria = @('提交与检查保留中文', '等待与验收摘要无乱码')
    $testSummary = '独立复核：路径、提示和验收条件完整'
    $testBody = New-SupervisedTaskBody '中文项目' $testWorkspace $testPrompt `
        'requirement' '' '' $testCriteria 'after_task_or_unblock'
    [byte[]]$requestBytes = Convert-ToUtf8JsonBytes $testBody
    $requestRoundTrip = Convert-JsonResponseBytes $requestBytes 'application/json'

    $parentJson = [ordered]@{
        record = [ordered]@{
            task_id = 'local-parent-task'
            project_id = '中文项目'
            workspace_path = $testWorkspace
            prompt = $testPrompt
            status = 'failed'
            finished_at_ms = 2
            workspace_status = [ordered]@{
                isolated = $true
                base_workspace_path = $testWorkspace
                active_workspace_path = 'C:\conversation-worktrees\中文项目\中文会话'
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
    $reviewBody = New-SupervisionReviewBody 'accepted' $testSummary @('后续继续观察中文日志')
    [byte[]]$reviewBytes = Convert-ToUtf8JsonBytes $reviewBody
    $reviewRoundTrip = Convert-JsonResponseBytes $reviewBytes 'application/json'
    $improvementBody = New-ImprovementTaskBody $decodedParent 'local-parent-task' '修复中文继承路径' `
        $testCriteria $true
    $resumeBody = New-ResumeTaskBody $decodedParent 'local-parent-task' $testCriteria 'after_task_or_unblock'
    $unsafeParent = [ordered]@{
        record = [ordered]@{
            task_id = 'local-running-task'
            project_id = '中文项目'
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
    if ($testBody.supervision.acceptance_criteria.Count -ne 2 -or
        $testBody.supervision.task_role -ne 'requirement' -or
        $requestRoundTrip.workspace_path -cne $testWorkspace -or
        $requestRoundTrip.prompt -cne $testPrompt -or
        $requestRoundTrip.supervision.acceptance_criteria[0] -cne $testCriteria[0] -or
        $decodedParent.record.workspace_path -cne $testWorkspace -or
        -not $invalidUtf8Rejected -or
        $reviewRoundTrip.summary -cne $testSummary -or
        $improvementBody.workspace_path -cne $testWorkspace -or
        $improvementBody.supervision.task_role -ne 'capability_repair' -or
        $improvementBody.supervision.parent_task_id -ne 'local-parent-task' -or
        $improvementBody.supervision.root_task_id -ne 'local-root-task' -or
        $resumeBody.workspace_path -cne $testWorkspace -or
        $resumeBody.prompt.IndexOf($testPrompt, [System.StringComparison]::Ordinal) -lt 0 -or
        $resumeBody.supervision.task_role -ne 'resume_original' -or
        $resumeBody.supervision.protocol -ne $script:SupervisionProtocol -or
        $resumeBody.supervision.parent_task_id -ne 'local-parent-task' -or
        $resumeBody.supervision.root_task_id -ne 'local-root-task' -or
        -not $unsafeResumeRejected -or
        $script:SupervisionProtocol -ne 'elon.desktop_pc_supervision.v1' -or
        $testDetailPath -ne '/api/local-tasks/local-test%3Fid?limit=1000') {
        throw 'Supervised request construction self-test failed.'
    }
    Convert-ToJsonResult ([ordered]@{
        ok = $true
        action = 'SelfTest'
        protocol = $script:SupervisionProtocol
        checks = @(
            'utf8_request_bytes', 'utf8_response_decode', 'invalid_utf8_rejected', 'non_ascii_workspace',
            'non_ascii_prompt', 'acceptance_criteria', 'review_summary',
            'improve_inherited_path', 'resume_inherited_path', 'resume_parent_guard',
            'task_detail_path'
        )
    })
    exit 0
}

$nodeConnection = if ($Action -eq 'Wait') {
    Get-NodeConnection -RetrySeconds $WaitSeconds
} else {
    Get-NodeConnection
}

switch ($Action) {
    'Probe' {
        Convert-ToJsonResult ([ordered]@{
            ok = $true; action = 'Probe'; protocol = $script:SupervisionProtocol
            node_url = $nodeConnection.BaseUrl; node_version = $nodeConnection.Version
        })
    }
    'Submit' {
        $submitBody = New-SupervisedTaskBody $ProjectId $WorkspacePath $Prompt $TaskRole `
            $ParentTaskId $RootTaskId $AcceptanceCriteria $ImprovementPolicy
        Submit-Body $nodeConnection $submitBody 'Submit'
    }
    'Inspect' {
        $detail = Get-TaskDetail $nodeConnection $TaskId
        Convert-ToJsonResult ([ordered]@{
            ok = $true; action = 'Inspect'; protocol = $script:SupervisionProtocol
            node_url = $nodeConnection.BaseUrl; task_id = $TaskId; detail = $detail
        })
    }
    'Wait' {
        $terminalStatuses = @('done', 'finished', 'success', 'succeeded', 'failed', 'error', 'canceled', 'cancelled', 'interrupted')
        $timer = [System.Diagnostics.Stopwatch]::StartNew()
        $detail = $null
        $status = ''
        do {
            try {
                # Poll only a small event window. The node computes the
                # supervision evidence summary from the complete journal.
                $detail = Get-TaskDetail $nodeConnection $TaskId 25
                $record = Get-RecordFromDetail $detail
                $status = ([string](Get-ObjectField $record 'status')).ToLowerInvariant()
                if ($terminalStatuses -contains $status -or $status -eq 'waiting_approval') { break }
            } catch {
                if ($timer.Elapsed.TotalSeconds -ge $WaitSeconds) { throw }
            }
            Start-Sleep -Seconds 2
        } while ($timer.Elapsed.TotalSeconds -lt $WaitSeconds)
        Convert-ToJsonResult ([ordered]@{
            ok = $true; action = 'Wait'; protocol = $script:SupervisionProtocol
            node_url = $nodeConnection.BaseUrl; task_id = $TaskId; status = $status; detail = $detail
        })
    }
    'Review' {
        if ([string]::IsNullOrWhiteSpace($TaskId)) { throw 'Review requires TaskId.' }
        $reviewBody = New-SupervisionReviewBody $Verdict $Summary $Improvements
        $encodedTaskId = [uri]::EscapeDataString($TaskId.Trim())
        $response = Invoke-NodeApi $nodeConnection 'Post' "/api/local-tasks/$encodedTaskId/supervision/review" $reviewBody
        Convert-ToJsonResult ([ordered]@{
            ok = $true; action = 'Review'; protocol = $script:SupervisionProtocol
            node_url = $nodeConnection.BaseUrl; task_id = $TaskId; verdict = $Verdict; response = $response
        })
    }
    'Improve' {
        $parentDetail = Get-TaskDetail $nodeConnection $TaskId
        $improvementBody = New-ImprovementTaskBody $parentDetail $TaskId $Prompt `
            $AcceptanceCriteria ([bool]$BlockingImprovement)
        Submit-Body $nodeConnection $improvementBody 'Improve'
    }
    'Resume' {
        $parentDetail = Get-TaskDetail $nodeConnection $TaskId
        $resumeBody = New-ResumeTaskBody $parentDetail $TaskId $AcceptanceCriteria $ImprovementPolicy
        Submit-Body $nodeConnection $resumeBody 'Resume'
    }
}
