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

function New-AiTaskFinishContract {
    param([Parameter(Mandatory = $true)][string]$RepoPath)
    $worktree = [System.IO.Path]::GetFullPath($RepoPath).TrimEnd('\', '/')
    $branch = Get-AiTaskGitValue $worktree @('branch', '--show-current')
    $baseCommit = Get-AiTaskGitValue $worktree @('rev-parse', 'HEAD^{commit}')
    $origin = Get-AiTaskGitValue $worktree @('remote', 'get-url', 'origin')
    $payload = [ordered]@{
        schema = 'elon.ai_finish_contract.v1'
        worktree = $worktree.Replace('\', '/')
        branch = $branch
        baseCommit = $baseCommit
        origin = $origin
        issuedAtUtc = [DateTime]::UtcNow.ToString('o')
        nonce = [Guid]::NewGuid().ToString('N')
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
        [Parameter(Mandatory = $true)][string]$ContractId
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
    & git -C $RepoPath merge-base --is-ancestor ([string]$contract.baseCommit) HEAD *> $null
    if ($LASTEXITCODE -ne 0) { throw 'Task HEAD is not descended from the preflight contract base.' }
    $contract
}
