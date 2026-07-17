function Get-NodeAgentReleaseIdentity {
    param(
        [Parameter(Mandatory = $true)][string]$Version,
        [string]$GitSha = ""
    )

    $versionValue = $Version.Trim()
    $gitShaValue = $GitSha.Trim()
    if ([string]::IsNullOrWhiteSpace($gitShaValue)) {
        return $versionValue
    }
    return "${versionValue}+${gitShaValue}"
}

function Test-NodeAgentPublishHandshakeReady {
    param(
        [Parameter(Mandatory = $true)]$Node,
        [Parameter(Mandatory = $true)][string]$TargetReleaseIdentity
    )

    if (-not $Node.public_dev_handshake_ready) {
        return $false
    }
    $reportedIdentity = [string]$Node.agent_version
    return [string]::Equals(
        $reportedIdentity.Trim(),
        $TargetReleaseIdentity.Trim(),
        [System.StringComparison]::OrdinalIgnoreCase
    )
}
