param()

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

. (Join-Path $PSScriptRoot "node-agent-release-contract.ps1")
. (Join-Path $PSScriptRoot "release-publish-lease.ps1")

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

$unicodeOwner = -join ([char]0x94b1, [char]0x4e00, [char]0x9f99)
$unicodeDevice = (-join ([char]0x4e00, [char]0x9f99)) + '4060'
$unicodeHandshakeFixture = @{
    broadcast_to = 1
    public_dev_handshake = @{
        nodes = @(@{
            owner_nickname = $unicodeOwner
            device_name = $unicodeDevice
            public_dev_handshake_ready = $true
        })
    }
} | ConvertTo-Json -Depth 6 -Compress
$unicodeHandshakeBase64 = [Convert]::ToBase64String(
    [Text.UTF8Encoding]::new($false).GetBytes($unicodeHandshakeFixture)
)
$unicodeHandshake = ConvertFrom-NodeAgentUtf8Base64Json -Value $unicodeHandshakeBase64
Assert-True ($unicodeHandshake.broadcast_to -eq 1) `
    "The remote UTF-8 JSON decoder must preserve numeric release evidence"
Assert-True ($unicodeHandshake.public_dev_handshake.nodes[0].owner_nickname -eq $unicodeOwner) `
    "The remote UTF-8 JSON decoder must preserve Chinese handshake identities on PowerShell 5.1"
$invalidRemoteJsonRejected = $false
try {
    ConvertFrom-NodeAgentUtf8Base64Json -Value 'not-base64' | Out-Null
} catch {
    $invalidRemoteJsonRejected = $_.Exception.Message.Contains('Remote UTF-8 JSON response is invalid')
}
Assert-True $invalidRemoteJsonRejected `
    "Invalid remote Base64 JSON must fail closed"

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
Assert-True ($publishScript.Contains('Enter-NodeAgentPublishLock -Path $PublishLockPath')) `
    "The publisher must acquire the process-wide release lock before building"
Assert-True ($publishScript.Contains('Exit-NodeAgentPublishLock -Lock $PublishLock')) `
    "The publisher must release the process-wide release lock in its finalizer"
Assert-True (Test-ElonNodeAgentLeaseBootstrapFallback `
    -Message 'release API HTTP 400: {"error":"bad-kind","message":"unknown kind: node_agent"}') `
    "The first node-agent release must bootstrap against a server that predates the node_agent lane"
Assert-True (-not (Test-ElonNodeAgentLeaseBootstrapFallback `
    -Message 'release API HTTP 500: {"error":"internal","message":"database unavailable"}')) `
    "Unrelated release API failures must not bypass the global lease"

$lockFixtureRoot = Join-Path ([System.IO.Path]::GetTempPath()) ("elon-node-agent-publish-lock-" + [Guid]::NewGuid().ToString("N"))
$lockFixturePath = Join-Path $lockFixtureRoot "publish.lock"
$publishLock = Enter-NodeAgentPublishLock -Path $lockFixturePath
try {
    Assert-True (Test-Path -LiteralPath $lockFixturePath -PathType Leaf) `
        "The release lock must leave auditable owner metadata"
    $concurrentBlocked = $false
    try {
        $concurrentLock = Enter-NodeAgentPublishLock -Path $lockFixturePath
        Exit-NodeAgentPublishLock -Lock $concurrentLock
    } catch {
        $concurrentBlocked = $_.Exception.Message.Contains('拒绝并发重复发布')
    }
    Assert-True $concurrentBlocked "A concurrent publisher must fail before any build or upload"
} finally {
    Exit-NodeAgentPublishLock -Lock $publishLock
    Remove-Item -LiteralPath $lockFixtureRoot -Recurse -Force -ErrorAction SilentlyContinue
}

$buildScript = Get-Content -LiteralPath (Join-Path $PSScriptRoot "..\server\build.rs") -Raw
Assert-True ($buildScript.Contains('compile_for(&["elon-pc-node"])')) `
    "The Windows resource must be scoped to the node client binary"
Assert-True ($buildScript.Contains('desktop-shell/src-tauri/icons/icon.ico')) `
    "The node client must reuse the checked-in desktop brand ICO"

$repoRoot = [System.IO.Path]::GetFullPath((Join-Path $PSScriptRoot ".."))
Assert-NodeAgentBackgroundGitLaunchPolicy -RepoRoot $repoRoot

$fixtureRoot = Join-Path ([System.IO.Path]::GetTempPath()) ("elon-node-agent-git-gate-" + [Guid]::NewGuid().ToString("N"))
try {
    $fixtureSource = Join-Path $fixtureRoot "server\src"
    New-Item -ItemType Directory -Force -Path $fixtureSource | Out-Null
    [System.IO.File]::WriteAllText(
        (Join-Path $fixtureSource "regression.rs"),
        'fn regression() { let _ = std::process::Command::new("git").output(); }'
    )
    $blocked = $false
    try {
        Assert-NodeAgentBackgroundGitLaunchPolicy -RepoRoot $fixtureRoot | Out-Null
    } catch {
        $blocked = $_.Exception.Message.Contains('elon_pc_dev_runtime::git_command()')
    }
    Assert-True $blocked "The release gate must reject a newly added bare Git launch"
} finally {
    if (Test-Path -LiteralPath $fixtureRoot) {
        Remove-Item -LiteralPath $fixtureRoot -Recurse -Force
    }
}

Write-Host "NODE_AGENT_RELEASE_CONTRACT_TESTS=passed" -ForegroundColor Green
