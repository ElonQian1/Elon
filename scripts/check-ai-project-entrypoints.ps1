[CmdletBinding()]
param()

$ErrorActionPreference = "Stop"

function Assert-Contains {
    param([string]$Content, [string]$Expected, [string]$Label)
    if (-not $Content.Contains($Expected)) {
        throw "$Label is missing '$Expected'."
    }
}

function Read-Json {
    param([string]$Path)
    try {
        return [System.IO.File]::ReadAllText($Path, [System.Text.Encoding]::UTF8) | ConvertFrom-Json
    } catch {
        throw "Cannot parse $Path`: $($_.Exception.Message)"
    }
}

$repoRoot = (& git rev-parse --show-toplevel).Trim()
if ($LASTEXITCODE -ne 0 -or [string]::IsNullOrWhiteSpace($repoRoot)) {
    throw "Current directory is not inside a Git repository."
}
Set-Location $repoRoot

$requiredFiles = @(
    "AI_CURRENT.md",
    "AI_ARCHITECTURE.md",
    "docs/decisions/reject-demo-oracle-role.md",
    "docs/decisions/reject-ai-to-ai-skill-route.md",
    "docs/distributed-compute/current-implementation-status.md",
    "docs/distributed-compute/implementation-roadmap.md",
    "docs/distributed-compute/README.md",
    "docs/distributed-compute/settlement-withdrawal-request-api.md",
    "docs/distributed-compute/settlement-withdrawal-terminal-api.md",
    "default-project-docs/files/AI_CURRENT.md"
)
foreach ($path in $requiredFiles) {
    if (-not (Test-Path -LiteralPath $path -PathType Leaf)) {
        throw "Required AI project entry file is missing: $path"
    }
}

$agents = [System.IO.File]::ReadAllText("AGENTS.md")
$copilot = [System.IO.File]::ReadAllText(".github/copilot-instructions.md")
$contextCompiler = [System.IO.File]::ReadAllText("server/src/context_compiler/agent_rag_project_docs.rs")
$defaultSeeder = [System.IO.File]::ReadAllText("server/src/project_default_docs.rs")
$current = [System.IO.File]::ReadAllText("AI_CURRENT.md")
$architecture = [System.IO.File]::ReadAllText("AI_ARCHITECTURE.md")
$computeStatus = [System.IO.File]::ReadAllText("docs/distributed-compute/current-implementation-status.md")
$computeRoadmap = [System.IO.File]::ReadAllText("docs/distributed-compute/implementation-roadmap.md")
$computeReadme = [System.IO.File]::ReadAllText("docs/distributed-compute/README.md")
Assert-Contains $agents "AI_CURRENT.md" "AGENTS.md"
Assert-Contains $copilot "AI_CURRENT.md" ".github/copilot-instructions.md"
Assert-Contains $contextCompiler 'path: "AI_CURRENT.md"' "agent project docs context"
Assert-Contains $defaultSeeder 'path: "AI_CURRENT.md"' "default project document seeder"
Assert-Contains $current "reject-demo-oracle-role.md" "AI_CURRENT.md"
Assert-Contains $current "reject-ai-to-ai-skill-route.md" "AI_CURRENT.md"

$computeSectionStartMarker = "<!-- distributed-compute-architecture:start -->"
$computeSectionEndMarker = "<!-- distributed-compute-architecture:end -->"
$computeSectionStart = $architecture.IndexOf($computeSectionStartMarker, [System.StringComparison]::Ordinal)
$computeSectionEnd = if ($computeSectionStart -ge 0) {
    $architecture.IndexOf(
        $computeSectionEndMarker,
        $computeSectionStart + $computeSectionStartMarker.Length,
        [System.StringComparison]::Ordinal
    )
} else {
    -1
}
if ($computeSectionStart -lt 0 -or $computeSectionEnd -le $computeSectionStart) {
    throw "AI_ARCHITECTURE.md must retain the distributed-compute architecture section before current gaps."
}
$computeArchitecture = $architecture.Substring(
    $computeSectionStart,
    ($computeSectionEnd + $computeSectionEndMarker.Length) - $computeSectionStart
)
Assert-Contains $computeArchitecture "docs/distributed-compute/current-implementation-status.md" "distributed-compute architecture section"
if ($computeArchitecture -match '\bimplementation_(?:uncompiled|unrun|unwired|partially_verified|verified)\b') {
    throw "Distributed-compute maturity labels belong in current-implementation-status.md, not AI_ARCHITECTURE.md."
}
foreach ($withdrawalDoc in @(
    "settlement-withdrawal-request-api.md",
    "settlement-withdrawal-terminal-api.md"
)) {
    Assert-Contains $computeStatus $withdrawalDoc "distributed-compute current status"
    Assert-Contains $computeReadme $withdrawalDoc "distributed-compute README"
}
Assert-Contains $computeReadme "implementation-roadmap.md" "distributed-compute README"
Assert-Contains $computeRoadmap "current-implementation-status.md" "distributed-compute roadmap"
if ($computeReadme -match '(?m)^### F[0-9]') {
    throw "Distributed-compute phase details belong in implementation-roadmap.md, not README.md."
}
if ($computeRoadmap -match '\bimplementation_(?:uncompiled|unrun|unwired|partially_verified|verified)\b') {
    throw "Distributed-compute maturity labels belong in current-implementation-status.md, not implementation-roadmap.md."
}

$manifest = Read-Json ".elon/document-sections.json"
$currentGovernance = $manifest.governance_facets."AI_CURRENT.md"
if ($manifest.assignments."AI_CURRENT.md" -ne "custom:overview") {
    throw "AI_CURRENT.md must remain in the project overview topic."
}
if (
    $currentGovernance.retrieval -ne "required" -or
    $currentGovernance.lifecycle -ne "active" -or
    $currentGovernance.authority -ne "authoritative"
) {
    throw "AI_CURRENT.md must remain a required, active, authoritative current-status document."
}

$retrieval = Read-Json ".elon/document-retrieval-cases.json"
$caseIds = @($retrieval.cases | ForEach-Object { $_.id })
foreach ($caseId in @(
    "current-project-status",
    "rejected-demo-oracle-role",
    "rejected-ai-to-ai-skill-route",
    "distributed-compute-current-status",
    "distributed-compute-implementation-roadmap"
)) {
    if ($caseIds -notcontains $caseId) {
        throw "Document retrieval regression case is missing: $caseId"
    }
}

$federationText = [System.IO.File]::ReadAllText(".elon/knowledge-federation.json")
$null = Read-Json ".elon/knowledge-federation.json"
if ($federationText -match "docs/ai-to-ai-\*\.md") {
    throw "Rejected AI-to-AI documents must not remain in federation include globs."
}

$defaultManifest = Read-Json "default-project-docs/files/elon/default-docs.json"
$defaultSections = Read-Json "default-project-docs/files/elon/document-sections.json"
if (@($defaultManifest.documents.path) -notcontains "AI_CURRENT.md") {
    throw "New projects must seed AI_CURRENT.md."
}
if ($defaultSections.governance_facets."AI_CURRENT.md".retrieval -ne "required") {
    throw "New projects must mark AI_CURRENT.md as required."
}

foreach ($deletedPath in @("docs/ai-to-ai-skill-roadmap.md", "docs/ai-to-ai-skill-oracle-roadmap.md")) {
    if (Test-Path -LiteralPath $deletedPath) {
        throw "Rejected route was restored as a current document: $deletedPath"
    }
}

Write-Host "AI_PROJECT_ENTRYPOINTS=passed"
