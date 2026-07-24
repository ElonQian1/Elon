param()

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

. (Join-Path $PSScriptRoot "node-agent-release-contract.ps1")
. (Join-Path $PSScriptRoot "release-publish-lease.ps1")
. (Join-Path $PSScriptRoot "node-agent-publish-replay.ps1")

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
$handshakeHelper = Get-Content -LiteralPath (Join-Path $PSScriptRoot "node-agent-publish-handshake.ps1") -Raw
$publishContractText = $publishScript + "`n" + $handshakeHelper
$replayHelper = Get-Content -LiteralPath (Join-Path $PSScriptRoot "node-agent-publish-replay.ps1") -Raw
$leaseHelper = Get-Content -LiteralPath (Join-Path $PSScriptRoot "release-publish-lease.ps1") -Raw
$serverPublishScript = Get-Content -LiteralPath (Join-Path $PSScriptRoot "publish-server.ps1") -Raw
$apkPublishScript = Get-Content -LiteralPath (Join-Path $PSScriptRoot "publish-apk.ps1") -Raw
Assert-True ($leaseHelper.Contains('[Parameter(Mandatory)][string]$Sha')) `
    "Every release heartbeat must carry the immutable claim SHA"
Assert-True ($leaseHelper.Contains('$body.phaseStatus = $Status')) `
    "Internal release phases must not masquerade as top-level stages"
Assert-True ($leaseHelper.Contains('release heartbeat failed closed')) `
    "Background heartbeat failures must be visible to the foreground publisher"
Assert-True (-not $leaseHelper.Contains('Receive-Job -Job $HeartbeatJob -ErrorAction SilentlyContinue')) `
    "Heartbeat shutdown must not swallow background errors"
Assert-True ($serverPublishScript.Contains("Start-ElonReleaseContextHeartbeat -Context `$script:ReleaseContext")) `
    "The server build must hold a visible fail-closed heartbeat"
Assert-True ($serverPublishScript.Contains("-Stage 'server'") -and `
    $serverPublishScript.Contains("Set-ElonReleasePhase -Context `$script:ReleaseContext -Phase 'pc_frontend'")) `
    "The PC frontend build must be an internal server phase until its own stage is committed"
Assert-True ($apkPublishScript.Contains("-Stage 'android_apk'") -and `
    $apkPublishScript.Contains("Set-ElonReleasePhase -Context `$script:ReleaseContext -Phase 'gradle_build'")) `
    "The APK Gradle build must expose a visible heartbeat phase"
Assert-True ($apkPublishScript.Contains("Set-ElonReleasePhase -Context `$script:ReleaseContext -Phase 'artifact_upload'")) `
    "The APK upload must expose a visible heartbeat phase"
Assert-True (-not $publishScript.Contains('-Stage $script:NodeReleaseActiveStage')) `
    "Node internal phases must never write arbitrary top-level stages"
Assert-True ($publishScript.Contains('[switch]$RequireAllOnlineTargetBuild')) `
    "The publisher must expose an explicit strict rollout switch"
Assert-True ($publishScript.Contains('[switch]$IncludeLinux')) `
    "Linux node publishing must be an explicit opt-in"
Assert-True ($publishScript.Contains('NODE_AGENT_LINUX_PUBLISH_STATUS=skipped_default')) `
    "A default Windows release must report that Linux was not published"
Assert-True ($publishScript.Contains("Invoke-RustCacheCargo -ProjectRoot `$RepoRoot -Domain 'node-agent-release'")) `
    "Node release builds must enter the managed shared Rust cache"
Assert-True ($publishScript.Contains('[string]$ReplayPublishedSha')) `
    "The publisher must expose an explicit immutable same-SHA replay mode"
Assert-True ($publishScript.Contains('ReplayPublishedSha was not already published/coalesced')) `
    "Explicit replay must fail closed instead of rebuilding an unknown SHA"
Assert-True ($replayHelper.Contains('merge-base --is-ancestor $sha origin/main')) `
    "Explicit replay must only accept a SHA retained by immutable origin/main history"
