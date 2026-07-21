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
$script:DesktopReviewCapability = 'desktop_review_ticket_v3'
$script:LastNodeAdminUrl = ''
$script:CachedNodeAdminUrl = ''
$script:Utf8NoBom = [System.Text.UTF8Encoding]::new($false)
$script:Utf8NoBomStrict = [System.Text.UTF8Encoding]::new($false, $true)
$OutputEncoding = $script:Utf8NoBom
try { [Console]::OutputEncoding = $script:Utf8NoBom } catch {}

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
                $supervisionStatus = Get-ObjectField $status 'desktop_supervision'
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
                        SupervisionProtocol = [string](Get-ObjectField $supervisionStatus 'protocol')
                        SupervisionCapabilities = @((Get-ObjectField $supervisionStatus 'capabilities') | ForEach-Object { [string]$_ })
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
        [object]$Body = $null,
        [byte[]]$BodyBytes = $null,
        [System.Collections.IDictionary]$ExtraHeaders = $null,
        [ValidateRange(1, 15)][int]$TimeoutSec = 15
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

function Get-WaitFailureCode {
    param([System.Exception]$Exception)
    if ($Exception -is [System.Net.WebException] -and
        $Exception.Status -eq [System.Net.WebExceptionStatus]::Timeout) {
        return 'request_timeout'
    }
    return 'node_unreachable'
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
    $rootTask = Get-RootTaskFromDetail $ParentDetail $RequestedParentTaskId
    # The node is the authority for root requirement, lineage, acceptance
    # criteria and workspace identity. Never copy the parent prompt here:
    # doing so recursively nests compiled executor prompts across generations.
    $resumePrompt = "Resolve elon.resume_context.v1 for parent_task_id=$RequestedParentTaskId and root_task_id=$rootTask."
    return New-SupervisedTaskBody $parentProjectId $parentWorkspace $resumePrompt `
        'resume_original' $RequestedParentTaskId $rootTask $BodyCriteria $BodyImprovementPolicy
}

. (Join-Path $PSScriptRoot 'invoke-supervised-task-delta.ps1')
. (Join-Path $PSScriptRoot 'invoke-supervised-task-review.ps1')
. (Join-Path $PSScriptRoot 'invoke-supervised-task-workspace.ps1')
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
        if ($Compact) {
            Assert-NodeSupervisionCapability $nodeConnection $script:DeltaWaitCapability 'Compact Wait'
        }
        $terminalStatuses = @('done', 'finished', 'success', 'succeeded', 'failed', 'error', 'canceled', 'cancelled', 'interrupted', 'resume_required')
        $timer = [System.Diagnostics.Stopwatch]::StartNew()
        $detail = $null
        $status = ''
        $cursor = if ($Since -ge 0) { $Since } else { 0 }
        $initialCursor = $cursor
        $cursorEpoch = $ExpectedCursorEpoch
        $waitLimit = if ($PSBoundParameters.ContainsKey('Limit')) { $Limit } else { 25 }
        $collectedEvents = New-Object System.Collections.Generic.List[object]
        $seenEvents = @{}
        $sawCursorReset = $false
        $lastWaitError = $null
        do {
            try {
                # Poll only a small event window. The node computes the
                # supervision evidence summary from the complete journal.
                $remainingEventCapacity = $waitLimit - $collectedEvents.Count
                if ($remainingEventCapacity -le 0) { break }
                $pageLimit = [Math]::Max(1, [Math]::Min($waitLimit, $remainingEventCapacity))
                $remainingSeconds = [Math]::Max(1, [Math]::Min(15, [Math]::Ceiling($WaitSeconds - $timer.Elapsed.TotalSeconds)))
                $detail = Get-TaskDetail $nodeConnection $TaskId $pageLimit $cursor $cursorEpoch $remainingSeconds
                $record = Get-RecordFromDetail $detail
                $status = ([string](Get-ObjectField $record 'status')).ToLowerInvariant()
                if (Merge-TaskDeltaEvents $collectedEvents $seenEvents $detail) {
                    $sawCursorReset = $true
                }
                $returnedCursor = [int](Get-ObjectField $detail 'last_event_seq')
                $cursor = Resolve-MonotonicTaskCursor $cursor $returnedCursor `
                    ([bool](Get-ObjectField $detail 'cursor_reset')) `
                    ([int](Get-ObjectField $detail 'resume_cursor'))
                $returnedEpoch = [string](Get-ObjectField $detail 'cursor_epoch')
                if (-not [string]::IsNullOrWhiteSpace($returnedEpoch)) { $cursorEpoch = $returnedEpoch }
                if ($terminalStatuses -contains $status -or $status -eq 'waiting_approval') { break }
                if ($collectedEvents.Count -ge $waitLimit) { break }
                if ((Get-ObjectField $detail 'has_more') -eq $true) { continue }
            } catch {
                $lastWaitError = $_.Exception
                if ($timer.Elapsed.TotalSeconds -ge $WaitSeconds) {
                    $reason = Get-WaitFailureCode $_.Exception
                    throw "$reason`: Compact Wait exhausted its WaitSeconds boundary."
                }
            }
            $remainingSleepMs = [Math]::Floor(($WaitSeconds - $timer.Elapsed.TotalSeconds) * 1000)
            if ($remainingSleepMs -gt 0) { Start-Sleep -Milliseconds ([Math]::Min(2000, $remainingSleepMs)) }
        } while ($timer.Elapsed.TotalSeconds -lt $WaitSeconds)
        if ($null -eq $detail -and $null -ne $lastWaitError) {
            $reason = Get-WaitFailureCode $lastWaitError
            throw "$reason`: Compact Wait exhausted its WaitSeconds boundary."
        }
        $isTerminal = $terminalStatuses -contains $status
        $resultDetail = if ($Compact) {
            Select-TaskDeltaChanges `
                (Convert-ToCompactTaskDetail $detail ($collectedEvents.ToArray()) $isTerminal) `
                $ExpectedStateDigest $ExpectedEvidenceDigest
        } else { $detail }
        Convert-ToJsonResult ([ordered]@{
            ok = $true; action = 'Wait'; protocol = $script:SupervisionProtocol
            node_url = $nodeConnection.BaseUrl; task_id = $TaskId; status = $status
            since = $(if ($Since -ge 0) { $Since } else { 0 }); limit = $waitLimit
            next_cursor = $cursor; detail = $resultDetail
            cursor_reset = [bool]($sawCursorReset -or (Get-ObjectField $detail 'cursor_reset'))
            requested_cursor = Get-ObjectField $detail 'requested_cursor'
            old_cursor = Get-ObjectField $detail 'old_cursor'
            new_cursor = Get-ObjectField $detail 'new_cursor'
            resume_cursor = Get-ObjectField $detail 'resume_cursor'
            cursor_epoch = Get-ObjectField $detail 'cursor_epoch'
            requested_cursor_epoch = Get-ObjectField $detail 'requested_cursor_epoch'
            previous_cursor_epoch = Get-ObjectField $detail 'previous_cursor_epoch'
            sidecar_update_epoch = Get-ObjectField $detail 'sidecar_update_epoch'
            delta_from = $initialCursor
            delta_to = $cursor
            delta_event_count = $collectedEvents.Count
            state_digest = $(if ($Compact) { Get-ObjectField $resultDetail 'state_digest' } else { $null })
            delta_schema = $(if ($Compact) { $script:TaskDeltaSchema } else { $null })
        })
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
}
