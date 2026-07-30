param(
    [string]$RunbookPath = "docs\release-quality-gates.md",
    [string]$CompletionScriptPath = "scripts\check-task-complete.ps1"
)

$ErrorActionPreference = "Stop"

function Stop-RunbookGuard {
    param([string]$Message)
    Write-Error $Message
    exit 1
}

function Get-RepoRoot {
    $root = (& git rev-parse --show-toplevel).Trim()
    if ($LASTEXITCODE -ne 0 -or [string]::IsNullOrWhiteSpace($root)) {
        Stop-RunbookGuard "Current directory is not inside a git repository."
    }
    return $root
}

function Read-TextFile {
    param([string]$Path)
    if (-not (Test-Path -LiteralPath $Path -PathType Leaf)) {
        Stop-RunbookGuard "Required file is missing: $Path"
    }
    return [System.IO.File]::ReadAllText($Path, [System.Text.Encoding]::UTF8)
}

function ConvertTo-RepoPath {
    param([string]$Path)
    return (($Path -replace "/", "\").Trim())
}

function Get-RunbookScriptRefs {
    param([string]$Text)
    $refs = New-Object System.Collections.Generic.HashSet[string]
    $matches = [regex]::Matches($Text, 'scripts[\\/][A-Za-z0-9_.-]+\.(?:ps1|psm1|js|sh)')
    foreach ($match in $matches) {
        $null = $refs.Add((ConvertTo-RepoPath $match.Value))
    }
    return @($refs | Sort-Object)
}

function Get-RunbookKinds {
    param([string]$Text)
    $kinds = New-Object System.Collections.Generic.HashSet[string]
    $matches = [regex]::Matches($Text, 'check-task-complete\.ps1\s+-Kind\s+([A-Za-z][A-Za-z0-9_]*)')
    foreach ($match in $matches) {
        $null = $kinds.Add($match.Groups[1].Value)
    }
    return @($kinds | Sort-Object)
}

function Get-ValidateSetKinds {
    param([string]$Text)
    $validateSet = [regex]::Match($Text, '\[ValidateSet\((?<items>[^\)]*)\)\]\s*\r?\n\s*\[string\]\$Kind')
    if (-not $validateSet.Success) {
        Stop-RunbookGuard "Cannot find ValidateSet for check-task-complete.ps1 -Kind."
    }
    $kinds = New-Object System.Collections.Generic.HashSet[string]
    $matches = [regex]::Matches($validateSet.Groups["items"].Value, '"([^"]+)"')
    foreach ($match in $matches) {
        $null = $kinds.Add($match.Groups[1].Value)
    }
    if ($kinds.Count -eq 0) {
        Stop-RunbookGuard "ValidateSet for check-task-complete.ps1 -Kind is empty."
    }
    return @($kinds | Sort-Object)
}

function Assert-ContainsAll {
    param(
        [string]$Label,
        [string[]]$Actual,
        [string[]]$Required
    )
    $missing = @()
    foreach ($item in $Required) {
        if ($Actual -notcontains $item) {
            $missing += $item
        }
    }
    if ($missing.Count -gt 0) {
        Stop-RunbookGuard "$Label missing required entries: $($missing -join ', ')"
    }
}

$repoRoot = Get-RepoRoot
Set-Location $repoRoot

$runbookFullPath = Join-Path $repoRoot $RunbookPath
$completionFullPath = Join-Path $repoRoot $CompletionScriptPath
$runbook = Read-TextFile $runbookFullPath
$completionScript = Read-TextFile $completionFullPath

$scriptRefs = Get-RunbookScriptRefs $runbook
$runbookKinds = Get-RunbookKinds $runbook
$validateSetKinds = Get-ValidateSetKinds $completionScript

$requiredScriptRefs = @(
    "scripts\publish-server.ps1",
    "scripts\publish-node-agent.ps1",
    "scripts\publish-apk.ps1",
    "scripts\publish-server-pc-frontend.ps1",
    "scripts\publish-health-checks.ps1",
    "scripts\check-local-quality.ps1",
    "scripts\check-pc-frontend-bundle-budget.js",
    "scripts\check-source-size.ps1",
    "scripts\check-document-modularity.ps1",
    "scripts\check-dependency-audit.ps1",
    "scripts\check-rust-warning-budget.ps1",
    "scripts\check-release-runbook.ps1",
    "scripts\check-ci-quality-gates.ps1",
    "scripts\check-realtime-runbook.ps1",
    "scripts\check-realtime-ownership.ps1",
    "scripts\check-realtime-diagnostics-snapshot.ps1",
    "scripts\check-task-complete.ps1"
)
$requiredRunbookKinds = @("Server", "PcFrontend", "NodeAgent", "AndroidFeature")

Assert-ContainsAll -Label "Runbook script refs" -Actual $scriptRefs -Required $requiredScriptRefs
Assert-ContainsAll -Label "Runbook completion kinds" -Actual $runbookKinds -Required $requiredRunbookKinds

foreach ($scriptRef in $scriptRefs) {
    if (-not (Test-Path -LiteralPath (Join-Path $repoRoot $scriptRef) -PathType Leaf)) {
        Stop-RunbookGuard "Runbook references missing script: $scriptRef"
    }
}

foreach ($kind in $runbookKinds) {
    if ($validateSetKinds -notcontains $kind) {
        Stop-RunbookGuard "Runbook references unsupported check-task-complete kind: $kind"
    }
}

Write-Host "RELEASE_RUNBOOK_GUARD=passed scripts=$($scriptRefs.Count) kinds=$($runbookKinds -join ',')"
