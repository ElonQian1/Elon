Set-StrictMode -Version 2.0

$script:AiTaskRustScratchSchema = 'elon.ai_task_rust_scratch.v1'
$script:AiTaskRustScratchRootName = 'elon-ai-task-rust-v1'
$script:AiTaskRustScratchMarkerName = '.elon-ai-task-rust-scratch.json'

function Get-AiTaskRustScratchRoot {
    param([string]$ExplicitRoot = '')

    $candidate = if (-not [string]::IsNullOrWhiteSpace($ExplicitRoot)) {
        $ExplicitRoot
    } elseif (-not [string]::IsNullOrWhiteSpace($env:ELON_AI_TASK_RUST_SCRATCH_ROOT)) {
        $env:ELON_AI_TASK_RUST_SCRATCH_ROOT
    } else {
        Join-Path ([System.IO.Path]::GetTempPath()) $script:AiTaskRustScratchRootName
    }
    if (-not [System.IO.Path]::IsPathRooted($candidate)) {
        throw 'AI task Rust scratch root must be absolute.'
    }
    $full = [System.IO.Path]::GetFullPath($candidate).TrimEnd('\', '/')
    $temp = [System.IO.Path]::GetFullPath([System.IO.Path]::GetTempPath()).TrimEnd('\', '/')
    $tempPrefix = $temp + [System.IO.Path]::DirectorySeparatorChar
    if (-not $full.StartsWith($tempPrefix, [System.StringComparison]::OrdinalIgnoreCase)) {
        throw "AI task Rust scratch root must remain under the operating-system temp root: $full"
    }
    if ($full -eq $temp) {
        throw 'AI task Rust scratch root cannot be the operating-system temp root itself.'
    }
    return $full
}

function Get-AiTaskNormalizedPath {
    param([Parameter(Mandatory = $true)][string]$Path)
    return [System.IO.Path]::GetFullPath($Path).TrimEnd('\', '/').Replace('\', '/')
}

function Assert-AiTaskPathUnderRoot {
    param(
        [Parameter(Mandatory = $true)][string]$Path,
        [Parameter(Mandatory = $true)][string]$Root
    )
    $pathFull = [System.IO.Path]::GetFullPath($Path).TrimEnd('\', '/')
    $rootFull = [System.IO.Path]::GetFullPath($Root).TrimEnd('\', '/')
    $prefix = $rootFull + [System.IO.Path]::DirectorySeparatorChar
    if ($pathFull -eq $rootFull -or
        -not $pathFull.StartsWith($prefix, [System.StringComparison]::OrdinalIgnoreCase)) {
        throw "AI task external artifact escaped its fixed root: $pathFull"
    }
    return $pathFull
}

function Assert-AiTaskScratchRootPathChain {
    param([Parameter(Mandatory = $true)][string]$Root)

    $rootFull = [System.IO.Path]::GetFullPath($Root).TrimEnd('\', '/')
    $tempFull = [System.IO.Path]::GetFullPath([System.IO.Path]::GetTempPath()).TrimEnd('\', '/')
    $tempPrefix = $tempFull + [System.IO.Path]::DirectorySeparatorChar
    if (-not $rootFull.StartsWith($tempPrefix, [System.StringComparison]::OrdinalIgnoreCase)) {
        throw 'AI task Rust scratch root path chain escaped the operating-system temp root.'
    }
    $relative = $rootFull.Substring($tempPrefix.Length)
    $current = $tempFull
    foreach ($segment in $relative -split '[\\/]') {
        if ([string]::IsNullOrWhiteSpace($segment)) { continue }
        $current = Join-Path $current $segment
        if (-not (Test-Path -LiteralPath $current)) { continue }
        $item = Get-Item -LiteralPath $current -Force -ErrorAction Stop
        if (($item.Attributes -band [System.IO.FileAttributes]::ReparsePoint) -ne 0) {
            throw "AI task Rust scratch root path chain contains a reparse point: $current"
        }
    }
}

function Assert-AiTaskNoReparsePoints {
    param([Parameter(Mandatory = $true)][string]$Path)

    $pending = [System.Collections.Generic.Stack[string]]::new()
    $pending.Push([System.IO.Path]::GetFullPath($Path))
    while ($pending.Count -gt 0) {
        $current = $pending.Pop()
        $item = Get-Item -LiteralPath $current -Force -ErrorAction Stop
        if (($item.Attributes -band [System.IO.FileAttributes]::ReparsePoint) -ne 0) {
            throw "AI task Rust scratch contains a reparse point: $current"
        }
        if (-not $item.PSIsContainer) { continue }
        foreach ($child in Get-ChildItem -LiteralPath $current -Force -ErrorAction Stop) {
            if (($child.Attributes -band [System.IO.FileAttributes]::ReparsePoint) -ne 0) {
                throw "AI task Rust scratch contains a reparse point: $($child.FullName)"
            }
            if ($child.PSIsContainer) { $pending.Push($child.FullName) }
        }
    }
}

function Read-AiTaskRustScratchMarker {
    param([Parameter(Mandatory = $true)][string]$ScratchPath)

    $markerPath = Join-Path $ScratchPath $script:AiTaskRustScratchMarkerName
    if (-not (Test-Path -LiteralPath $markerPath -PathType Leaf)) {
        throw "AI task Rust scratch marker is missing: $ScratchPath"
    }
    $markerItem = Get-Item -LiteralPath $markerPath -Force
    if (($markerItem.Attributes -band [System.IO.FileAttributes]::ReparsePoint) -ne 0) {
        throw "AI task Rust scratch marker cannot be a reparse point: $markerPath"
    }
    try {
        return [System.IO.File]::ReadAllText($markerPath, [System.Text.UTF8Encoding]::new($false, $true)) |
            ConvertFrom-Json -ErrorAction Stop
    } catch {
        throw "AI task Rust scratch marker is invalid JSON: $markerPath"
    }
}

function Assert-AiTaskRustScratchMarker {
    param(
        [Parameter(Mandatory = $true)]$Marker,
        [Parameter(Mandatory = $true)][string]$ScratchPath,
        [Parameter(Mandatory = $true)][string]$ContractId,
        [Parameter(Mandatory = $true)]$ValidatedContract
    )

    $properties = @($Marker.PSObject.Properties.Name | Sort-Object)
    $expectedProperties = @(
        'branch', 'contract_id', 'created_at_utc', 'nonce', 'purpose', 'schema',
        'scratch_path', 'worktree'
    ) | Sort-Object
    if (($properties -join "`n") -ne ($expectedProperties -join "`n")) {
        throw 'AI task Rust scratch marker fields are not exact.'
    }
    $normalizedScratch = Get-AiTaskNormalizedPath $ScratchPath
    if ([string]$Marker.schema -ne $script:AiTaskRustScratchSchema -or
        [string]$Marker.contract_id -ne $ContractId -or
        [string]$Marker.worktree -ne [string]$ValidatedContract.worktree -or
        [string]$Marker.branch -ne [string]$ValidatedContract.branch -or
        [string]$Marker.scratch_path -ne $normalizedScratch -or
        [string]$Marker.purpose -notmatch '^[A-Za-z0-9][A-Za-z0-9._-]{0,63}$' -or
        [string]$Marker.nonce -notmatch '^[0-9a-f]{32}$') {
        throw 'AI task Rust scratch marker identity drifted.'
    }
    $timestamp = [DateTimeOffset]::MinValue
    if (-not [DateTimeOffset]::TryParseExact(
        [string]$Marker.created_at_utc,
        'o',
        [System.Globalization.CultureInfo]::InvariantCulture,
        [System.Globalization.DateTimeStyles]::RoundtripKind,
        [ref]$timestamp
    )) {
        throw 'AI task Rust scratch marker timestamp is invalid.'
    }
    $leaf = Split-Path -Leaf $ScratchPath
    $expectedPrefix = $ContractId.Substring(0, 16) + '-' + [string]$Marker.purpose + '-'
    if (-not $leaf.Equals($expectedPrefix + [string]$Marker.nonce, [System.StringComparison]::Ordinal)) {
        throw 'AI task Rust scratch directory name does not match its marker.'
    }
}

function New-AiTaskRustScratch {
    param(
        [Parameter(Mandatory = $true)][string]$RepoPath,
        [Parameter(Mandatory = $true)][string]$ContractId,
        [Parameter(Mandatory = $true)][string]$Purpose,
        [string]$ScratchRoot = ''
    )

    if ($Purpose -notmatch '^[A-Za-z0-9][A-Za-z0-9._-]{0,63}$') {
        throw 'Rust scratch purpose must use 1-64 ASCII letters, digits, dot, underscore, or hyphen.'
    }
    if (-not (Get-Command Assert-AiTaskFinishContract -ErrorAction SilentlyContinue)) {
        throw 'Assert-AiTaskFinishContract must be loaded before allocating task scratch.'
    }
    $validated = Assert-AiTaskFinishContract -RepoPath $RepoPath -ContractId $ContractId
    $root = Get-AiTaskRustScratchRoot -ExplicitRoot $ScratchRoot
    Assert-AiTaskScratchRootPathChain -Root $root
    [System.IO.Directory]::CreateDirectory($root) | Out-Null
    Assert-AiTaskScratchRootPathChain -Root $root
    $rootItem = Get-Item -LiteralPath $root -Force
    if (($rootItem.Attributes -band [System.IO.FileAttributes]::ReparsePoint) -ne 0) {
        throw "AI task Rust scratch root cannot be a reparse point: $root"
    }

    $nonce = [Guid]::NewGuid().ToString('N')
    $leaf = "$($ContractId.Substring(0, 16))-$Purpose-$nonce"
    $scratch = Assert-AiTaskPathUnderRoot -Path (Join-Path $root $leaf) -Root $root
    if (Test-Path -LiteralPath $scratch) {
        throw "AI task Rust scratch already exists: $scratch"
    }
    [System.IO.Directory]::CreateDirectory($scratch) | Out-Null
    try {
        $marker = [ordered]@{
            schema = $script:AiTaskRustScratchSchema
            contract_id = $ContractId
            worktree = [string]$validated.worktree
            branch = [string]$validated.branch
            purpose = $Purpose
            nonce = $nonce
            scratch_path = Get-AiTaskNormalizedPath $scratch
            created_at_utc = [DateTime]::UtcNow.ToString('o')
        }
        $markerPath = Join-Path $scratch $script:AiTaskRustScratchMarkerName
        $encoding = [System.Text.UTF8Encoding]::new($false)
        [System.IO.File]::WriteAllText($markerPath, ($marker | ConvertTo-Json -Depth 4 -Compress), $encoding)
        $cache = Join-Path $scratch 'cache'
        $target = Join-Path $scratch 'target'
        [System.IO.Directory]::CreateDirectory($cache) | Out-Null
        [System.IO.Directory]::CreateDirectory($target) | Out-Null
        return [pscustomobject]@{
            schema = $script:AiTaskRustScratchSchema
            scratch_path = $scratch
            cache_root = $cache
            target_dir = $target
            contract_id = $ContractId
            purpose = $Purpose
        }
    } catch {
        Remove-Item -LiteralPath $scratch -Recurse -Force -ErrorAction SilentlyContinue
        throw
    }
}

function Clear-AiTaskRustScratch {
    param(
        [Parameter(Mandatory = $true)][string]$RepoPath,
        [Parameter(Mandatory = $true)][string]$ContractId,
        [Parameter(Mandatory = $true)]$ValidatedContract,
        [string]$ScratchRoot = '',
        [switch]$Skip
    )

    if ($Skip) {
        Write-Host 'EXTERNAL_RUST_SCRATCH_CLEANUP=skipped_by_option'
        return [pscustomobject]@{ removed_count = 0; skipped = $true }
    }
    if ($ContractId -notmatch '^[0-9a-f]{64}$') {
        throw 'TaskContract must be a SHA-256 id.'
    }
    $repoIdentity = Get-AiTaskNormalizedPath $RepoPath
    if (-not $repoIdentity.Equals(
        [string]$ValidatedContract.worktree,
        [System.StringComparison]::OrdinalIgnoreCase
    )) {
        throw 'AI task Rust scratch cleanup worktree identity mismatch.'
    }
    $root = Get-AiTaskRustScratchRoot -ExplicitRoot $ScratchRoot
    if (-not (Test-Path -LiteralPath $root -PathType Container)) {
        Write-Host 'EXTERNAL_RUST_SCRATCH_CLEANUP=none'
        return [pscustomobject]@{ removed_count = 0; skipped = $false }
    }
    Assert-AiTaskScratchRootPathChain -Root $root
    $rootItem = Get-Item -LiteralPath $root -Force
    if (($rootItem.Attributes -band [System.IO.FileAttributes]::ReparsePoint) -ne 0) {
        throw "AI task Rust scratch root cannot be a reparse point: $root"
    }

    $prefix = $ContractId.Substring(0, 16) + '-'
    $candidates = @(Get-ChildItem -LiteralPath $root -Directory -Force | Where-Object {
        $_.Name.StartsWith($prefix, [System.StringComparison]::Ordinal)
    } | Sort-Object Name)
    $validatedCandidates = @()
    foreach ($candidate in $candidates) {
        $path = Assert-AiTaskPathUnderRoot -Path $candidate.FullName -Root $root
        $marker = Read-AiTaskRustScratchMarker -ScratchPath $path
        Assert-AiTaskRustScratchMarker `
            -Marker $marker `
            -ScratchPath $path `
            -ContractId $ContractId `
            -ValidatedContract $ValidatedContract
        $topLevel = @(Get-ChildItem -LiteralPath $path -Force | ForEach-Object { $_.Name } | Sort-Object)
        $expected = @($script:AiTaskRustScratchMarkerName, 'cache', 'target') | Sort-Object
        if (($topLevel -join "`n") -ne ($expected -join "`n")) {
            throw "AI task Rust scratch contains unknown top-level members: $path"
        }
        Assert-AiTaskNoReparsePoints -Path $path
        $validatedCandidates += [pscustomobject]@{ Path = $path; Name = $candidate.Name }
    }
    foreach ($candidate in $validatedCandidates) {
        Assert-AiTaskNoReparsePoints -Path $candidate.Path
        Remove-Item -LiteralPath $candidate.Path -Recurse -Force -ErrorAction Stop
        Write-Host "EXTERNAL_RUST_SCRATCH_REMOVED=$($candidate.Name)"
    }
    $removed = $validatedCandidates.Count
    Write-Host "EXTERNAL_RUST_SCRATCH_CLEANUP=removed:$removed"
    return [pscustomobject]@{ removed_count = $removed; skipped = $false }
}
