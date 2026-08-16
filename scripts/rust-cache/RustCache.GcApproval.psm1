Import-Module "$PSScriptRoot\RustCache.Paths.psm1" -DisableNameChecking
Import-Module "$PSScriptRoot\RustCache.Inventory.psm1" -DisableNameChecking

function Get-RustCacheSha256Text {
    param([Parameter(Mandatory)][AllowEmptyString()][string]$Text)

    $sha256 = [System.Security.Cryptography.SHA256]::Create()
    try {
        $bytes = [System.Text.Encoding]::UTF8.GetBytes($Text)
        return ([BitConverter]::ToString($sha256.ComputeHash($bytes))).Replace("-", "").ToLowerInvariant()
    } finally {
        $sha256.Dispose()
    }
}

function Assert-RustCacheGcRequestId {
    param([Parameter(Mandatory)][string]$RequestId)
    if ($RequestId -notmatch '^[0-9a-f]{32}$') {
        throw "Rust cache GC request ID must be 32 lowercase hexadecimal characters."
    }
}

function Assert-RustCacheGcNodeId {
    param([Parameter(Mandatory)][string]$NodeId)
    if ($NodeId.Length -gt 160 -or $NodeId -notmatch '^[A-Za-z0-9][A-Za-z0-9._:-]*$') {
        throw "Rust cache GC node ID is invalid."
    }
}

function Get-RustCacheGcPlanPath {
    param(
        [Parameter(Mandatory)][string]$CacheRoot,
        [Parameter(Mandatory)][string]$RequestId
    )
    Assert-RustCacheGcRequestId -RequestId $RequestId
    return Join-Path $CacheRoot "reports\gc\plans\$RequestId.json"
}

function Get-RustCacheGcReceiptPath {
    param(
        [Parameter(Mandatory)][string]$CacheRoot,
        [Parameter(Mandatory)][string]$RequestId
    )
    Assert-RustCacheGcRequestId -RequestId $RequestId
    return Join-Path $CacheRoot "reports\gc\receipts\$RequestId.json"
}

function Get-RustCacheGcPlanCore {
    param([Parameter(Mandatory)]$Plan)

    $actions = @($Plan.actions | Sort-Object action_id | ForEach-Object {
        [ordered]@{
            action_id = [string]$_.action_id
            path = [string]$_.path
            reason = [string]$_.reason
            size_bytes = [int64]$_.size_bytes
            project_id = [string]$_.project_id
            domain = [string]$_.domain
            cache_scope = [string]$_.cache_scope
            last_used_utc = [string]$_.last_used_utc
        }
    })
    return [ordered]@{
        schema = "elon.rust_cache.gc_plan.v1"
        request_id = [string]$Plan.request_id
        plan_id = [string]$Plan.plan_id
        node_id = [string]$Plan.node_id
        generated_at_utc = [string]$Plan.generated_at_utc
        expires_at_utc = [string]$Plan.expires_at_utc
        cache_root = [string]$Plan.cache_root
        cache_root_sha256 = [string]$Plan.cache_root_sha256
        repo_root = [string]$Plan.repo_root
        options = [ordered]@{
            force_aged = [bool]$Plan.options.force_aged
            workspace_only = [bool]$Plan.options.workspace_only
            recover_missing_workspaces = [bool]$Plan.options.recover_missing_workspaces
            shared_aliases_only = [bool]$Plan.options.shared_aliases_only
        }
        approval_requirements = [ordered]@{
            exact_action_set = $true
            active_writer_count_unchanged = $true
            local_rescan_required = $true
        }
        active_writer_count_at_plan = [int]$Plan.active_writer_count_at_plan
        actions = $actions
    }
}

function Get-RustCacheGcPlanDigest {
    param([Parameter(Mandatory)]$Plan)
    $core = Get-RustCacheGcPlanCore -Plan $Plan
    return Get-RustCacheSha256Text -Text ($core | ConvertTo-Json -Compress -Depth 12)
}

