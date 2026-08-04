[CmdletBinding()]
param(
    [Parameter(Mandatory = $true)]
    [string]$CaseId,
    [Parameter(Mandatory = $true)]
    [string]$ModelId,
    [Parameter(Mandatory = $true)]
    [string]$TaskFile,
    [string]$CodexBuild,
    [string]$ProjectRoot,
    [string]$OutputPath
)

$ErrorActionPreference = "Stop"

function Get-Sha256Text {
    param([string]$Value)
    $sha = [System.Security.Cryptography.SHA256]::Create()
    try {
        $bytes = [System.Text.Encoding]::UTF8.GetBytes($Value)
        return ([System.BitConverter]::ToString($sha.ComputeHash($bytes))).Replace('-', '').ToLowerInvariant()
    } finally {
        $sha.Dispose()
    }
}

function Assert-BoundedValue {
    param([string]$Name, [string]$Value, [int]$Limit)
    if ([string]::IsNullOrWhiteSpace($Value) -or $Value.Length -gt $Limit -or $Value -match '[\r\n]') {
        throw "$Name must contain 1-$Limit characters without newlines."
    }
}

if ($CaseId -notmatch '^[A-Za-z0-9._-]{1,64}$') {
    throw 'CaseId must contain 1-64 ASCII letters, digits, dots, underscores, or hyphens.'
}
Assert-BoundedValue -Name 'ModelId' -Value $ModelId -Limit 80

if ([string]::IsNullOrWhiteSpace($ProjectRoot)) {
    $ProjectRoot = Split-Path -Parent $PSScriptRoot
}
$ProjectRoot = [System.IO.Path]::GetFullPath($ProjectRoot)
$TaskFile = [System.IO.Path]::GetFullPath($TaskFile)
if (-not (Test-Path -LiteralPath $TaskFile -PathType Leaf)) { throw "TaskFile does not exist: $TaskFile" }
$taskLength = (Get-Item -LiteralPath $TaskFile).Length
if ($taskLength -le 0 -or $taskLength -gt 1048576) { throw 'TaskFile must contain 1 byte to 1 MiB.' }

if ([string]::IsNullOrWhiteSpace($CodexBuild)) {
    $CodexBuild = $env:BROWSER_USE_CODEX_APP_VERSION
}
if ([string]::IsNullOrWhiteSpace($CodexBuild)) {
    try {
        $CodexBuild = @(Get-AppxPackage -Name OpenAI.Codex -ErrorAction Stop | Sort-Object Version -Descending | Select-Object -First 1).Version.ToString()
    } catch {
        $CodexBuild = ""
    }
}
Assert-BoundedValue -Name 'CodexBuild' -Value $CodexBuild -Limit 80

$gitHeadOutput = @(& git -C $ProjectRoot rev-parse HEAD 2>$null)
$gitHeadExitCode = $LASTEXITCODE
$gitHead = if ($gitHeadOutput.Count -gt 0) { ([string]$gitHeadOutput[0]).Trim() } else { "" }
if ($gitHeadExitCode -ne 0 -or $gitHead -notmatch '^[0-9a-fA-F]{40,64}$') { throw 'ProjectRoot must be a Git worktree with a resolvable HEAD.' }
$trackedChanges = @(& git -C $ProjectRoot status --porcelain --untracked-files=no 2>$null)
$gitStatusExitCode = $LASTEXITCODE
if ($gitStatusExitCode -ne 0) { throw 'Unable to inspect tracked Git worktree state.' }
if ($trackedChanges.Count -gt 0) { throw 'Benchmark plans require a clean tracked worktree. Commit the benchmarked code first.' }

$taskSha256 = (Get-FileHash -Algorithm SHA256 -LiteralPath $TaskFile).Hash.ToLowerInvariant()
$schema = 'elon.project_memory_ab_manifest.v1'
$canonical = @(
    "schema=$schema",
    "case_id=$CaseId",
    "model_id=$ModelId",
    "task_sha256=$taskSha256",
    "git_head=$($gitHead.ToLowerInvariant())",
    "codex_build=$CodexBuild"
) -join "`n"
$manifestSha256 = Get-Sha256Text -Value $canonical
$benchmarkKey = "pmab-$($manifestSha256.Substring(0, 32))"

if ([string]::IsNullOrWhiteSpace($OutputPath)) {
    $OutputPath = Join-Path $ProjectRoot ".ai-tmp\project-memory-benchmarks\$benchmarkKey.json"
}
$OutputPath = [System.IO.Path]::GetFullPath($OutputPath)
$outputDirectory = Split-Path -Parent $OutputPath
New-Item -ItemType Directory -Force -Path $outputDirectory | Out-Null

$manifest = [ordered]@{
    schema = $schema
    benchmark_key = $benchmarkKey
    case_id = $CaseId
    model_id = $ModelId
    task_sha256 = $taskSha256
    git_head = $gitHead.ToLowerInvariant()
    codex_build = $CodexBuild
    project_root = $ProjectRoot
    tracked_worktree_clean = $true
    manifest_sha256 = $manifestSha256
    created_at_utc = [DateTime]::UtcNow.ToString('o')
    stores_task_text = $false
}
$manifestJson = $manifest | ConvertTo-Json -Depth 6
[System.IO.File]::WriteAllText($OutputPath, "$manifestJson`n", (New-Object System.Text.UTF8Encoding($false)))

[ordered]@{
    schema = 'elon.project_memory_ab_plan_result.v1'
    manifest_path = $OutputPath
    benchmark_key = $benchmarkKey
    measurement_windows = @('baseline_without_project_memory', 'with_project_memory')
    observer_common_args = @(
        '--benchmark-manifest', $OutputPath,
        '--task-file', $TaskFile,
        '--model-id', $ModelId,
        '--codex-build', $CodexBuild
    )
    task_text_stored = $false
    claim_rule = 'Only compare both completed windows when the observer reports benchmark_protocol_verified=true for this manifest.'
} | ConvertTo-Json -Depth 8
