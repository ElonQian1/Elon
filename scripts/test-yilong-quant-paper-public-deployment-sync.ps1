[CmdletBinding()]
param()

$ErrorActionPreference = "Stop"
$projectRoot = [System.IO.Path]::GetFullPath((Join-Path $PSScriptRoot ".."))
$integrationPath = Join-Path $projectRoot "docs\yilong-quant-integration.md"
$requirementPath = Join-Path $projectRoot "docs\requirements\yilong-quant-paper-public-deployment-contract-sync-v1.md"
$indexPath = Join-Path $projectRoot "AI_INDEX.md"
$catalogPath = Join-Path $projectRoot "server\src\official_project_catalog\catalog.json"
$quantCommit = "0b87604e9105d7b0c1e4ba0da6b8b2c3c43d6ddc"

function Assert-QuantSyncContract {
    param([bool]$Condition, [string]$Message)
    if (-not $Condition) { throw "Yilong Quant public deployment sync contract failed: $Message" }
}

$integration = Get-Content -Raw -Encoding UTF8 $integrationPath
$requirement = Get-Content -Raw -Encoding UTF8 $requirementPath
$index = Get-Content -Raw -Encoding UTF8 $indexPath
$catalog = Get-Content -Raw -Encoding UTF8 $catalogPath | ConvertFrom-Json

foreach ($marker in @(
    $quantCommit,
    "configuration_ready",
    "deployment_contract_verified",
    "environment_deployed",
    "scripts/check-paper-public-deployment.ps1",
    "scope=public_https_read_only",
    "network_calls_made=true",
    "status=ready"
)) {
    Assert-QuantSyncContract $integration.Contains($marker) "integration document is missing $marker"
    Assert-QuantSyncContract $requirement.Contains($marker) "requirement is missing $marker"
}

Assert-QuantSyncContract $index.Contains("docs/requirements/yilong-quant-paper-public-deployment-contract-sync-v1.md") "AI index does not route to the V9 sync requirement"

$quantProjects = @($catalog.projects | Where-Object { $_.id -eq "yilong-quant" })
Assert-QuantSyncContract ($quantProjects.Count -eq 1) "official catalog must contain exactly one yilong-quant project"
$downloads = $quantProjects[0].landing.downloads
foreach ($client in @("web", "windows", "android")) {
    Assert-QuantSyncContract ([string]$downloads.$client.status -ceq "planned") "official catalog $client client must remain planned"
}
$webNote = [string]$downloads.web.note
$notDeployedMarker = -join @([char]0x5C1A, [char]0x672A, [char]0x90E8, [char]0x7F72)
Assert-QuantSyncContract -Condition ($webNote.Contains($notDeployedMarker)) -Message "Web catalog note no longer says the public target is not deployed"

Write-Output "Yilong Quant Paper public deployment sync contract passed."
