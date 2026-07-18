[CmdletBinding()]
param(
    [Parameter(Mandatory = $true)]
    [ValidateSet('Probe', 'Projects', 'Submit', 'Inspect', 'Wait', 'Review', 'Improve', 'Resume', 'SelfTest')]
    [string]$Action,
    [string]$ProjectId = 'elon-self',
    [string]$WorkspacePath,
    [string]$Prompt,
    [string[]]$AcceptanceCriteria = @(),
    [string]$AcceptanceCriteriaJson = '',
    [string]$AcceptanceCriteriaFile = '',
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
    [string]$ProjectFilterId = '',
    [switch]$IncludeSystemProjects,
    [switch]$BlockingImprovement,
    [ValidateRange(1, 55)]
    [int]$WaitSeconds = 55,
    [ValidateRange(-1, 2147483647)]
    [int]$Since = -1,
    [ValidateRange(1, 200)]
    [int]$Limit = 200,
    [switch]$Compact
)

Set-StrictMode -Version 2.0
$ErrorActionPreference = 'Stop'
$script:SupervisionProtocol = 'elon.desktop_pc_supervision.v1'
$script:LastNodeAdminUrl = ''
$script:CachedNodeAdminUrl = ''
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

function Resolve-AcceptanceCriteria {
    param(
        [string[]]$LegacyCriteria,
        [string]$CriteriaJson,
        [string]$CriteriaFile
    )
    $sources = 0
    if (@($LegacyCriteria | Where-Object { -not [string]::IsNullOrWhiteSpace($_) }).Count -gt 0) { $sources++ }
    if (-not [string]::IsNullOrWhiteSpace($CriteriaJson)) { $sources++ }
    if (-not [string]::IsNullOrWhiteSpace($CriteriaFile)) { $sources++ }
    if ($sources -gt 1) {
        throw 'Use only one of AcceptanceCriteria, AcceptanceCriteriaJson, or AcceptanceCriteriaFile.'
    }

    $parsed = $null
    if (-not [string]::IsNullOrWhiteSpace($CriteriaFile)) {
        $fullPath = [System.IO.Path]::GetFullPath($CriteriaFile)
        if (-not (Test-Path -LiteralPath $fullPath -PathType Leaf)) {
            throw "AcceptanceCriteriaFile does not exist: $fullPath"
        }
        $info = Get-Item -LiteralPath $fullPath
        if ($info.Length -gt 262144) { throw 'AcceptanceCriteriaFile exceeds 256 KiB.' }
        [byte[]]$bytes = [System.IO.File]::ReadAllBytes($fullPath)
        $text = Convert-ResponseBytesToText $bytes 'application/json; charset=utf-8'
        $parsed = $text | ConvertFrom-Json
    } elseif (-not [string]::IsNullOrWhiteSpace($CriteriaJson)) {
        $parsed = $CriteriaJson | ConvertFrom-Json
    } else {
        return [string[]]@($LegacyCriteria | ForEach-Object { ([string]$_).Trim() } | Where-Object { $_ })
    }

    $criteriaValue = $parsed
    $property = $parsed.PSObject.Properties['acceptance_criteria']
    if ($null -ne $property) { $criteriaValue = $property.Value }
    $items = @($criteriaValue | ForEach-Object { ([string]$_).Trim() } | Where-Object { $_ })
    if ($items.Count -gt 20) { throw 'Acceptance criteria cannot contain more than 20 items.' }
    foreach ($item in $items) {
        if ($item.Length -gt 2000) { throw 'An acceptance criterion exceeds 2000 characters.' }
    }
    return [string[]]$items
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

function Get-NodeUrlCachePath {
    $root = if (-not [string]::IsNullOrWhiteSpace($env:LOCALAPPDATA)) {
        Join-Path $env:LOCALAPPDATA 'ElonNode'
    } else {
        Join-Path ([System.IO.Path]::GetTempPath()) 'ElonNode'
    }
    return Join-Path $root 'supervisor-node-url.txt'
}

function Get-CachedNodeUrl {
    $path = Get-NodeUrlCachePath
    if (-not (Test-Path -LiteralPath $path -PathType Leaf)) { return '' }
    try {
        $url = ([System.IO.File]::ReadAllText($path, $script:Utf8NoBomStrict)).TrimEnd('/')
        if ($url -match '^http://127\.0\.0\.1:(7799|78(?:0[0-9]|1[0-9]))$') {
            $script:CachedNodeAdminUrl = $url
            return $url
        }
    } catch {}
    return ''
}

function Save-CachedNodeUrl {
    param([string]$Url)
    if ($Url -notmatch '^http://127\.0\.0\.1:(7799|78(?:0[0-9]|1[0-9]))$') { return }
    if ($script:CachedNodeAdminUrl -eq $Url) { return }
    try {
        $path = Get-NodeUrlCachePath
        [System.IO.Directory]::CreateDirectory([System.IO.Path]::GetDirectoryName($path)) | Out-Null
        [System.IO.File]::WriteAllText($path, $Url, $script:Utf8NoBom)
        $script:CachedNodeAdminUrl = $Url
    } catch {}
}

function Get-NodeConnection {
    param([int]$RetrySeconds = 0)
    $candidateUrls = New-Object System.Collections.Generic.List[string]
    if (-not [string]::IsNullOrWhiteSpace($env:ELON_NODE_ADMIN_URL)) {
        $candidateUrls.Add($env:ELON_NODE_ADMIN_URL.TrimEnd('/'))
    }
    if (-not [string]::IsNullOrWhiteSpace($script:LastNodeAdminUrl)) {
        $candidateUrls.Add($script:LastNodeAdminUrl)
    }
    $cachedUrl = Get-CachedNodeUrl
    if (-not [string]::IsNullOrWhiteSpace($cachedUrl)) { $candidateUrls.Add($cachedUrl) }
    $candidateUrls.Add('http://127.0.0.1:7799')
    foreach ($port in 7800..7819) {
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
                    $script:LastNodeAdminUrl = $candidateUrl
                    Save-CachedNodeUrl $candidateUrl
                    return [pscustomobject]@{
                        BaseUrl = $candidateUrl
                        Header = $header
                        Token = $token
                        Version = $version
                        ProbeMs = $timer.ElapsedMilliseconds
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

function Get-CloudProjectsPath {
    param([bool]$IncludeSystem)
    if ($IncludeSystem) { return '/api/cloud-projects?include_system=true' }
    return '/api/cloud-projects'
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
    param([string]$RequestedTaskId, [int]$Limit = 200, [int]$Since = -1)
    if ([string]::IsNullOrWhiteSpace($RequestedTaskId)) { throw "$Action requires TaskId." }
    $encodedTaskId = [uri]::EscapeDataString($RequestedTaskId.Trim())
    if ($Since -ge 0) {
        return "/api/local-tasks/${encodedTaskId}?since=$Since&limit=$Limit"
    }
    return "/api/local-tasks/${encodedTaskId}?limit=$Limit"
}

function Get-TaskDetail {
    param([object]$Connection, [string]$RequestedTaskId, [int]$Limit = 200, [int]$Since = -1)
    $detailPath = Get-TaskDetailPath $RequestedTaskId $Limit $Since
    Invoke-NodeApi $Connection 'Get' $detailPath
}

function Convert-ToCompactTaskDetail {
    param([object]$Detail)
    $record = Get-RecordFromDetail $Detail
    $supervision = Get-ObjectField $Detail 'supervision'
    $events = @((Get-ObjectField $Detail 'events') | ForEach-Object {
        $event = Get-ObjectField $_ 'event'
        [ordered]@{
            seq = Get-ObjectField $_ 'seq'
            type = Get-ObjectField $event 'type'
            phase = Get-ObjectField $event 'phase'
            lifecycle = Get-ObjectField $event 'lifecycle'
        }
    })
    return [ordered]@{
        record = [ordered]@{
            task_id = Get-ObjectField $record 'task_id'
            status = Get-ObjectField $record 'status'
            error = Get-ObjectField $record 'error'
            finished_at_ms = Get-ObjectField $record 'finished_at_ms'
        }
        runtime = Get-ObjectField $Detail 'runtime'
        approval_state = Get-ObjectField $Detail 'approval_state'
        evidence = Get-ObjectField $supervision 'evidence'
        events = $events
        last_event_seq = Get-ObjectField $Detail 'last_event_seq'
        has_more = Get-ObjectField $Detail 'has_more'
    }
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
    $terminalStatuses = @('done', 'failed', 'canceled', 'interrupted', 'resume_required')
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
    $resumeStatus = Get-ObjectField $ParentDetail 'resume_workspace_status'
    $derivation = [string](Get-ObjectField $resumeStatus 'derivation')
    $allowedDerivations = @(
        'workspace_status',
        'legacy_started_cwd_git_registry',
        'platform_receipt_commit_rebuild_available',
        'workspace_status_git_recovery_ready_legacy_branch_ref'
    )
    if ((Get-ObjectField $resumeStatus 'eligible') -ne $true -or
        $allowedDerivations -notcontains $derivation -or
        (Get-ObjectField $resumeStatus 'occupied') -eq $true -or
        [string]::IsNullOrWhiteSpace([string](Get-ObjectField $resumeStatus 'active_workspace_path')) -or
        [string]::IsNullOrWhiteSpace([string](Get-ObjectField $resumeStatus 'branch')) -or
        [string]::IsNullOrWhiteSpace([string](Get-ObjectField $resumeStatus 'git_head'))) {
        throw 'Resume requires the current node to validate an eligible isolated parent worktree.'
    }
    $workspaceStatus = Get-ObjectField $record 'workspace_status'
    if ((Get-ObjectField $workspaceStatus 'isolated') -ne $true -or
        [string]::IsNullOrWhiteSpace([string](Get-ObjectField $workspaceStatus 'base_workspace_path')) -or
        [string]::IsNullOrWhiteSpace([string](Get-ObjectField $workspaceStatus 'active_workspace_path')) -or
        [string]::IsNullOrWhiteSpace([string](Get-ObjectField $workspaceStatus 'branch'))) {
        if ($derivation -ne 'legacy_started_cwd_git_registry') {
            throw 'Resume requires a node-validated isolated parent worktree.'
        }
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

. (Join-Path $PSScriptRoot 'invoke-supervised-task-self-test.ps1')
if ($Action -eq 'SelfTest') { Invoke-SupervisionSelfTest; exit 0 }

$resolvedAcceptanceCriteria = @(Resolve-AcceptanceCriteria `
    $AcceptanceCriteria $AcceptanceCriteriaJson $AcceptanceCriteriaFile)

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
            probe_ms = $nodeConnection.ProbeMs; cache_contains_token = $false
        })
    }
    'Projects' {
        $projectsPath = Get-CloudProjectsPath ([bool]$IncludeSystemProjects)
        $response = Invoke-NodeApi $nodeConnection 'Get' $projectsPath
        $projects = @(Get-ObjectField $response 'projects')
        if (-not [string]::IsNullOrWhiteSpace($ProjectFilterId)) {
            $requestedProjectId = $ProjectFilterId.Trim()
            $projects = @($projects | Where-Object {
                [string](Get-ObjectField $_ 'id') -eq $requestedProjectId
            })
        }
        Convert-ToJsonResult ([ordered]@{
            ok = $true; action = 'Projects'; protocol = $script:SupervisionProtocol
            node_url = $nodeConnection.BaseUrl; node_version = $nodeConnection.Version
            node_id = Get-ObjectField $response 'node_id'
            transport = Get-ObjectField $response 'transport'
            projects = $projects
        })
    }
    'Submit' {
        $submitBody = New-SupervisedTaskBody $ProjectId $WorkspacePath $Prompt $TaskRole `
            $ParentTaskId $RootTaskId $resolvedAcceptanceCriteria $ImprovementPolicy
        Submit-Body $nodeConnection $submitBody 'Submit'
    }
    'Inspect' {
        $detail = Get-TaskDetail $nodeConnection $TaskId $Limit $Since
        $nextCursor = [int](Get-ObjectField $detail 'last_event_seq')
        $resultDetail = if ($Compact) { Convert-ToCompactTaskDetail $detail } else { $detail }
        Convert-ToJsonResult ([ordered]@{
            ok = $true; action = 'Inspect'; protocol = $script:SupervisionProtocol
            node_url = $nodeConnection.BaseUrl; task_id = $TaskId
            since = $Since; limit = $Limit; next_cursor = $nextCursor; detail = $resultDetail
        })
    }
    'Wait' {
        $terminalStatuses = @('done', 'finished', 'success', 'succeeded', 'failed', 'error', 'canceled', 'cancelled', 'interrupted', 'resume_required')
        $timer = [System.Diagnostics.Stopwatch]::StartNew()
        $detail = $null
        $status = ''
        $cursor = if ($Since -ge 0) { $Since } else { 0 }
        $waitLimit = if ($PSBoundParameters.ContainsKey('Limit')) { $Limit } else { 25 }
        do {
            try {
                # Poll only a small event window. The node computes the
                # supervision evidence summary from the complete journal.
                $detail = Get-TaskDetail $nodeConnection $TaskId $waitLimit $cursor
                $record = Get-RecordFromDetail $detail
                $status = ([string](Get-ObjectField $record 'status')).ToLowerInvariant()
                $returnedCursor = [int](Get-ObjectField $detail 'last_event_seq')
                if ($returnedCursor -gt $cursor) { $cursor = $returnedCursor }
                if ($terminalStatuses -contains $status -or $status -eq 'waiting_approval') { break }
                if ((Get-ObjectField $detail 'has_more') -eq $true) { continue }
            } catch {
                if ($timer.Elapsed.TotalSeconds -ge $WaitSeconds) { throw }
            }
            Start-Sleep -Seconds 2
        } while ($timer.Elapsed.TotalSeconds -lt $WaitSeconds)
        $resultDetail = if ($Compact) { Convert-ToCompactTaskDetail $detail } else { $detail }
        Convert-ToJsonResult ([ordered]@{
            ok = $true; action = 'Wait'; protocol = $script:SupervisionProtocol
            node_url = $nodeConnection.BaseUrl; task_id = $TaskId; status = $status
            since = $(if ($Since -ge 0) { $Since } else { 0 }); limit = $waitLimit
            next_cursor = $cursor; detail = $resultDetail
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
            $resolvedAcceptanceCriteria ([bool]$BlockingImprovement)
        Submit-Body $nodeConnection $improvementBody 'Improve'
    }
    'Resume' {
        $parentDetail = Get-TaskDetail $nodeConnection $TaskId
        $resumeBody = New-ResumeTaskBody $parentDetail $TaskId $resolvedAcceptanceCriteria $ImprovementPolicy
        Submit-Body $nodeConnection $resumeBody 'Resume'
    }
}
