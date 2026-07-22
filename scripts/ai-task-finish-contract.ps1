function Get-AiTaskFinishContractRoot {
    $base = if (-not [string]::IsNullOrWhiteSpace($env:LOCALAPPDATA)) {
        Join-Path $env:LOCALAPPDATA 'ElonNode'
    } else {
        Join-Path ([System.IO.Path]::GetTempPath()) 'ElonNode'
    }
    Join-Path $base 'ai-finish-contracts-v1'
}

function Get-AiTaskSha256 {
    param([Parameter(Mandatory = $true)][byte[]]$Bytes)
    $sha = [System.Security.Cryptography.SHA256]::Create()
    try { $digest = $sha.ComputeHash($Bytes) } finally { $sha.Dispose() }
    -join ($digest | ForEach-Object { $_.ToString('x2') })
}

function Get-AiTaskGitValue {
    param([string]$RepoPath, [string[]]$Arguments)
    $value = @(& git -C $RepoPath @Arguments 2>&1)
    if ($LASTEXITCODE -ne 0) { throw "git $($Arguments -join ' ') failed: $($value -join ' ')" }
    (($value | ForEach-Object { [string]$_ }) -join "`n").Trim()
}

function Get-AiTaskWorktreeLeaseReason {
    param([Parameter(Mandatory = $true)][string]$RepoPath)
    $target = [System.IO.Path]::GetFullPath($RepoPath).TrimEnd('\', '/').Replace('\', '/')
    $lines = @(& git -C $RepoPath worktree list --porcelain 2>&1)
    if ($LASTEXITCODE -ne 0) { throw "git worktree list --porcelain failed: $($lines -join ' ')" }
    $currentPath = $null
    foreach ($raw in $lines) {
        $line = [string]$raw
        if ($line.StartsWith('worktree ')) {
            $currentPath = [System.IO.Path]::GetFullPath($line.Substring(9)).TrimEnd('\', '/').Replace('\', '/')
            continue
        }
        if ($currentPath -eq $target -and $line.StartsWith('locked')) {
            return $line.Substring(6).Trim()
        }
    }
    return $null
}

function Get-AiTaskPlatformIdentity {
    param(
        [Parameter(Mandatory = $true)][string]$RepoPath,
        [Parameter(Mandatory = $true)][string]$Branch,
        [switch]$AllowReleasedLease,
        [string]$ExpectedRoot = ''
    )
    if ($Branch -notlike 'ai/session/*') { return $null }
    $parts = $Branch -split '/'
    if ($parts.Count -ne 4 -or $parts[0] -ne 'ai' -or $parts[1] -ne 'session') {
        throw 'Platform session branch shape is not trusted.'
    }
    $full = [System.IO.Path]::GetFullPath($RepoPath).TrimEnd('\', '/')
    $conversation = Split-Path -Leaf $full
    $project = Split-Path -Leaf (Split-Path -Parent $full)
    $marker = Split-Path -Leaf (Split-Path -Parent (Split-Path -Parent $full))
    if ($marker -ne 'conversation-worktrees' -or
        -not $project.Equals($parts[2], [System.StringComparison]::OrdinalIgnoreCase) -or
        -not $conversation.Equals($parts[3], [System.StringComparison]::OrdinalIgnoreCase)) {
        throw 'Platform session path, project, conversation, and branch identity differ.'
    }
    $lease = Get-AiTaskWorktreeLeaseReason -RepoPath $RepoPath
    if ([string]::IsNullOrWhiteSpace($ExpectedRoot)) {
        if ([string]::IsNullOrWhiteSpace($lease) -or $lease -notmatch '^elon-supervision:([A-Za-z0-9._-]{1,160})$') {
            throw 'Platform session preflight requires an exact elon-supervision:<root> lease.'
        }
        $ExpectedRoot = $Matches[1]
    } else {
        $expectedLease = "elon-supervision:$ExpectedRoot"
        if (-not [string]::IsNullOrWhiteSpace($lease) -and $lease -ne $expectedLease) {
            throw "Platform session lease identity mismatch: $lease"
        }
        if ([string]::IsNullOrWhiteSpace($lease) -and -not $AllowReleasedLease) {
            throw 'Platform session root lease disappeared before validation.'
        }
    }
    [pscustomobject]@{
        Provenance = 'elon.conversation_worktree.v1'
        RootTaskId = $ExpectedRoot
        LeaseReason = "elon-supervision:$ExpectedRoot"
        GitCommonDir = Get-AiTaskGitValue $RepoPath @('rev-parse', '--path-format=absolute', '--git-common-dir')
    }
}

function New-AiTaskFinishContract {
    param([Parameter(Mandatory = $true)][string]$RepoPath)
    $worktree = [System.IO.Path]::GetFullPath($RepoPath).TrimEnd('\', '/')
    $branch = Get-AiTaskGitValue $worktree @('branch', '--show-current')
    $baseCommit = Get-AiTaskGitValue $worktree @('rev-parse', 'HEAD^{commit}')
    $origin = Get-AiTaskGitValue $worktree @('remote', 'get-url', 'origin')
    $platform = Get-AiTaskPlatformIdentity -RepoPath $worktree -Branch $branch
    $payload = [ordered]@{
        schema = 'elon.ai_finish_contract.v1'
        worktree = $worktree.Replace('\', '/')
        branch = $branch
        baseCommit = $baseCommit
        origin = $origin
        issuedAtUtc = [DateTime]::UtcNow.ToString('o')
        nonce = [Guid]::NewGuid().ToString('N')
    }
    if ($null -ne $platform) {
        $payload.platformProvenance = $platform.Provenance
        $payload.supervisionRootTaskId = $platform.RootTaskId
        $payload.leaseReason = $platform.LeaseReason
        $payload.gitCommonDir = $platform.GitCommonDir
    }
    $encoding = [System.Text.UTF8Encoding]::new($false)
    $bytes = $encoding.GetBytes(($payload | ConvertTo-Json -Depth 5 -Compress))
    $contractId = Get-AiTaskSha256 $bytes
    $root = Get-AiTaskFinishContractRoot
    [System.IO.Directory]::CreateDirectory($root) | Out-Null
    $path = Join-Path $root "$contractId.json"
    $stream = [System.IO.File]::Open($path, [System.IO.FileMode]::CreateNew, [System.IO.FileAccess]::Write, [System.IO.FileShare]::Read)
    try { $stream.Write($bytes, 0, $bytes.Length); $stream.Flush($true) } finally { $stream.Dispose() }
    $contractId
}

function Assert-AiTaskFinishContract {
    param(
        [Parameter(Mandatory = $true)][string]$RepoPath,
        [Parameter(Mandatory = $true)][string]$ContractId,
        [switch]$AllowReleasedPlatformLease
    )
    if ($ContractId -notmatch '^[0-9a-f]{64}$') { throw 'TaskContract must be a SHA-256 id.' }
    $path = Join-Path (Get-AiTaskFinishContractRoot) "$ContractId.json"
    if (-not (Test-Path -LiteralPath $path -PathType Leaf)) { throw "Task finish contract not found: $ContractId" }
    $bytes = [System.IO.File]::ReadAllBytes($path)
    if ((Get-AiTaskSha256 $bytes) -ne $ContractId) { throw 'Task finish contract digest mismatch.' }
    $encoding = [System.Text.UTF8Encoding]::new($false, $true)
    $contract = $encoding.GetString($bytes) | ConvertFrom-Json
    if ($contract.schema -ne 'elon.ai_finish_contract.v1') { throw 'Unsupported task finish contract schema.' }
    $worktree = [System.IO.Path]::GetFullPath($RepoPath).TrimEnd('\', '/').Replace('\', '/')
    if (-not $worktree.Equals([string]$contract.worktree, [System.StringComparison]::OrdinalIgnoreCase)) {
        throw 'Task finish contract worktree identity mismatch.'
    }
    $branch = Get-AiTaskGitValue $RepoPath @('branch', '--show-current')
    $origin = Get-AiTaskGitValue $RepoPath @('remote', 'get-url', 'origin')
    if ($branch -ne [string]$contract.branch -or $origin -ne [string]$contract.origin) {
        throw 'Task finish contract branch or repository identity mismatch.'
    }
    if ($branch -like 'ai/session/*') {
        if ([string]$contract.platformProvenance -ne 'elon.conversation_worktree.v1' -or
            [string]::IsNullOrWhiteSpace([string]$contract.supervisionRootTaskId) -or
            [string]$contract.leaseReason -ne "elon-supervision:$([string]$contract.supervisionRootTaskId)") {
            throw 'Platform task finish contract lacks immutable provenance/root/lease identity.'
        }
        $platform = Get-AiTaskPlatformIdentity `
            -RepoPath $RepoPath `
            -Branch $branch `
            -ExpectedRoot ([string]$contract.supervisionRootTaskId) `
            -AllowReleasedLease:$AllowReleasedPlatformLease
        if ([string]$platform.GitCommonDir -ne [string]$contract.gitCommonDir) {
            throw 'Platform task finish contract Git common-dir identity mismatch.'
        }
    }
    & git -C $RepoPath merge-base --is-ancestor ([string]$contract.baseCommit) HEAD *> $null
    if ($LASTEXITCODE -ne 0) { throw 'Task HEAD is not descended from the preflight contract base.' }
    $contract
}
