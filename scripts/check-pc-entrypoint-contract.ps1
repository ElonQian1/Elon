# Verify that the PC frontend has one canonical implementation and that the
# historical PC source tree is not accidentally restored.

$ErrorActionPreference = "Stop"

function Stop-PcEntrypointContract {
    param([string]$Message)
    Write-Error $Message
    exit 1
}

$repoRoot = (& git rev-parse --show-toplevel).Trim()
if ($LASTEXITCODE -ne 0 -or [string]::IsNullOrWhiteSpace($repoRoot)) {
    Stop-PcEntrypointContract "Current directory is not inside a git repository."
}
Set-Location $repoRoot

function Read-RequiredText {
    param([string]$Path)
    if (-not (Test-Path -LiteralPath $Path -PathType Leaf)) {
        Stop-PcEntrypointContract "Required PC entrypoint file is missing: $Path"
    }
    return [System.IO.File]::ReadAllText((Join-Path $repoRoot $Path), [System.Text.Encoding]::UTF8)
}

function Assert-Text {
    param(
        [string]$Path,
        [string]$Text,
        [string]$Needle
    )
    if (-not $Text.Contains($Needle)) {
        Stop-PcEntrypointContract "$Path is missing the canonical entrypoint contract: $Needle"
    }
}

$router = Read-RequiredText "server/src/router.rs"
$app = Read-RequiredText "pc-frontend/src/App.tsx"
$vite = Read-RequiredText "pc-frontend/vite.config.ts"
$projects = Read-RequiredText "pc-frontend/src/features/projects/ProjectsPage.tsx"
$plaza = Read-RequiredText "pc-frontend/src/features/plaza/PlazaPage.tsx"
$migration = Read-RequiredText "docs/pc-frontend-migration.md"

Assert-Text "server/src/router.rs" $router '.nest_service("/pc", pc_router)'
Assert-Text "server/src/router.rs" $router '.nest_service("/pc-next", pc_next_router)'
Assert-Text "server/src/router.rs" $router '.nest_service("/pc-legacy", pc_legacy_svc)'
Assert-Text "pc-frontend/src/App.tsx" $app 'path="plaza"'
Assert-Text "pc-frontend/src/App.tsx" $app 'path="projects"'
Assert-Text "pc-frontend/vite.config.ts" $vite "base: '/pc/'"
Assert-Text "pc-frontend/src/features/projects/ProjectsPage.tsx" $projects '<ProjectPlazaView />'
Assert-Text "pc-frontend/src/features/plaza/PlazaPage.tsx" $plaza '<ProjectPlazaView />'
Assert-Text "docs/pc-frontend-migration.md" $migration '/pc-legacy'

$trackedPcLegacySources = @(git ls-files | Where-Object {
    $_ -match '(^|/)server/src/assets/pc_[^/]+\.(html|js|css)$'
})
if ($trackedPcLegacySources.Count -gt 0) {
    Stop-PcEntrypointContract ("Historical PC source files must not return: " + ($trackedPcLegacySources -join ', '))
}

Write-Host "PC_ENTRYPOINT_CONTRACT=passed"

