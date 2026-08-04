<#
.SYNOPSIS
    Verify that the documented multi-client feature ownership still matches the repository.

.DESCRIPTION
    This is a structural migration audit. It does not claim that the UI behavior is
    functionally identical across clients; it verifies that each documented feature has
    an explicit implementation anchor and that the published entrypoints still point to
    the expected client. Functional parity remains a release/acceptance task.
#>
param(
    [switch]$Detailed
)

$ErrorActionPreference = "Stop"

function Stop-FeatureParityAudit {
    param([string]$Message)
    Write-Error $Message
    exit 1
}

$repoRoot = (& git rev-parse --show-toplevel).Trim()
if ($LASTEXITCODE -ne 0 -or [string]::IsNullOrWhiteSpace($repoRoot)) {
    Stop-FeatureParityAudit "Current directory is not inside a git repository."
}
Set-Location $repoRoot

$matrixPath = Join-Path $repoRoot "docs\architecture\feature-parity-matrix.md"
$routerPath = Join-Path $repoRoot "server\src\router.rs"
$appPath = Join-Path $repoRoot "pc-frontend\src\App.tsx"

foreach ($required in @($matrixPath, $routerPath, $appPath)) {
    if (-not (Test-Path -LiteralPath $required -PathType Leaf)) {
        Stop-FeatureParityAudit "Required migration audit file is missing: $required"
    }
}

function Assert-Path {
    param(
        [string]$Path,
        [string]$Reason
    )
    $fullPath = Join-Path $repoRoot $Path
    if (-not (Test-Path -LiteralPath $fullPath)) {
        Stop-FeatureParityAudit "Missing feature ownership anchor '$Path' ($Reason)."
    }
    if ($Detailed) {
        Write-Host "FEATURE_ANCHOR=passed path=$Path reason=$Reason"
    }
}

function Assert-Text {
    param(
        [string]$Path,
        [string]$Needle,
        [string]$Reason
    )
    $fullPath = Join-Path $repoRoot $Path
    $text = [System.IO.File]::ReadAllText($fullPath, [System.Text.Encoding]::UTF8)
    if (-not $text.Contains($Needle)) {
        Stop-FeatureParityAudit "Missing entrypoint contract '$Needle' in '$Path' ($Reason)."
    }
    if ($Detailed) {
        Write-Host "FEATURE_CONTRACT=passed path=$Path needle=$Needle"
    }
}

$matrix = [System.IO.File]::ReadAllText($matrixPath, [System.Text.Encoding]::UTF8)
if ($matrix -notmatch '(?m)^\|.+\|.+\|.+\|.+\|.+\|.+\|$') {
    Stop-FeatureParityAudit "Feature parity matrix is missing its canonical table structure."
}

Assert-Text "server\src\router.rs" '.route("/", get(web::web_page))' "mobile web root"
Assert-Text "server\src\router.rs" '.route("/web", get(web::web_page))' "mobile web alias"
Assert-Text "server\src\router.rs" '.nest_service("/pc", pc_router)' "canonical PC entrypoint"
Assert-Text "server\src\router.rs" '.nest_service("/pc-next", pc_next_router)' "PC compatibility alias"
Assert-Text "server\src\router.rs" '.nest_service("/pc-legacy", pc_legacy_svc)' "read-only historical PC snapshot"
Assert-Text "server\src\assets\web_page.html" '/assets/project_plaza.js' "mobile web project plaza"
Assert-Text "server\src\assets\web_page.html" '/assets/project_home.js' "mobile web project center"
Assert-Text "pc-frontend\src\App.tsx" 'path="ai"' "PC home AI"
Assert-Text "pc-frontend\src\App.tsx" 'path="plaza"' "PC project plaza"
Assert-Text "pc-frontend\src\App.tsx" 'path="projects"' "PC project center"
Assert-Text "pc-frontend\src\App.tsx" 'path="doctor"' "PC computer doctor"
Assert-Text "pc-frontend\src\App.tsx" 'path="dev-tasks"' "PC AI development tasks"
Assert-Text "pc-frontend\src\App.tsx" 'path="/login"' "PC authentication route"

$anchors = @{
    "home-ai-pc" = @(
        "pc-frontend\src\features\ai\AiChatPage.tsx",
        "server\src\lm_chat.rs",
        "server\src\home_ai_orchestrator.rs"
    )
    "project-plaza" = @(
        "pc-frontend\src\features\plaza\ProjectPlazaView.tsx",
        "server\src\assets\project_plaza.js",
        "android\app\src\main\java\com\elon\app\agent\ui\ProjectPlazaActivity.kt"
    )
    "project-center" = @(
        "pc-frontend\src\features\projects\ProjectsPage.tsx",
        "pc-frontend\src\features\projects\ProjectDetailPage.tsx",
        "server\src\project_api.rs",
        "server\src\project_store.rs"
    )
    "computer-doctor" = @(
        "pc-frontend\src\features\doctor\DoctorPage.tsx",
        "pc-frontend\src\features\doctor\localApi.ts",
        "server\src\node_router.rs"
    )
    "development-tasks" = @(
        "pc-frontend\src\features\dev\DevTasksPage.tsx",
        "pc-frontend\src\features\local-tasks\LocalTasksPage.tsx",
        "server\src\ai_cli"
    )
    "auth-account" = @(
        "pc-frontend\src\features\auth\LoginPage.tsx",
        "pc-frontend\src\features\account\AccountPage.tsx",
        "server\src\auth_api.rs"
    )
    "android-main-chat" = @(
        "android\app\src\main\kotlin\com\elon\app\MainActivity.kt",
        "android\app\src\main\kotlin\com\elon\app\MainSendMessageActions.kt",
        "android\app\src\main\java\com\elon\app\agent\application\InputGateway.kt"
    )
}

foreach ($feature in $anchors.Keys | Sort-Object) {
    foreach ($anchor in $anchors[$feature]) {
        Assert-Path $anchor $feature
    }
}

$legacyPcSources = @(git ls-files | Where-Object {
    $_ -match '(^|/)server/src/assets/pc_[^/]+\.(html|js|css)$'
})
if ($legacyPcSources.Count -gt 0) {
    Stop-FeatureParityAudit ("Historical PC source files are tracked again: " + ($legacyPcSources -join ', '))
}

Write-Host "FEATURE_PARITY_AUDIT=passed features=$($anchors.Count) legacy_pc_sources=0"
