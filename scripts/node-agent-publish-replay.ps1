Set-StrictMode -Version Latest

function Resolve-NodeAgentPublishSha {
    param([string]$RepoRoot, [string]$ReplayPublishedSha)
    if ([string]::IsNullOrWhiteSpace($ReplayPublishedSha)) {
        return Assert-NodeAgentPublishHeadCurrent -Phase 'publish start'
    }
    Invoke-GitFetchMain
    $sha = $ReplayPublishedSha.Trim().ToLowerInvariant()
    if ($sha -notmatch '^[0-9a-f]{40}$') { throw 'ReplayPublishedSha must be a full 40-character Git SHA.' }
    git -C $RepoRoot merge-base --is-ancestor $sha origin/main
    if ($LASTEXITCODE -ne 0) { throw 'ReplayPublishedSha is not an ancestor of immutable origin/main.' }
    return $sha
}

function Get-RemoteNodeAgentArtifactIdentity {
    $script = @'
set -eu
cd /opt/elon/data/downloads
test -f node-agent-version.json
test -f elon-pc-node.exe
test -f elon-node-agent-windows.zip
printf '%s\n' "$(base64 < node-agent-version.json | tr -d '\n')"
sha256sum elon-pc-node.exe | awk '{print $1}'
sha256sum elon-node-agent-windows.zip | awk '{print $1}'
if test -f elon-node-agent-windows-setup.exe; then
  sha256sum elon-node-agent-windows-setup.exe | awk '{print $1}'
else
  printf '%s\n' legacy-none
fi
'@
    $raw = Invoke-RemoteBash -Script $script
    $lines = @($raw -split "`n")
    if ($lines.Count -lt 4) { throw 'Remote node artifact identity is incomplete; replay refused.' }
    [pscustomobject]@{
        Metadata = ConvertFrom-NodeAgentUtf8Base64Json -Value $lines[0]
        ExeSha256 = ([string]$lines[1]).Trim().ToLowerInvariant()
        ClientSha256 = ([string]$lines[2]).Trim().ToLowerInvariant()
        InstallerSha256 = if (([string]$lines[3]).Trim() -eq 'legacy-none') {
            ''
        } else {
            ([string]$lines[3]).Trim().ToLowerInvariant()
        }
    }
}

function Assert-RemoteNodeAgentReplayIdentity {
    param([Parameter(Mandatory)][object]$Identity, [Parameter(Mandatory)][string]$ExpectedGitSha)
    $metadata = $Identity.Metadata
    if ([string]$metadata.gitSha -ne $ExpectedGitSha) {
        throw "Remote artifact gitSha=$($metadata.gitSha) differs from replay SHA=$ExpectedGitSha; broadcast refused."
    }
    $metadataInstallerSha256 = ''
    if ($metadata.PSObject.Properties.Name -contains 'windowsInstallerSha256') {
        $metadataInstallerSha256 = [string]$metadata.windowsInstallerSha256
    }
    if ([string]$metadata.sha256 -ne $Identity.ExeSha256 -or
        [string]$metadata.windowsClientSha256 -ne $Identity.ClientSha256 -or
        (-not [string]::IsNullOrWhiteSpace($metadataInstallerSha256) -and
            $metadataInstallerSha256 -ne $Identity.InstallerSha256)) {
        throw 'Remote metadata differs from actual EXE/client/installer SHA-256; stale artifact broadcast refused.'
    }
}

function Invoke-NodeAgentPublishReplay {
    param(
        [string]$GitSha, [string]$PackageVersion, [string]$BatchId,
        [string]$ReleaseIdentity, [bool]$SkipBroadcast, [string]$BroadcastAdminToken,
        [bool]$UseRemoteAdminToken, [int]$HandshakeWaitSec, [string]$BaseUrl
    )
    $identity = Get-RemoteNodeAgentArtifactIdentity
    Assert-RemoteNodeAgentReplayIdentity -Identity $identity -ExpectedGitSha $GitSha
    if ($SkipBroadcast) {
        Write-Host '  Same-SHA artifacts verified; -SkipBroadcast suppressed the online replay.' -ForegroundColor Yellow
    } elseif (-not [string]::IsNullOrWhiteSpace($BroadcastAdminToken)) {
        Invoke-NoProxyJson -Uri "$BaseUrl/api/admin/nodes/push-update" -Method Post `
            -Headers @{ Authorization = "Bearer $BroadcastAdminToken" } -Body '{}' -TimeoutSec 20 | Out-Null
    } else {
        Invoke-RemoteNodeAgentUpdateBroadcast | Out-Null
    }
    if (-not $SkipBroadcast) {
        Wait-NodePublicDevHandshake -Token $BroadcastAdminToken -UseRemoteToken $UseRemoteAdminToken `
            -TimeoutSec $HandshakeWaitSec -TargetReleaseIdentity $ReleaseIdentity
    }
    Write-Host "NODE_AGENT_REPLAY_BATCH_ID=$BatchId"
    Write-Host "NODE_AGENT_REPLAY_VERSION=$PackageVersion"
    Write-Host "NODE_AGENT_REPLAY_GIT_SHA=$GitSha"
    Write-Host "NODE_AGENT_REPLAY_EXE_SHA256=$($identity.ExeSha256)"
    Write-Host "NODE_AGENT_REPLAY_CLIENT_SHA256=$($identity.ClientSha256)"
    Write-Host "NODE_AGENT_REPLAY_INSTALLER_SHA256=$($identity.InstallerSha256)"
}
