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
    [pscustomobject]@{
        ProjectId = $projectId; RequestedPath = $requested; AuthorizedPath = $resolved
        ResolvedPath = $resolved
        Corrected = (-not [string]::IsNullOrWhiteSpace($requested) -and $requested -ne $resolved)
        RuntimePermission = $permission
    }
}

function Submit-Body {
    param([object]$Connection, [object]$Body, [string]$ResultAction, [object]$PathResolution = $null)
    $submission = Invoke-IdempotentNodePost $Connection '/api/local-tasks' $Body
    $response = $submission.Response
    $Connection = $submission.Connection
    Convert-ToJsonResult ([ordered]@{
        ok = $true; action = $ResultAction; protocol = $script:SupervisionProtocol
        node_url = $Connection.BaseUrl; node_version = $Connection.Version
        task_id = [string](Get-ObjectField $response 'task_id')
        path_resolution = $PathResolution; response = $response
    })
}
