param()

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

. (Join-Path $PSScriptRoot "node-agent-release-contract.ps1")

function Assert-True {
    param([bool]$Condition, [string]$Message)
    if (-not $Condition) { throw $Message }
}

$target = Get-NodeAgentReleaseIdentity `
    -Version "0.3.69" `
    -GitSha "c97f4b6fd9c5c9ce0e54564663f6677b7b6b8fb2"
Assert-True ($target -eq "0.3.69+c97f4b6fd9c5c9ce0e54564663f6677b7b6b8fb2") `
    "Published identity must include the full Git SHA"
Assert-True ((Get-NodeAgentReleaseIdentity -Version "0.3.69") -eq "0.3.69") `
    "Development identity must keep the Cargo version"

$oldReadyNode = [pscustomobject]@{
    public_dev_handshake_ready = $true
    agent_version = "0.3.69"
}
Assert-True (-not (Test-NodeAgentPublishHandshakeReady `
    -Node $oldReadyNode `
    -TargetReleaseIdentity $target)) `
    "An old build must not pass even when its capability handshake is ready"

$targetReadyNode = [pscustomobject]@{
    public_dev_handshake_ready = $true
    agent_version = $target
}
Assert-True (Test-NodeAgentPublishHandshakeReady `
    -Node $targetReadyNode `
    -TargetReleaseIdentity $target) `
    "The target build should pass after its capability handshake is ready"

$targetPendingNode = [pscustomobject]@{
    public_dev_handshake_ready = $false
    agent_version = $target
}
Assert-True (-not (Test-NodeAgentPublishHandshakeReady `
    -Node $targetPendingNode `
    -TargetReleaseIdentity $target)) `
    "The target build must not pass before its capability handshake is ready"

$brandIcon = Join-Path $PSScriptRoot "..\desktop-shell\src-tauri\icons\icon.ico"
$brandIconSha256 = Get-WindowsBrandIconAssetSha256 -IconPath $brandIcon
Assert-True ($brandIconSha256 -match '^[0-9a-f]{64}$') `
    "The checked-in Windows brand ICO must produce a stable 32px bitmap hash"

$publishScript = Get-Content -LiteralPath (Join-Path $PSScriptRoot "publish-node-agent.ps1") -Raw
Assert-True ($publishScript.Contains('[switch]$RequireAllOnlineTargetBuild')) `
    "The publisher must expose an explicit strict rollout switch"
Assert-True ($publishScript.Contains('NODE_AGENT_TARGET_BUILD_STATUS=partial')) `
    "The publisher must report partial rollout without claiming ready"
Assert-True ($publishScript.Contains('if ($RequireAllOnlineTargetBuild)')) `
    "Strict rollout must remain available when every online node is required"
Assert-True ($publishScript.Contains('Assert-WindowsExecutableBrandIcon -ExecutablePath $WinBin')) `
    "The Windows release build must verify its extracted AssociatedIcon"
Assert-True ($publishScript.Contains('Assert-WindowsExecutableBrandIcon -ExecutablePath $PackageClient')) `
    "The packaged main client must retain the brand icon"
Assert-True ($publishScript.Contains('Assert-WindowsExecutableBrandIcon -ExecutablePath $PackageUninstall')) `
    "The packaged uninstall copy must retain the brand icon"

$buildScript = Get-Content -LiteralPath (Join-Path $PSScriptRoot "..\server\build.rs") -Raw
Assert-True ($buildScript.Contains('compile_for(&["elon-pc-node"])')) `
    "The Windows resource must be scoped to the node client binary"
Assert-True ($buildScript.Contains('desktop-shell/src-tauri/icons/icon.ico')) `
    "The node client must reuse the checked-in desktop brand ICO"

Write-Host "NODE_AGENT_RELEASE_CONTRACT_TESTS=passed" -ForegroundColor Green
