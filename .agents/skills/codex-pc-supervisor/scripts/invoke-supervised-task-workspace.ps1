function Resolve-SubmitProjectWorkspace {
    param([object]$Connection, [string]$RequestedProjectId, [string]$RequestedWorkspacePath)
    $projectId = $RequestedProjectId.Trim()
    $requested = if ([string]::IsNullOrWhiteSpace($RequestedWorkspacePath)) { '' } else {
        [System.IO.Path]::GetFullPath($RequestedWorkspacePath)
    }
    $payload = Invoke-NodeApi $Connection 'Get' (Get-CloudProjectsPath $true)
    $projects = @((Get-ObjectField $payload 'projects'))
    $matches = @($projects | Where-Object { [string](Get-ObjectField $_ 'id') -eq $projectId })
    if ($matches.Count -ne 1) {
        throw "WORKSPACE_IDENTITY_MISMATCH: project_id '$projectId' did not resolve to exactly one node project."
    }
    $project = $matches[0]
    $boundNode = [string](Get-ObjectField $project 'node_id')
    $currentNode = [string](Get-ObjectField $payload 'node_id')
    $authorized = [string](Get-ObjectField $project 'workspace_path')
    $permission = [string](Get-ObjectField $project 'runtime_permission')
    if ([string]::IsNullOrWhiteSpace($boundNode) -or $boundNode -ne $currentNode -or
        [string]::IsNullOrWhiteSpace($authorized)) {
        throw "WORKSPACE_IDENTITY_MISMATCH: project_id '$projectId' is not authoritatively bound to this node and workspace."
    }
    if ($permission -notin @('full_access', 'danger_full_access')) {
        throw "PROJECT_FULL_ACCESS_DISABLED: project_id '$projectId' is explicitly configured as '$permission'."
    }
    $resolved = [System.IO.Path]::GetFullPath($authorized)
    if (-not (Test-Path -LiteralPath $resolved -PathType Container)) {
        throw "WORKSPACE_IDENTITY_MISMATCH: authoritative workspace_path is unavailable: $resolved"
    }
    $grantState = Ensure-SubmitFullAccessGrant $Connection $projectId $resolved
    [pscustomobject]@{
        ProjectId = $projectId; RequestedPath = $requested; AuthorizedPath = $resolved
        ResolvedPath = $resolved
        Corrected = (-not [string]::IsNullOrWhiteSpace($requested) -and $requested -ne $resolved)
        RuntimePermission = $permission
        FullAccessGrant = $grantState
    }
}

function Convert-ToComparableWorkspacePath {
    param([string]$Value)
    if ([string]::IsNullOrWhiteSpace($Value)) { return '' }
    $path = [System.IO.Path]::GetFullPath($Value.Trim())
    if ($path.StartsWith('\\?\')) { $path = $path.Substring(4) }
    return $path.TrimEnd('\', '/').ToLowerInvariant()
}

function Test-EquivalentProjectId {
    param([string]$Left, [string]$Right)
    $canonical = {
        param([string]$Value)
        $value = $Value.Trim().ToLowerInvariant()
        if ($value -eq 'elon-project') { return 'elon-self' }
        return $value
    }
    return (& $canonical $Left) -eq (& $canonical $Right)
}

function New-FullAccessGrantBody {
    param([string]$ProjectId, [string]$WorkspacePath)
    [ordered]@{
        project_id = $ProjectId.Trim()
        workspace_path = [System.IO.Path]::GetFullPath($WorkspacePath)
        confirm_full_access = $true
    }
}

function Ensure-SubmitFullAccessGrant {
    param([object]$Connection, [string]$ProjectId, [string]$WorkspacePath)
    $expectedPath = Convert-ToComparableWorkspacePath $WorkspacePath
    $payload = Invoke-NodeApi $Connection 'Get' '/api/full-access/grants'
    $matched = @((Get-ObjectField $payload 'grants') | Where-Object {
        (Test-EquivalentProjectId ([string](Get-ObjectField $_ 'project_id')) $ProjectId) -and
        (Convert-ToComparableWorkspacePath ([string](Get-ObjectField $_ 'workspace_path'))) -eq $expectedPath
    })
    if ($matched.Count -gt 0) { return 'already_granted' }

    $response = Invoke-NodeApi $Connection 'Post' '/api/full-access/grants' `
        (New-FullAccessGrantBody $ProjectId $WorkspacePath)
    $grant = Get-ObjectField $response 'grant'
    if ($null -eq $grant -or
        -not (Test-EquivalentProjectId ([string](Get-ObjectField $grant 'project_id')) $ProjectId) -or
        (Convert-ToComparableWorkspacePath ([string](Get-ObjectField $grant 'workspace_path'))) -ne $expectedPath) {
        throw 'PROJECT_FULL_ACCESS_GRANT_MISMATCH: node did not persist the exact authoritative project workspace grant.'
    }
    return 'granted'
}

function Submit-Body {
    param([object]$Connection, [object]$Body, [string]$ResultAction, [object]$PathResolution = $null)
    $submitWatch = [System.Diagnostics.Stopwatch]::StartNew()
    $submission = Invoke-IdempotentNodePost $Connection '/api/local-tasks' $Body
    $submitWatch.Stop()
    $response = $submission.Response
    $Connection = $submission.Connection
    Convert-ToJsonResult ([ordered]@{
        ok = $true; action = $ResultAction; protocol = $script:SupervisionProtocol
        node_url = $Connection.BaseUrl; node_version = $Connection.Version
        task_id = [string](Get-ObjectField $response 'task_id')
        submit_ms = $submitWatch.ElapsedMilliseconds
        path_resolution = $PathResolution; response = $response
    })
}
