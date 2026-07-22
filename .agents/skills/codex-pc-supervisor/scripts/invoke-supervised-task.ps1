[CmdletBinding()]
param(
    [Parameter(Mandatory = $true)]
    [ValidateSet('Probe', 'Projects', 'InspectProjectBinding', 'BindProject', 'ReconcileUpdate', 'Submit', 'Inspect', 'Wait', 'Review', 'Improve', 'Resume', 'Supersede', 'SelfTest')]
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
    [string]$AmendmentReason = '',
    [string]$ProjectFilterId = '',
    [switch]$IncludeSystemProjects,
    [switch]$BlockingImprovement,
    [ValidateRange(1, 55)]
    [int]$WaitSeconds = 55,
    [ValidateRange(-1, 2147483647)]
    [int]$Since = -1,
    [ValidateRange(1, 200)]
    [int]$Limit = 200,
    [string]$ExpectedCursorEpoch = '',
    [string]$ExpectedStateDigest = '',
    [string]$ExpectedEvidenceDigest = '',
    [string]$StateRoot = '',
    [string]$InstallRoot = '',
    [string]$DesktopReviewStateRoot = '',
    [string]$DesktopReviewInstallRoot = '',
    [switch]$Compact
)

Set-StrictMode -Version 2.0
$ErrorActionPreference = 'Stop'
$script:SupervisionProtocol = 'elon.desktop_pc_supervision.v1'
$script:DeltaWaitCapability = 'delta_wait_v1'
$script:TaskDeltaSchema = 'elon.supervision.task_delta.v1'
$script:ResumeContextCapability = 'resume_context_v1'
$script:ContractSupersedeCapability = 'contract_supersede_v1'
$script:DesktopReviewCapability = 'desktop_review_ticket_v3'
$script:LastNodeAdminUrl = ''
$script:CachedNodeAdminUrl = ''
$script:Utf8NoBom = [System.Text.UTF8Encoding]::new($false)
$script:Utf8NoBomStrict = [System.Text.UTF8Encoding]::new($false, $true)
$OutputEncoding = $script:Utf8NoBom
try { [Console]::OutputEncoding = $script:Utf8NoBom } catch {}
try { Add-Type -AssemblyName System.Net.Http -ErrorAction Stop } catch {}