Assert-True ($publishContractText.Contains('NODE_AGENT_TARGET_BUILD_STATUS=partial')) `
    "The publisher must report partial rollout without claiming ready"
Assert-True ($publishContractText.Contains('if ($RequireAllOnlineTargetBuild)')) `
    "Strict rollout must remain available when every online node is required"
Assert-True ($publishScript.Contains('[switch]$SynchronousRemote') -and `
    $publishScript.Contains('NODE_AGENT_LOCAL_SERVER_DEPENDENCY=none')) `
    "The default publisher must finish locally while remote release requires worker mode"
Assert-True ($publishScript.Contains('Assert-WindowsExecutableBrandIcon -ExecutablePath $WinBin')) `
    "The Windows release build must verify its extracted AssociatedIcon"
Assert-True ($publishScript.Contains('Assert-WindowsExecutableBrandIcon -ExecutablePath $PackageClient')) `
    "The packaged main client must retain the brand icon"
Assert-True ($publishScript.Contains('Assert-WindowsExecutableBrandIcon -ExecutablePath $PackageUninstall')) `
    "The packaged uninstall copy must retain the brand icon"
Assert-True ($publishScript.Contains('Enter-NodeAgentPublishLock -Path $PublishLockPath') -and `
    $publishScript.Contains('node-agent-local-publish-v1.lock') -and `
    $publishScript.Contains('node-agent-remote-publish-v1.lock')) `
    "Local and remote publisher lanes must each acquire their own process-wide build lock"
Assert-True ($publishScript.Contains('Exit-NodeAgentPublishLock -Lock $PublishLock')) `
    "The publisher must release the process-wide release lock in its finalizer"
Assert-True (Test-ElonNodeAgentLeaseBootstrapFallback `
    -Message 'release API HTTP 400: {"error":"bad-kind","message":"unknown kind: node_agent"}') `
    "The first node-agent release must bootstrap against a server that predates the node_agent lane"
Assert-True (-not (Test-ElonNodeAgentLeaseBootstrapFallback `
    -Message 'release API HTTP 500: {"error":"internal","message":"database unavailable"}')) `
    "Unrelated release API failures must not bypass the global lease"
Assert-True ($leaseHelper.Contains('replayOnly')) `
    "A coalesced node SHA must return an explicit replay action instead of silently ending"
Assert-True ($replayHelper.Contains('Assert-RemoteNodeAgentReplayIdentity')) `
    "Replay must verify immutable remote artifacts before broadcasting"
Assert-True ($replayHelper.Contains('NODE_AGENT_REPLAY_EXE_SHA256=')) `
    "Replay evidence must report the exact EXE SHA-256"
Assert-True ($replayHelper.Contains('NODE_AGENT_REPLAY_CLIENT_SHA256=')) `
    "Replay evidence must report the exact Windows client package SHA-256"
Assert-True ($replayHelper.Contains("if ([string]`$metadata.sha256 -ne `$Identity.ExeSha256")) `
    "Artifact mismatch must fail closed before a replay broadcast"
$replayFixture = [pscustomobject]@{
    Metadata = [pscustomobject]@{
        gitSha = 'same-sha'; sha256 = ('a' * 64); windowsClientSha256 = ('b' * 64)
    }
    ExeSha256 = ('a' * 64)
    ClientSha256 = ('b' * 64)
}
Assert-RemoteNodeAgentReplayIdentity -Identity $replayFixture -ExpectedGitSha 'same-sha'
$staleReplayRejected = $false
$replayFixture.Metadata.sha256 = ('c' * 64)
try {
    Assert-RemoteNodeAgentReplayIdentity -Identity $replayFixture -ExpectedGitSha 'same-sha'
} catch {
    $staleReplayRejected = $_.Exception.Message.Contains('SHA-256')
}
Assert-True $staleReplayRejected `
    "A coalesced replay must reject stale server artifacts before sending the update"

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
