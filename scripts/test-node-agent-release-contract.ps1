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

Write-Host "NODE_AGENT_RELEASE_CONTRACT_TESTS=passed" -ForegroundColor Green
