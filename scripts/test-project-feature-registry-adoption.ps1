param(
    [string]$RegistryPath = ".elon\project-features.json",
    [string]$FeatureId = "project-feature-registry-adoption-v1"
)

$ErrorActionPreference = "Stop"

function Stop-RegistryAdoptionTest {
    param([string]$Message)
    Write-Error $Message
    exit 1
}

function Assert-RegistryAdoption {
    param(
        [bool]$Condition,
        [string]$Message
    )
    if (-not $Condition) {
        Stop-RegistryAdoptionTest $Message
    }
}

function Get-NormalizedTextSha256 {
    param([string]$Path)

    $text = [System.IO.File]::ReadAllText($Path, [System.Text.Encoding]::UTF8)
    $normalizedText = $text.Replace("`r`n", "`n").Replace("`r", "`n")
    $bytes = [System.Text.UTF8Encoding]::new($false).GetBytes($normalizedText)
    $sha256 = [System.Security.Cryptography.SHA256]::Create()
    try {
        return (($sha256.ComputeHash($bytes) | ForEach-Object { $_.ToString("x2") }) -join "")
    } finally {
        $sha256.Dispose()
    }
}

function Get-RepoRoot {
    $root = (& git rev-parse --show-toplevel).Trim()
    if ($LASTEXITCODE -ne 0 -or [string]::IsNullOrWhiteSpace($root)) {
        Stop-RegistryAdoptionTest "Current directory is not inside a git repository."
    }
    return $root
}

$repoRoot = Get-RepoRoot
Set-Location $repoRoot

$registryFullPath = Join-Path $repoRoot $RegistryPath
Assert-RegistryAdoption (Test-Path -LiteralPath $registryFullPath -PathType Leaf) "Feature registry is missing: $RegistryPath"

$registryText = [System.IO.File]::ReadAllText($registryFullPath, [System.Text.Encoding]::UTF8)
Assert-RegistryAdoption ([System.Text.Encoding]::UTF8.GetByteCount($registryText) -le 65536) "Initial feature registry exceeds the 64 KiB bounded-read budget."

try {
    $registry = $registryText | ConvertFrom-Json
} catch {
    Stop-RegistryAdoptionTest "Feature registry is not valid JSON: $($_.Exception.Message)"
}

Assert-RegistryAdoption ($registry.version -eq 1) "Feature registry version must be 1."
$features = @($registry.features)
Assert-RegistryAdoption ($features.Count -ge 1 -and $features.Count -le 12) "Initial feature registry must contain between 1 and 12 reviewed features."

$featureIds = @($features | ForEach-Object { [string]$_.id })
Assert-RegistryAdoption (($featureIds | Select-Object -Unique).Count -eq $featureIds.Count) "Feature registry contains duplicate feature IDs."

$feature = @($features | Where-Object { $_.id -eq $FeatureId } | Select-Object -First 1)
Assert-RegistryAdoption ($feature.Count -eq 1) "Required adoption feature is missing: $FeatureId"
$feature = $feature[0]

$allowedStatuses = @("proposed", "accepted", "ready", "claimed", "in_progress", "implemented", "verified", "released", "rejected", "archived")
Assert-RegistryAdoption ($allowedStatuses -contains [string]$feature.status) "Adoption feature has an unsupported status."
Assert-RegistryAdoption (-not [string]::IsNullOrWhiteSpace([string]$feature.summary)) "Adoption feature summary is missing."
Assert-RegistryAdoption ([string]$feature.summary -notmatch "[\r\n]") "Adoption feature summary must remain a single bounded line."

$requirementRelativePath = [string]$feature.requirement.path
Assert-RegistryAdoption (-not [System.IO.Path]::IsPathRooted($requirementRelativePath)) "Requirement path must remain repository-relative."
$requirementFullPath = Join-Path $repoRoot $requirementRelativePath
Assert-RegistryAdoption (Test-Path -LiteralPath $requirementFullPath -PathType Leaf) "Bound requirement is missing: $requirementRelativePath"

$actualRequirementHash = Get-NormalizedTextSha256 -Path $requirementFullPath
$expectedRequirementHash = ([string]$feature.requirement.content_hash).ToLowerInvariant()
Assert-RegistryAdoption ($actualRequirementHash -eq $expectedRequirementHash) "Requirement content hash drifted; review it through project_feature_workflow before work continues."

$requiredTaskPaths = @(
    ".elon/project-features.json",
    "AI_CURRENT.md",
    "docs/requirements/project-feature-registry-adoption-v1.md"
)
$taskPaths = @($feature.task_paths | ForEach-Object { ([string]$_).Replace("\", "/") })
foreach ($requiredTaskPath in $requiredTaskPaths) {
    Assert-RegistryAdoption ($taskPaths -contains $requiredTaskPath) "Adoption feature is missing task path: $requiredTaskPath"
}

$criteria = @($feature.acceptance_criteria)
Assert-RegistryAdoption ($criteria.Count -ge 1 -and $criteria.Count -le 32) "Acceptance criteria must remain present and bounded."

$currentPath = Join-Path $repoRoot "AI_CURRENT.md"
Assert-RegistryAdoption (Test-Path -LiteralPath $currentPath -PathType Leaf) "AI_CURRENT.md is missing."
$currentText = [System.IO.File]::ReadAllText($currentPath, [System.Text.Encoding]::UTF8)
Assert-RegistryAdoption ($currentText.Contains(".elon/project-features.json")) "AI_CURRENT.md does not expose the project feature registry entry point."
Assert-RegistryAdoption ($currentText.Contains("project_feature_workflow")) "AI_CURRENT.md does not route agents through the feature workflow."

Write-Host "PROJECT_FEATURE_REGISTRY_ADOPTION=passed features=$($features.Count) status=$($feature.status) evidence=$(@($feature.implementation_evidence).Count)"
