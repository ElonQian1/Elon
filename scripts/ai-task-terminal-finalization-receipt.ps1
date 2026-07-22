function Get-AiTerminalFinalizationReceiptPath {
    param([Parameter(Mandatory = $true)][string]$RepoPath)
    $gitDir = Get-AiTaskGitValue $RepoPath @('rev-parse', '--path-format=absolute', '--git-dir')
    Join-Path ([System.IO.Path]::GetFullPath($gitDir)) 'elon-terminal-finalization-v1.json'
}

function Normalize-AiTerminalPath {
    param([Parameter(Mandatory = $true)][string]$Path)
    [System.IO.Path]::GetFullPath($Path).TrimEnd('\', '/').Replace('\', '/')
}

function Test-AiTerminalTimestamp {
    param([AllowNull()][object]$Value)
    if ([string]::IsNullOrWhiteSpace([string]$Value)) { return $false }
    $parsed = [DateTimeOffset]::MinValue
    [DateTimeOffset]::TryParse(
        [string]$Value,
        [Globalization.CultureInfo]::InvariantCulture,
        [Globalization.DateTimeStyles]::RoundtripKind,
        [ref]$parsed
    )
}

function Write-AiTerminalFinalizationReceipt {
    param(
        [Parameter(Mandatory = $true)][string]$Path,
        [Parameter(Mandatory = $true)]$Receipt
    )
    $encoding = [System.Text.UTF8Encoding]::new($false)
    $bytes = $encoding.GetBytes(($Receipt | ConvertTo-Json -Depth 8 -Compress))
    $directory = Split-Path -Parent $Path
    [System.IO.Directory]::CreateDirectory($directory) | Out-Null
    $temporary = Join-Path $directory ('.terminal-finalization-' + [Guid]::NewGuid().ToString('N') + '.tmp')
    $backup = Join-Path $directory ('.terminal-finalization-' + [Guid]::NewGuid().ToString('N') + '.bak')
    try {
        $stream = [System.IO.File]::Open(
            $temporary,
            [System.IO.FileMode]::CreateNew,
            [System.IO.FileAccess]::Write,
            [System.IO.FileShare]::None
        )
        try {
            $stream.Write($bytes, 0, $bytes.Length)
            $stream.Flush($true)
        } finally {
            $stream.Dispose()
        }
        if (Test-Path -LiteralPath $Path -PathType Leaf) {
            [System.IO.File]::Replace($temporary, $Path, $backup, $true)
        } else {
            [System.IO.File]::Move($temporary, $Path)
        }
    } finally {
        if (Test-Path -LiteralPath $temporary) { Remove-Item -LiteralPath $temporary -Force }
        if (Test-Path -LiteralPath $backup) { Remove-Item -LiteralPath $backup -Force }
    }
}

function Read-AiTerminalFinalizationReceipt {
    param([Parameter(Mandatory = $true)][string]$Path)
    if (-not (Test-Path -LiteralPath $Path -PathType Leaf)) { return $null }
    $bytes = [System.IO.File]::ReadAllBytes($Path)
    $encoding = [System.Text.UTF8Encoding]::new($false, $true)
    try {
        $receipt = $encoding.GetString($bytes) | ConvertFrom-Json -ErrorAction Stop
    } catch {
        throw "Malformed terminal finalization receipt: $($_.Exception.Message)"
    }
    $expected = @(
        'schema', 'state', 'finalizationId', 'taskId', 'completionEventId',
        'terminalStatus', 'taskContractId', 'supervisionRootTaskId', 'worktree',
        'baseWorkspace', 'gitDir', 'gitCommonDir', 'branch', 'origin', 'finalHead',
        'leaseMarkerFingerprint', 'fingerprint', 'preparedAtUtc', 'completedAtUtc',
        'boundAtUtc'
    )
    $actual = @($receipt.PSObject.Properties.Name)
    if ($actual.Count -ne $expected.Count -or @($expected | Where-Object { $_ -notin $actual }).Count -ne 0) {
        throw 'Terminal finalization receipt schema fields are incomplete or unexpected.'
    }
    if ([string]$receipt.schema -ne 'elon.terminal_finalization.v1') {
        throw 'Unsupported terminal finalization receipt schema.'
    }
    $receipt
}

function Get-AiTerminalFinalizationIdentity {
    param(
        [Parameter(Mandatory = $true)][string]$TaskRoot,
        [Parameter(Mandatory = $true)][string]$BasePath,
        [Parameter(Mandatory = $true)][string]$TaskContract,
        [Parameter(Mandatory = $true)]$ValidatedContract
    )
    $status = Invoke-AiTerminalGitRequired -RepoPath $TaskRoot -GitArgs @('status', '--porcelain=v1', '--untracked-files=all')
    if (-not [string]::IsNullOrWhiteSpace($status)) {
        throw 'Terminal finalization fingerprint requires a clean task worktree.'
    }
    $branch = (Invoke-AiTerminalGitRequired -RepoPath $TaskRoot -GitArgs @('branch', '--show-current')).Trim()
    if ((Invoke-AiTerminalGitRequired -RepoPath $BasePath -GitArgs @('branch', '--show-current')).Trim() -ne 'main') {
        throw 'Terminal finalization base workspace is not main.'
    }
    if ($branch -ne [string]$ValidatedContract.branch) {
        throw 'Terminal finalization branch drifted from TaskContract.'
    }
    $active = Normalize-AiTerminalPath $TaskRoot
    $top = Normalize-AiTerminalPath (Invoke-AiTerminalGitRequired -RepoPath $TaskRoot -GitArgs @('rev-parse', '--show-toplevel'))
    if (-not $top.Equals($active, [StringComparison]::OrdinalIgnoreCase)) {
        throw 'Terminal finalization active workspace is not its Git root.'
    }
    $base = Normalize-AiTerminalPath $BasePath
    if ($base.Equals($active, [StringComparison]::OrdinalIgnoreCase)) {
        throw 'Terminal finalization refuses the shared base workspace.'
    }
    $common = Normalize-AiTerminalPath (Invoke-AiTerminalGitRequired -RepoPath $TaskRoot -GitArgs @('rev-parse', '--path-format=absolute', '--git-common-dir'))
    $baseCommon = Normalize-AiTerminalPath (Invoke-AiTerminalGitRequired -RepoPath $BasePath -GitArgs @('rev-parse', '--path-format=absolute', '--git-common-dir'))
    $contractCommon = Normalize-AiTerminalPath ([string]$ValidatedContract.gitCommonDir)
    if (-not $common.Equals($baseCommon, [StringComparison]::OrdinalIgnoreCase) -or
        -not $common.Equals($contractCommon, [StringComparison]::OrdinalIgnoreCase)) {
        throw 'Terminal finalization Git common-dir identity drifted.'
    }
    $origin = (Invoke-AiTerminalGitRequired -RepoPath $TaskRoot -GitArgs @('remote', 'get-url', 'origin')).Trim()
    $baseOrigin = (Invoke-AiTerminalGitRequired -RepoPath $BasePath -GitArgs @('remote', 'get-url', 'origin')).Trim()
    if ($origin -ne [string]$ValidatedContract.origin -or $baseOrigin -ne $origin) {
        throw 'Terminal finalization origin identity drifted.'
    }
    Invoke-AiTerminalFetchWithRetry -RepoPath $BasePath
    $head = (Invoke-AiTerminalGitRequired -RepoPath $TaskRoot -GitArgs @('rev-parse', 'HEAD^{commit}')).Trim()
    $landed = Invoke-AiTerminalGitCapture -RepoPath $TaskRoot -GitArgs @('merge-base', '--is-ancestor', $head, 'origin/main')
    if ($landed.ExitCode -ne 0) {
        throw "Terminal finalization HEAD is not pushed/landed in origin/main: $head"
    }
    [pscustomobject]@{ Fields = [ordered]@{
        taskContractId = $TaskContract
        supervisionRootTaskId = [string]$ValidatedContract.supervisionRootTaskId
        worktree = $active
        baseWorkspace = $base
        gitDir = Normalize-AiTerminalPath (Invoke-AiTerminalGitRequired -RepoPath $TaskRoot -GitArgs @('rev-parse', '--path-format=absolute', '--git-dir'))
        gitCommonDir = $common
        branch = $branch
        origin = $origin
        finalHead = $head
    } }
}

function Get-AiTerminalFinalizationFingerprint {
    param(
        [Parameter(Mandatory = $true)]$Identity,
        [Parameter(Mandatory = $true)][string]$LeaseMarkerFingerprint
    )
    $lines = @($Identity.Fields.Keys | ForEach-Object { "$_=$($Identity.Fields[$_])" })
    $lines += "leaseMarkerFingerprint=$LeaseMarkerFingerprint"
    $encoding = [System.Text.UTF8Encoding]::new($false)
    Get-AiTaskSha256 ($encoding.GetBytes(($lines -join "`n")))
}

function Assert-AiTerminalFinalizationReceipt {
    param(
        [Parameter(Mandatory = $true)]$Receipt,
        [Parameter(Mandatory = $true)]$Identity,
        [Parameter(Mandatory = $true)][string]$TaskContract,
        [Parameter(Mandatory = $true)][string]$RootTaskId
    )
    if ([string]$Receipt.state -notin @('prepared', 'completed')) {
        throw 'Terminal finalization receipt state is invalid.'
    }
    $fingerprint = Get-AiTerminalFinalizationFingerprint -Identity $Identity `
        -LeaseMarkerFingerprint ([string]$Receipt.leaseMarkerFingerprint)
    if ([string]$Receipt.finalizationId -notmatch '^[0-9a-f]{32}$' -or
        [string]$Receipt.taskContractId -ne $TaskContract -or
        [string]$Receipt.supervisionRootTaskId -ne $RootTaskId -or
        [string]$Receipt.leaseMarkerFingerprint -notmatch '^[0-9a-f]{64}$' -or
        [string]$Receipt.fingerprint -notmatch '^[0-9a-f]{64}$' -or
        [string]$Receipt.fingerprint -ne $fingerprint) {
        throw 'Terminal finalization receipt immutable identity is invalid.'
    }
    foreach ($name in $Identity.Fields.Keys) {
        if ([string]$Receipt.$name -ne [string]$Identity.Fields[$name]) {
            throw "Terminal finalization receipt field drifted: $name"
        }
    }
    if (-not (Test-AiTerminalTimestamp $Receipt.preparedAtUtc)) {
        throw 'Terminal finalization preparedAtUtc is invalid.'
    }
    $binding = @($Receipt.taskId, $Receipt.completionEventId, $Receipt.terminalStatus, $Receipt.boundAtUtc)
    $bindingPresent = @($binding | Where-Object { -not [string]::IsNullOrWhiteSpace([string]$_) }).Count
    if ([string]$Receipt.state -eq 'prepared') {
        if ($bindingPresent -ne 0 -or $null -ne $Receipt.completedAtUtc) {
            throw 'Prepared terminal finalization receipt contains completed or bound fields.'
        }
        return
    }
    if (-not (Test-AiTerminalTimestamp $Receipt.completedAtUtc)) {
        throw 'Completed terminal finalization receipt lacks a valid completedAtUtc.'
    }
    if ($bindingPresent -notin @(0, 4)) {
        throw 'Completed terminal finalization receipt has a partial completion binding.'
    }
    if ($bindingPresent -eq 4 -and
        ([string]$Receipt.terminalStatus -notin @('done', 'failed', 'canceled') -or
         -not (Test-AiTerminalTimestamp $Receipt.boundAtUtc))) {
        throw 'Completed terminal finalization receipt binding is invalid.'
    }
}

function New-AiPreparedTerminalFinalizationReceipt {
    param(
        [Parameter(Mandatory = $true)]$Identity,
        [Parameter(Mandatory = $true)][string]$LeaseMarkerFingerprint
    )
    $receipt = [ordered]@{
        schema = 'elon.terminal_finalization.v1'; state = 'prepared'
        finalizationId = [Guid]::NewGuid().ToString('N')
        taskId = $null; completionEventId = $null; terminalStatus = $null
        taskContractId = $Identity.Fields.taskContractId
        supervisionRootTaskId = $Identity.Fields.supervisionRootTaskId
        worktree = $Identity.Fields.worktree; baseWorkspace = $Identity.Fields.baseWorkspace
        gitDir = $Identity.Fields.gitDir; gitCommonDir = $Identity.Fields.gitCommonDir
        branch = $Identity.Fields.branch; origin = $Identity.Fields.origin
        finalHead = $Identity.Fields.finalHead
        leaseMarkerFingerprint = $LeaseMarkerFingerprint
        fingerprint = $null; preparedAtUtc = [DateTime]::UtcNow.ToString('o')
        completedAtUtc = $null; boundAtUtc = $null
    }
    $receipt.fingerprint = Get-AiTerminalFinalizationFingerprint -Identity $Identity `
        -LeaseMarkerFingerprint $LeaseMarkerFingerprint
    $receipt
}