function Get-RustCacheGcPlanSummary {
    param([Parameter(Mandatory)]$Plan)

    [int64]$reclaimBytes = 0
    $reasonCounts = @{}
    foreach ($action in @($Plan.actions)) {
        $reclaimBytes += [int64]$action.size_bytes
        $reason = [string]$action.reason
        if (-not $reasonCounts.ContainsKey($reason)) { $reasonCounts[$reason] = 0 }
        $reasonCounts[$reason]++
    }
    $reasons = @($reasonCounts.Keys | Sort-Object | ForEach-Object {
        [ordered]@{ reason = $_; count = $reasonCounts[$_] }
    })
    return [ordered]@{
        schema = "elon.rust_cache.gc_plan_summary.v1"
        request_id = [string]$Plan.request_id
        plan_id = [string]$Plan.plan_id
        plan_digest = [string]$Plan.plan_digest
        node_id = [string]$Plan.node_id
        generated_at_utc = [string]$Plan.generated_at_utc
        expires_at_utc = [string]$Plan.expires_at_utc
        options = [ordered]@{
            force_aged = [bool]$Plan.options.force_aged
            workspace_only = [bool]$Plan.options.workspace_only
            recover_missing_workspaces = [bool]$Plan.options.recover_missing_workspaces
            shared_aliases_only = [bool]$Plan.options.shared_aliases_only
        }
        action_count = @($Plan.actions).Count
        reclaim_bytes = $reclaimBytes
        active_writer_count = [int]$Plan.active_writer_count_at_plan
        reasons = $reasons
        security = [ordered]@{
            absolute_paths_included = $false
            secrets_included = $false
            destructive_actions_authorized = $false
            approval_binds_plan_digest = $true
            target_rescan_required = $true
        }
    }
}

function Write-RustCacheImmutableJson {
    param(
        [Parameter(Mandatory)][string]$Path,
        [Parameter(Mandatory)]$Value
    )

    $parent = Split-Path $Path -Parent
    New-Item -ItemType Directory -Force -Path $parent | Out-Null
    $payload = $Value | ConvertTo-Json -Depth 14
    $utf8 = New-Object System.Text.UTF8Encoding($false)
    $bytes = $utf8.GetBytes($payload)
    try {
        $stream = [System.IO.File]::Open($Path, [System.IO.FileMode]::CreateNew, [System.IO.FileAccess]::Write, [System.IO.FileShare]::Read)
        try {
            $stream.Write($bytes, 0, $bytes.Length)
            $stream.Flush($true)
        } finally {
            $stream.Dispose()
        }
    } catch [System.IO.IOException] {
        throw "Immutable Rust cache GC artifact already exists: $Path"
    }
}

