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

function Get-NodeConnection {
    $candidateUrls = New-Object System.Collections.Generic.List[string]
    if (-not [string]::IsNullOrWhiteSpace($env:ELON_NODE_ADMIN_URL)) {
        $candidateUrls.Add($env:ELON_NODE_ADMIN_URL.TrimEnd('/'))
    }
    foreach ($port in 7799..7819) {
        $candidateUrls.Add("http://127.0.0.1:$port")
    }

    foreach ($candidateUrl in ($candidateUrls | Select-Object -Unique)) {
        try {
            $statusHeaders = @{ Origin = $candidateUrl }
            $status = Invoke-RestMethod -Method Get -Uri "$candidateUrl/api/status" `
                -Headers $statusHeaders -TimeoutSec 2 -UseBasicParsing
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
    $arguments = @{
        Method = $Method
        Uri = "$($Connection.BaseUrl)$Path"
        Headers = $requestHeaders
        TimeoutSec = 15
        UseBasicParsing = $true
    }
    if ($null -ne $Body) {
        $arguments.ContentType = 'application/json; charset=utf-8'
        $arguments.Body = $Body | ConvertTo-Json -Depth 20 -Compress
    }
    Invoke-RestMethod @arguments
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

function Get-TaskDetail {
    param([object]$Connection, [string]$RequestedTaskId)
    if ([string]::IsNullOrWhiteSpace($RequestedTaskId)) { throw "$Action requires TaskId." }
    $encodedTaskId = [uri]::EscapeDataString($RequestedTaskId.Trim())
    Invoke-NodeApi $Connection 'Get' "/api/local-tasks/$encodedTaskId?limit=1000"
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
    $testBody = New-SupervisedTaskBody 'self-test' 'C:\self-test-workspace' 'Verify supervision' `
        'requirement' '' '' @('Task completes', 'Evidence is inspectable') 'after_task_or_unblock'
    if ($testBody.supervision.acceptance_criteria.Count -ne 2 -or
        $testBody.supervision.task_role -ne 'requirement' -or
        $script:SupervisionProtocol -ne 'elon.desktop_pc_supervision.v1') {
        throw 'Supervised request construction self-test failed.'
    }
    Convert-ToJsonResult ([ordered]@{
        ok = $true
        action = 'SelfTest'
        protocol = $script:SupervisionProtocol
        checks = @('request_contract', 'acceptance_criteria', 'executor_role')
    })
    exit 0
}

$nodeConnection = Get-NodeConnection

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
            $detail = Get-TaskDetail $nodeConnection $TaskId
            $record = Get-RecordFromDetail $detail
            $status = ([string](Get-ObjectField $record 'status')).ToLowerInvariant()
            if ($terminalStatuses -contains $status -or $status -eq 'waiting_approval') { break }
            Start-Sleep -Seconds 2
        } while ($timer.Elapsed.TotalSeconds -lt $WaitSeconds)
        Convert-ToJsonResult ([ordered]@{
            ok = $true; action = 'Wait'; protocol = $script:SupervisionProtocol
            node_url = $nodeConnection.BaseUrl; task_id = $TaskId; status = $status; detail = $detail
        })
    }
    'Review' {
        if ([string]::IsNullOrWhiteSpace($TaskId)) { throw 'Review requires TaskId.' }
        $reviewBody = [ordered]@{
            verdict = $Verdict
            summary = $Summary
            improvements = @($Improvements | Where-Object { -not [string]::IsNullOrWhiteSpace($_) })
            reviewed_by = 'codex_desktop'
        }
        $encodedTaskId = [uri]::EscapeDataString($TaskId.Trim())
        $response = Invoke-NodeApi $nodeConnection 'Post' "/api/local-tasks/$encodedTaskId/supervision/review" $reviewBody
        Convert-ToJsonResult ([ordered]@{
            ok = $true; action = 'Review'; protocol = $script:SupervisionProtocol
            node_url = $nodeConnection.BaseUrl; task_id = $TaskId; verdict = $Verdict; response = $response
        })
    }
    'Improve' {
        if ([string]::IsNullOrWhiteSpace($Prompt)) { throw 'Improve requires a capability improvement Prompt.' }
        $parentDetail = Get-TaskDetail $nodeConnection $TaskId
        $parentRecord = Get-RecordFromDetail $parentDetail
        $parentProjectId = [string](Get-ObjectField $parentRecord 'project_id')
        $parentWorkspace = [string](Get-ObjectField $parentRecord 'workspace_path')
        $rootTask = Get-RootTaskFromDetail $parentDetail $TaskId
        $role = if ($BlockingImprovement) { 'capability_repair' } else { 'post_task_improvement' }
        $prefix = if ($BlockingImprovement) { 'Repair the Yilong PC capability blocking the original task, then return verification evidence:' } else { 'Improve the Yilong PC executor after the user task is complete:' }
        $improvementBody = New-SupervisedTaskBody $parentProjectId $parentWorkspace "$prefix`n$Prompt" `
            $role $TaskId $rootTask $AcceptanceCriteria 'after_task_only'
        Submit-Body $nodeConnection $improvementBody 'Improve'
    }
    'Resume' {
        $parentDetail = Get-TaskDetail $nodeConnection $TaskId
        $parentRecord = Get-RecordFromDetail $parentDetail
        $parentProjectId = [string](Get-ObjectField $parentRecord 'project_id')
        $parentWorkspace = [string](Get-ObjectField $parentRecord 'workspace_path')
        $parentPrompt = [string](Get-ObjectField $parentRecord 'prompt')
        $rootTask = Get-RootTaskFromDetail $parentDetail $TaskId
        $resumePrompt = "The capability repair is complete. Resume the original task. Inspect the current workspace and prior failure evidence before repeating work:`n$parentPrompt"
        $resumeBody = New-SupervisedTaskBody $parentProjectId $parentWorkspace $resumePrompt `
            'resume_original' $TaskId $rootTask $AcceptanceCriteria $ImprovementPolicy
        Submit-Body $nodeConnection $resumeBody 'Resume'
    }
}
