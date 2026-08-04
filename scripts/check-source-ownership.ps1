# Prevent accidental edits to legacy mobile-web assets during normal commits.
# This path guard documents the boundary between the React PC frontend and the
# still-active mobile Web implementation.
param(
    [switch]$Staged,
    [switch]$EnforceLegacy,
    [string]$BaseRef = ""
)

$ErrorActionPreference = "Stop"

function Stop-SourceOwnershipGuard {
    param([string]$Message)
    Write-Error $Message
    exit 1
}

$repoRoot = (& git rev-parse --show-toplevel).Trim()
if ($LASTEXITCODE -ne 0 -or [string]::IsNullOrWhiteSpace($repoRoot)) {
    Stop-SourceOwnershipGuard "Current directory is not inside a git repository."
}
Set-Location $repoRoot

$requiredDocs = @(
    "docs/architecture/source-of-truth.md",
    "docs/architecture/legacy-inventory.md",
    "docs/architecture/feature-parity-matrix.md"
)
foreach ($doc in $requiredDocs) {
    if (-not (Test-Path -LiteralPath $doc -PathType Leaf)) {
        Stop-SourceOwnershipGuard "Missing source ownership document: $doc"
    }
}

if ($Staged) {
    $changed = @(git diff --cached --name-only --diff-filter=ACMR)
} elseif (-not [string]::IsNullOrWhiteSpace($BaseRef)) {
    $changed = @(git diff --name-only --diff-filter=ACMR "$BaseRef...HEAD")
} else {
    $changed = @(git diff --name-only --diff-filter=ACMR HEAD)
    $changed += @(git ls-files --others --exclude-standard)
}

$changed = @($changed | ForEach-Object { $_.Trim().Replace('\', '/') } | Where-Object { $_ })
$legacyPaths = @(
    "server/src/assets/web_page.html",
    "server/src/assets/project_plaza.js",
    "server/src/assets/project_plaza.css",
    "server/src/assets/project_home.js",
    "server/src/assets/project_home.css"
)
$legacyChanges = @($changed | Where-Object { $legacyPaths -contains $_ })
$forbiddenPcLegacySources = @($changed | Where-Object {
    $_ -match '^server/src/assets/pc_[^/]+\.(html|js|css)$'
})

if ($forbiddenPcLegacySources.Count -gt 0) {
    Stop-SourceOwnershipGuard ("Forbidden legacy PC source path added: " + ($forbiddenPcLegacySources -join ', '))
}

if ($EnforceLegacy -and $legacyChanges.Count -gt 0) {
    $scope = [Environment]::GetEnvironmentVariable("ELON_CHANGE_SCOPE")
    $allow = [Environment]::GetEnvironmentVariable("ELON_ALLOW_LEGACY_CHANGE")
    if ($scope -ne "MobileWeb" -and $allow -ne "1") {
        $legacyText = $legacyChanges -join ", "
        $message = "Legacy mobile-web files changed: " + $legacyText + ". Set ELON_CHANGE_SCOPE=MobileWeb for an intentional mobile-web task."
        Stop-SourceOwnershipGuard $message
    }
}

Write-Host ("SOURCE_OWNERSHIP_GUARD=passed changed={0} legacy={1} enforce={2}" -f $changed.Count, $legacyChanges.Count, $EnforceLegacy)