function Get-ObjectField {
    param([object]$InputObject, [string]$Name)
    if ($null -eq $InputObject) { return $null }
    if ($InputObject -is [System.Collections.IDictionary]) {
        if ($InputObject.Contains($Name)) { return $InputObject[$Name] }
        return $null
    }
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
        [byte[]]$BodyBytes = $null,
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
    if ($null -ne $Body -or $null -ne $BodyBytes) {
        [byte[]]$requestBytes = if ($null -ne $BodyBytes) { $BodyBytes } else { Convert-ToUtf8JsonBytes $Body }
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

. (Join-Path $PSScriptRoot 'invoke-supervised-task-connection.ps1')

function Invoke-NodeApi {
    param(
        [object]$Connection,
        [string]$Method,
        [string]$Path,
        [object]$Body = $null,
        [byte[]]$BodyBytes = $null,
        [System.Collections.IDictionary]$ExtraHeaders = $null,
        [ValidateRange(1, 60)][int]$TimeoutSec = 15
    )
    $requestHeaders = @{ Origin = $Connection.BaseUrl }
    $requestHeaders[$Connection.Header] = $Connection.Token
    if ($null -ne $ExtraHeaders) {
        foreach ($name in $ExtraHeaders.Keys) {
            $requestHeaders[[string]$name] = [string]$ExtraHeaders[$name]
        }
    }
    Invoke-Utf8JsonRequest -Method $Method -Uri "$($Connection.BaseUrl)$Path" `
        -Headers $requestHeaders -Body $Body -BodyBytes $BodyBytes -TimeoutSec $TimeoutSec
}

function New-DesktopReviewTicket {
    param([string]$OwnerUserId, [string]$RequestedTaskId, [string]$Method, [string]$EndpointPath, [byte[]]$BodyBytes)
    $stateRoot = if (-not [string]::IsNullOrWhiteSpace($DesktopReviewStateRoot)) { $DesktopReviewStateRoot } elseif (-not [string]::IsNullOrWhiteSpace($StateRoot)) { $StateRoot } else { [string]$env:ELON_DESKTOP_REVIEW_STATE_ROOT }
    $installRoot = if (-not [string]::IsNullOrWhiteSpace($DesktopReviewInstallRoot)) { $DesktopReviewInstallRoot } elseif (-not [string]::IsNullOrWhiteSpace($InstallRoot)) { $InstallRoot } else { [string]$env:ELON_DESKTOP_REVIEW_INSTALL_ROOT }
    if ([string]::IsNullOrWhiteSpace($stateRoot) -or [string]::IsNullOrWhiteSpace($installRoot)) {
        throw 'desktop_review_paths_not_configured: set -DesktopReviewStateRoot/-DesktopReviewInstallRoot or ELON_DESKTOP_REVIEW_STATE_ROOT/ELON_DESKTOP_REVIEW_INSTALL_ROOT'
    }
    $signer = Join-Path ([IO.Path]::GetFullPath($installRoot)) '_internal\new-desktop-review-ticket.ps1'
    if (Test-Path -LiteralPath $signer -PathType Leaf) {
        $sha = [Security.Cryptography.SHA256]::Create()
        try { $bodyHash = -join ($sha.ComputeHash($BodyBytes) | ForEach-Object { $_.ToString('x2') }) } finally { $sha.Dispose() }
        $ticket = & $signer -OwnerUserId $OwnerUserId -TaskId $RequestedTaskId -Method $Method `
            -EndpointPath $EndpointPath -BodySha256 $bodyHash -StateRoot $stateRoot -InstallRoot $installRoot
        if ($LASTEXITCODE -ne 0 -or [string]::IsNullOrWhiteSpace([string]$ticket)) {
            throw 'desktop_review_signer_unavailable: signer failed or private key/ACL is inaccessible'
        }
        return [string]$ticket
    }
    throw 'desktop_review_signer_missing: configured InstallRoot does not contain the signer'
}

function Get-CloudProjectsPath {
    param([bool]$IncludeSystem)
    if ($IncludeSystem) { return '/api/cloud-projects?include_system=true' }
    return '/api/cloud-projects'
}

function New-ProjectBindingBody {
    param([string]$RequestedProjectId, [string]$RequestedWorkspacePath)
    if ([string]::IsNullOrWhiteSpace($RequestedProjectId) -or
        [string]::IsNullOrWhiteSpace($RequestedWorkspacePath)) {
        throw 'ProjectId and WorkspacePath are required.'
    }
    $resolved = [System.IO.Path]::GetFullPath($RequestedWorkspacePath)
    if (-not [System.IO.Path]::IsPathRooted($resolved)) { throw 'WorkspacePath must be absolute.' }
    return [ordered]@{ project_id = $RequestedProjectId.Trim(); workspace_path = $resolved }
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
    param(
        [string]$RequestedTaskId,
        [int]$Limit = 200,
        [int]$Since = -1,
        [string]$CursorEpoch = ''
    )
    if ([string]::IsNullOrWhiteSpace($RequestedTaskId)) { throw "$Action requires TaskId." }
    $encodedTaskId = [uri]::EscapeDataString($RequestedTaskId.Trim())
    $query = @("limit=$Limit")
    if ($Since -ge 0) { $query = @("since=$Since") + $query }
    if (-not [string]::IsNullOrWhiteSpace($CursorEpoch)) {
        $query += "expected_cursor_epoch=$([uri]::EscapeDataString($CursorEpoch.Trim()))"
    }
    return "/api/local-tasks/${encodedTaskId}?$($query -join '&')"
}

function Get-TaskDetail {
    param(
        [object]$Connection,
        [string]$RequestedTaskId,
        [int]$Limit = 200,
        [int]$Since = -1,
        [string]$CursorEpoch = '',
        [ValidateRange(1, 15)][int]$TimeoutSec = 15
    )
    $detailPath = Get-TaskDetailPath $RequestedTaskId $Limit $Since $CursorEpoch
    Invoke-NodeApi $Connection 'Get' $detailPath -TimeoutSec $TimeoutSec
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
    $parentRole = [string](Get-ObjectField $contract 'task_role')
    if (@('requirement', 'resume_original') -notcontains $parentRole) {
        throw 'Resume parent task_role must be requirement or resume_original.'
    }
    $resumeStatus = Get-ObjectField $ParentDetail 'resume_workspace_status'
    $derivation = [string](Get-ObjectField $resumeStatus 'derivation')
    $allowedDerivations = @(
        'workspace_status',
        'inherited_workspace_status',
        'legacy_started_cwd_git_registry',
        'platform_receipt_commit_rebuild_available',
        'workspace_status_git_recovery_ready_recorded_head',
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
    }
}

function Resolve-MonotonicTaskCursor {
    param(
        [int]$CurrentCursor,
        [int]$ReturnedCursor,
        [bool]$CursorReset,
        [int]$ResumeCursor
    )
    if ($CursorReset) { return $ResumeCursor }
    return [Math]::Max($CurrentCursor, $ReturnedCursor)
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
    $workspaceStatus = Get-ObjectField $parentRecord 'workspace_status'
    $recordedBaseWorkspace = [string](Get-ObjectField $workspaceStatus 'base_workspace_path')
    if (-not [string]::IsNullOrWhiteSpace($recordedBaseWorkspace)) {
        $parentWorkspace = $recordedBaseWorkspace
    }
    $rootTask = Get-RootTaskFromDetail $ParentDetail $RequestedParentTaskId
    $role = if ($IsBlocking) { 'capability_repair' } else { 'post_task_improvement' }
    $prefix = if ($IsBlocking) { 'Repair the Yilong PC capability blocking the original task, then return verification evidence:' } else { 'Improve the Yilong PC executor after the user task is complete:' }
    return New-SupervisedTaskBody $parentProjectId $parentWorkspace "$prefix`n$ImprovementPrompt" `
        $role $RequestedParentTaskId $rootTask $BodyCriteria 'after_task_only'
}

. (Join-Path $PSScriptRoot 'invoke-supervised-task-delta.ps1')
. (Join-Path $PSScriptRoot 'invoke-supervised-task-wait.ps1')
. (Join-Path $PSScriptRoot 'invoke-supervised-task-idempotency.ps1')
. (Join-Path $PSScriptRoot 'invoke-supervised-task-review.ps1')
. (Join-Path $PSScriptRoot 'invoke-supervised-task-workspace.ps1')
. (Join-Path $PSScriptRoot 'invoke-supervised-task-continuation.ps1')
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
            supervision_protocol = $nodeConnection.SupervisionProtocol
            supervision_capabilities = @($nodeConnection.SupervisionCapabilities)
            probe_ms = $nodeConnection.ProbeMs; probe_strategy = $nodeConnection.ProbeStrategy
            phase_timings = $nodeConnection.ProbeTimings
            cache_contains_token = $false
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
            cloud_round_trip_ms = Get-ObjectField $response 'cloud_round_trip_ms'
            project_count = Get-ObjectField $response 'project_count'
            projects = $projects
        })
    }
    'InspectProjectBinding' {
        $body = New-ProjectBindingBody $ProjectId $WorkspacePath
        $response = Invoke-NodeApi $nodeConnection 'Post' '/api/cloud-projects/inspect-binding' $body
        Convert-ToJsonResult ([ordered]@{
            ok = $true; action = 'InspectProjectBinding'; protocol = $script:SupervisionProtocol
            node_url = $nodeConnection.BaseUrl; binding = Get-ObjectField $response 'binding'
        })
    }
    'BindProject' {
        $body = New-ProjectBindingBody $ProjectId $WorkspacePath
        $response = Invoke-NodeApi $nodeConnection 'Post' '/api/cloud-projects/rebind' $body
        Convert-ToJsonResult ([ordered]@{
            ok = $true; action = 'BindProject'; protocol = $script:SupervisionProtocol
            node_url = $nodeConnection.BaseUrl
            project_id = Get-ObjectField $response 'project_id'
            binding = Get-ObjectField $response 'binding'
            cloud_receipt = Get-ObjectField $response 'cloud_receipt'
            timings = Get-ObjectField $response 'timings'
        })
    }
    'ReconcileUpdate' {
        # Reconcile performs a complete, fail-closed audit of historical task
        # ownership and worktree evidence. Keep ordinary task reads at 15s,
        # but give this explicit maintenance operation a bounded 60s window.
        $response = Invoke-NodeApi $nodeConnection 'Post' '/api/update-recovery/reconcile' @{} `
            -TimeoutSec 60
        Convert-ToJsonResult ([ordered]@{
            ok = $true; action = 'ReconcileUpdate'; protocol = $script:SupervisionProtocol
            node_url = $nodeConnection.BaseUrl
            reconcile_id = Get-ObjectField $response 'reconcile_id'
            install_may_proceed = Get-ObjectField $response 'install_may_proceed'
            excluded_terminal_history_count = Get-ObjectField $response 'excluded_terminal_history_count'
            active_foreground_task_ids = Get-ObjectField $response 'active_foreground_task_ids'
            orphan_rows_reconciled = Get-ObjectField $response 'orphan_rows_reconciled'
            orphan_reconcile_error = Get-ObjectField $response 'orphan_reconcile_error'
            install_gate = Get-ObjectField $response 'install_gate'
        })
    }
    'Submit' {
        $resolution = Resolve-SubmitProjectWorkspace $nodeConnection $ProjectId $WorkspacePath
        $submitBody = New-SupervisedTaskBody $ProjectId $resolution.ResolvedPath $Prompt $TaskRole `
            $ParentTaskId $RootTaskId $resolvedAcceptanceCriteria $ImprovementPolicy
        Submit-Body $nodeConnection $submitBody 'Submit' ([ordered]@{
            project_id = $resolution.ProjectId
            requested_workspace_path = $resolution.RequestedPath
            authorized_workspace_path = $resolution.AuthorizedPath
            resolved_workspace_path = $resolution.ResolvedPath
            corrected = $resolution.Corrected
            runtime_permission = $resolution.RuntimePermission
        })
    }
    'Inspect' {
        if ($Compact) {
            Assert-NodeSupervisionCapability $nodeConnection $script:DeltaWaitCapability 'Compact Inspect'
        }
        $detail = Get-TaskDetail $nodeConnection $TaskId $Limit $Since $ExpectedCursorEpoch
        $nextCursor = Resolve-MonotonicTaskCursor $Since `
            ([int](Get-ObjectField $detail 'last_event_seq')) `
            ([bool](Get-ObjectField $detail 'cursor_reset')) `
            ([int](Get-ObjectField $detail 'resume_cursor'))
        $record = Get-RecordFromDetail $detail
        $inspectStatus = ([string](Get-ObjectField $record 'status')).ToLowerInvariant()
        $terminalStatuses = @('done', 'finished', 'success', 'succeeded', 'failed', 'error', 'canceled', 'cancelled', 'interrupted', 'resume_required')
        $resultDetail = if ($Compact) {
            Select-TaskDeltaChanges `
                (Convert-ToCompactTaskDetail $detail $null ($terminalStatuses -contains $inspectStatus)) `
                $ExpectedStateDigest $ExpectedEvidenceDigest
        } else { $detail }
        Convert-ToJsonResult ([ordered]@{
            ok = $true; action = 'Inspect'; protocol = $script:SupervisionProtocol
            node_url = $nodeConnection.BaseUrl; task_id = $TaskId
            since = $Since; limit = $Limit; next_cursor = $nextCursor; detail = $resultDetail
            cursor_reset = [bool](Get-ObjectField $detail 'cursor_reset')
            requested_cursor = Get-ObjectField $detail 'requested_cursor'
            old_cursor = Get-ObjectField $detail 'old_cursor'
            new_cursor = Get-ObjectField $detail 'new_cursor'
            resume_cursor = Get-ObjectField $detail 'resume_cursor'
            cursor_epoch = Get-ObjectField $detail 'cursor_epoch'
            requested_cursor_epoch = Get-ObjectField $detail 'requested_cursor_epoch'
            previous_cursor_epoch = Get-ObjectField $detail 'previous_cursor_epoch'
            sidecar_update_epoch = Get-ObjectField $detail 'sidecar_update_epoch'
            delta_from = $(if ($Since -ge 0) { $Since } else { 0 })
            delta_to = $nextCursor
            delta_event_count = @((Get-ObjectField $detail 'events')).Count
            state_digest = $(if ($Compact) { Get-ObjectField $resultDetail 'state_digest' } else { $null })
            delta_schema = $(if ($Compact) { $script:TaskDeltaSchema } else { $null })
        })
    }
    'Wait' {
        Invoke-SupervisedWait $nodeConnection $TaskId ([bool]$Compact) $WaitSeconds $Since `
            $Limit ([bool]$PSBoundParameters.ContainsKey('Limit')) $ExpectedCursorEpoch `
            $ExpectedStateDigest $ExpectedEvidenceDigest
    }
    'Review' {
        Assert-NodeSupervisionCapability $nodeConnection $script:DesktopReviewCapability 'Desktop Review'
        if ([string]::IsNullOrWhiteSpace($TaskId)) { throw 'Review requires TaskId.' }
        $reviewBody = Convert-ToPublicSupervisionReviewBody `
            (New-SupervisionReviewBody $Verdict $Summary $Improvements)
        [byte[]]$reviewBytes = Convert-ToUtf8JsonBytes $reviewBody
        $encodedTaskId = [uri]::EscapeDataString($TaskId.Trim())
        $reviewPath = "/api/local-tasks/$encodedTaskId/supervision/desktop-review"
        $detail = Get-TaskDetail $nodeConnection $TaskId 1
        $record = Get-RecordFromDetail $detail
        $ownerUserId = [string](Get-ObjectField $record 'owner_user_id')
        $ticket = New-DesktopReviewTicket $ownerUserId $TaskId.Trim() 'POST' $reviewPath $reviewBytes
        $response = Invoke-NodeApi -Connection $nodeConnection -Method 'Post' -Path $reviewPath `
            -BodyBytes $reviewBytes -ExtraHeaders @{ 'x-elon-desktop-review-ticket' = $ticket }
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
        Assert-NodeSupervisionCapability $nodeConnection $script:ResumeContextCapability 'Resume'
        $parentDetail = Get-TaskDetail $nodeConnection $TaskId
        $resumeBody = New-ResumeTaskBody $parentDetail $TaskId $resolvedAcceptanceCriteria $ImprovementPolicy
        Submit-Body $nodeConnection $resumeBody 'Resume'
    }
    'Supersede' {
        Assert-NodeSupervisionCapability $nodeConnection $script:ContractSupersedeCapability 'Supersede'
        $parentDetail = Get-TaskDetail $nodeConnection $TaskId
        $supersedeBody = New-SupersedeTaskBody $parentDetail $TaskId $Prompt `
            $resolvedAcceptanceCriteria $AmendmentReason $ImprovementPolicy
        Submit-Body $nodeConnection $supersedeBody 'Supersede'
    }
}