function Read-RustCacheGcApprovalPlan {
    param(
        [Parameter(Mandatory)][string]$CacheRoot,
        [Parameter(Mandatory)][string]$RequestId
    )

    $path = Get-RustCacheGcPlanPath -CacheRoot $CacheRoot -RequestId $RequestId
    if (-not (Test-Path -LiteralPath $path -PathType Leaf)) {
        throw "Rust cache GC plan does not exist for request $RequestId."
    }
    $plan = Get-Content -Raw -LiteralPath $path -Encoding UTF8 | ConvertFrom-Json
    if (
        $plan.schema -ne "elon.rust_cache.gc_plan.v1" -or
        $plan.request_id -ne $RequestId -or
        $plan.plan_id -notmatch '^[0-9a-f]{32}$' -or
        $plan.plan_digest -notmatch '^[0-9a-f]{64}$'
    ) {
        throw "Rust cache GC plan contract is invalid."
    }
    $digest = Get-RustCacheGcPlanDigest -Plan $plan
    if ($digest -ne [string]$plan.plan_digest) {
        throw "Rust cache GC plan integrity check failed."
    }
    $resolvedRoot = [System.IO.Path]::GetFullPath($CacheRoot).TrimEnd('\', '/')
    $plannedRoot = [System.IO.Path]::GetFullPath([string]$plan.cache_root).TrimEnd('\', '/')
    if ($resolvedRoot -ne $plannedRoot) {
        throw "Rust cache GC plan belongs to another cache root."
    }
    return $plan
}

function New-RustCacheGcApprovalPlan {
    param(
        [string]$CacheRoot,
        [string]$RepoRoot,
        [Parameter(Mandatory)][string]$RequestId,
        [Parameter(Mandatory)][string]$NodeId,
        [switch]$ForceAged,
        [switch]$WorkspaceOnly,
        [switch]$RecoverMissingWorkspaces,
        [switch]$SharedAliasesOnly,
        [ValidateRange(5, 1440)][int]$ExpiresMinutes = 1440
    )

    Assert-RustCacheGcRequestId -RequestId $RequestId
    Assert-RustCacheGcNodeId -NodeId $NodeId
    $root = Resolve-RustCacheRoot -ExplicitRoot $CacheRoot -RepoRoot $RepoRoot
    $path = Get-RustCacheGcPlanPath -CacheRoot $root -RequestId $RequestId
    if (Test-Path -LiteralPath $path -PathType Leaf) {
        $existing = Read-RustCacheGcApprovalPlan -CacheRoot $root -RequestId $RequestId
        if ($existing.node_id -ne $NodeId) { throw "Rust cache GC request identity conflict." }
        return $existing
    }

    $report = Invoke-RustCacheGc -CacheRoot $root -RepoRoot $RepoRoot -ForceAged:$ForceAged -WorkspaceOnly:$WorkspaceOnly -RecoverMissingWorkspaces:$RecoverMissingWorkspaces -SharedAliasesOnly:$SharedAliasesOnly
    $generated = [DateTime]::UtcNow
    $actions = @($report.actions | Where-Object { $_.action -eq "would-delete" } | ForEach-Object {
        [ordered]@{
            action_id = [string]$_.action_id
            path = [string]$_.path
            reason = [string]$_.reason
            size_bytes = [int64]$_.size_bytes
            project_id = [string]$_.project_id
            domain = [string]$_.domain
            cache_scope = [string]$_.cache_scope
            last_used_utc = ([DateTime]$_.last_used_utc).ToUniversalTime().ToString("o")
        }
    })
    $core = [ordered]@{
        schema = "elon.rust_cache.gc_plan.v1"
        request_id = $RequestId
        plan_id = [Guid]::NewGuid().ToString("N")
        node_id = $NodeId
        generated_at_utc = $generated.ToString("o")
        expires_at_utc = $generated.AddMinutes($ExpiresMinutes).ToString("o")
        cache_root = $root
        cache_root_sha256 = Get-RustCacheSha256Text -Text ([System.IO.Path]::GetFullPath($root).ToLowerInvariant())
        repo_root = if ([string]::IsNullOrWhiteSpace($RepoRoot)) { "" } else { [System.IO.Path]::GetFullPath($RepoRoot) }
        options = [ordered]@{
            force_aged = [bool]$ForceAged
            workspace_only = [bool]$WorkspaceOnly
            recover_missing_workspaces = [bool]$RecoverMissingWorkspaces
            shared_aliases_only = [bool]$SharedAliasesOnly
        }
        approval_requirements = [ordered]@{
            exact_action_set = $true
            active_writer_count_unchanged = $true
            local_rescan_required = $true
        }
        active_writer_count_at_plan = @($report.active_build_processes).Count
        actions = @($actions | Sort-Object action_id)
    }
    $plan = [ordered]@{}
    foreach ($entry in $core.GetEnumerator()) { $plan[$entry.Key] = $entry.Value }
    $plan.plan_digest = Get-RustCacheGcPlanDigest -Plan $plan
    $plan.summary = Get-RustCacheGcPlanSummary -Plan $plan
    Write-RustCacheImmutableJson -Path $path -Value $plan
    return Read-RustCacheGcApprovalPlan -CacheRoot $root -RequestId $RequestId
}

function Invoke-RustCacheApprovedPlan {
    param(
        [string]$CacheRoot,
        [Parameter(Mandatory)][string]$RequestId,
        [Parameter(Mandatory)][string]$PlanId,
        [Parameter(Mandatory)][string]$PlanDigest,
        [string]$NodeId
    )

    $root = Resolve-RustCacheRoot -ExplicitRoot $CacheRoot
    $receiptPath = Get-RustCacheGcReceiptPath -CacheRoot $root -RequestId $RequestId
    if (Test-Path -LiteralPath $receiptPath -PathType Leaf) {
        $receipt = Get-Content -Raw -LiteralPath $receiptPath -Encoding UTF8 | ConvertFrom-Json
        if ($receipt.plan_id -ne $PlanId -or $receipt.plan_digest -ne $PlanDigest) {
            throw "Rust cache GC receipt identity conflict."
        }
        return $receipt
    }
    $plan = Read-RustCacheGcApprovalPlan -CacheRoot $root -RequestId $RequestId
    if ($plan.plan_id -ne $PlanId -or $plan.plan_digest -ne $PlanDigest) {
        throw "Rust cache GC approval does not match the immutable local plan."
    }
    if (-not [string]::IsNullOrWhiteSpace($NodeId) -and $plan.node_id -ne $NodeId) {
        throw "Rust cache GC approval belongs to another node."
    }
    if ([DateTime]::Parse([string]$plan.expires_at_utc).ToUniversalTime() -le [DateTime]::UtcNow) {
        throw "Rust cache GC approval plan expired; create and approve a new plan."
    }

    $report = Invoke-RustCacheGc `
        -CacheRoot $root `
        -RepoRoot ([string]$plan.repo_root) `
        -Apply `
        -ForceAged:([bool]$plan.options.force_aged) `
        -WorkspaceOnly:([bool]$plan.options.workspace_only) `
        -RecoverMissingWorkspaces:([bool]$plan.options.recover_missing_workspaces) `
        -SharedAliasesOnly:([bool]$plan.options.shared_aliases_only) `
        -ApprovedActions @($plan.actions) `
        -ExpectedActiveBuildCount ([int]$plan.active_writer_count_at_plan)
    $actionResults = @($report.actions | Where-Object { $_.action -eq "delete" -or $_.reason -eq "lock-appeared" } | ForEach-Object {
        [ordered]@{
            action_id = [string]$_.action_id
            status = if ($_.action -eq "delete") { "removed" } else { "preserved" }
            reason = [string]$_.reason
            size_bytes = [int64]$_.size_bytes
        }
    })
    $removed = @($actionResults | Where-Object { $_.status -eq "removed" })
    [int64]$reclaimedBytes = 0
    foreach ($action in $removed) { $reclaimedBytes += [int64]$action.size_bytes }
    $receipt = [ordered]@{
        schema = "elon.rust_cache.gc_receipt.v1"
        request_id = $RequestId
        plan_id = $PlanId
        plan_digest = $PlanDigest
        node_id = [string]$plan.node_id
        status = if ($removed.Count -eq @($plan.actions).Count) { "completed" } else { "partial" }
        completed_at_utc = [DateTime]::UtcNow.ToString("o")
        approved_action_count = @($plan.actions).Count
        removed_action_count = $removed.Count
        reclaimed_bytes = $reclaimedBytes
        action_results = $actionResults
        security = [ordered]@{
            absolute_paths_included = $false
            secrets_included = $false
            execution_bound_to_plan_digest = $true
            local_rescan_completed = $true
        }
    }
    Write-RustCacheImmutableJson -Path $receiptPath -Value $receipt
    return Get-Content -Raw -LiteralPath $receiptPath -Encoding UTF8 | ConvertFrom-Json
}

Export-ModuleMember -Function New-RustCacheGcApprovalPlan, Read-RustCacheGcApprovalPlan, Get-RustCacheGcPlanSummary, Invoke-RustCacheApprovedPlan
