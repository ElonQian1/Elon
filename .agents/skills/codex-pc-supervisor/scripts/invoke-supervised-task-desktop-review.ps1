function Request-DesktopReviewBrokerTicket {
    param(
        [object]$Connection,
        [string]$OwnerUserId,
        [string]$RequestedTaskId,
        [string]$Method,
        [string]$EndpointPath,
        [byte[]]$BodyBytes
    )
    if ($null -eq $Connection -or
        -not [bool](Get-ObjectField $Connection 'DesktopReviewBrokerAvailable')) {
        throw 'desktop_review_broker_unavailable: the installed NodeAgent did not start its memory-only Desktop reviewer broker'
    }
    $pipeName = [string](Get-ObjectField $Connection 'DesktopReviewBrokerPipe')
    if ($pipeName -notmatch '^elon-desktop-review-[0-9a-f]{24}$') {
        throw 'desktop_review_broker_unavailable: NodeAgent returned an invalid broker endpoint'
    }
    $sha = [Security.Cryptography.SHA256]::Create()
    try { $bodyHash = -join ($sha.ComputeHash($BodyBytes) | ForEach-Object { $_.ToString('x2') }) } finally { $sha.Dispose() }
    $request = [ordered]@{
        protocol = $script:DesktopReviewBrokerProtocol
        owner_user_id = $OwnerUserId
        task_id = $RequestedTaskId
        method = $Method.ToUpperInvariant()
        endpoint_path = $EndpointPath
        body_sha256 = $bodyHash
    }
    $client = New-Object System.IO.Pipes.NamedPipeClientStream(
        '.', $pipeName, [IO.Pipes.PipeDirection]::InOut, [IO.Pipes.PipeOptions]::None)
    try {
        $client.Connect(3000)
        $writer = New-Object IO.StreamWriter($client, $script:Utf8NoBom, 1024, $true)
        $reader = New-Object IO.StreamReader($client, $script:Utf8NoBomStrict, $false, 1024, $true)
        try {
            $writer.WriteLine(($request | ConvertTo-Json -Compress))
            $writer.Flush()
            $line = $reader.ReadLine()
        } finally {
            $reader.Dispose()
            $writer.Dispose()
        }
    } catch {
        throw "desktop_review_broker_unavailable: $($_.Exception.Message)"
    } finally {
        $client.Dispose()
    }
    if ([string]::IsNullOrWhiteSpace($line)) {
        throw 'desktop_review_broker_unavailable: broker returned no response'
    }
    try { $response = $line | ConvertFrom-Json -ErrorAction Stop }
    catch { throw 'desktop_review_broker_unavailable: broker returned invalid UTF-8 JSON' }
    if (-not [bool](Get-ObjectField $response 'ok')) {
        $code = [string](Get-ObjectField $response 'code')
        if ([string]::IsNullOrWhiteSpace($code)) { $code = 'desktop_review_broker_denied' }
        throw "$($code): Desktop review signing was denied without exposing a private key"
    }
    $ticket = [string](Get-ObjectField $response 'ticket')
    if ($ticket -notmatch '^v3\.') {
        throw 'desktop_review_broker_unavailable: broker response did not contain a v3 ticket'
    }
    return $ticket
}

function New-DesktopReviewTicket {
    param(
        [string]$OwnerUserId,
        [string]$RequestedTaskId,
        [string]$Method,
        [string]$EndpointPath,
        [byte[]]$BodyBytes,
        [object]$Connection = $null
    )
    $stateRoot = if (-not [string]::IsNullOrWhiteSpace($DesktopReviewStateRoot)) { $DesktopReviewStateRoot } elseif (-not [string]::IsNullOrWhiteSpace($StateRoot)) { $StateRoot } else { [string]$env:ELON_DESKTOP_REVIEW_STATE_ROOT }
    $installRoot = if (-not [string]::IsNullOrWhiteSpace($DesktopReviewInstallRoot)) { $DesktopReviewInstallRoot } elseif (-not [string]::IsNullOrWhiteSpace($InstallRoot)) { $InstallRoot } else { [string]$env:ELON_DESKTOP_REVIEW_INSTALL_ROOT }
    if ([string]::IsNullOrWhiteSpace($stateRoot) -or [string]::IsNullOrWhiteSpace($installRoot)) {
        $capabilities = if ($null -ne $Connection) { @((Get-ObjectField $Connection 'SupervisionCapabilities')) } else { @() }
        if ($script:DesktopReviewBrokerCapability -in $capabilities) {
            return Request-DesktopReviewBrokerTicket $Connection $OwnerUserId $RequestedTaskId `
                $Method $EndpointPath $BodyBytes
        }
        throw 'desktop_review_paths_not_configured: set -DesktopReviewStateRoot/-DesktopReviewInstallRoot or ELON_DESKTOP_REVIEW_STATE_ROOT/ELON_DESKTOP_REVIEW_INSTALL_ROOT; from a distinct Desktop Windows identity run <InstallRoot>\_internal\desktop-review-credential.ps1 -Action Diagnose -StateRoot <Desktop-only-state> -InstallRoot <Node-install>; the executor SID cannot act as Desktop reviewer'
    }
    $signer = Join-Path ([IO.Path]::GetFullPath($installRoot)) '_internal\new-desktop-review-ticket.ps1'
    if (Test-Path -LiteralPath $signer -PathType Leaf) {
        $sha = [Security.Cryptography.SHA256]::Create()
        try { $bodyHash = -join ($sha.ComputeHash($BodyBytes) | ForEach-Object { $_.ToString('x2') }) } finally { $sha.Dispose() }
        $ticket = & $signer -OwnerUserId $OwnerUserId -TaskId $RequestedTaskId -Method $Method `
            -EndpointPath $EndpointPath -BodySha256 $bodyHash -StateRoot $stateRoot -InstallRoot $installRoot
        if ($LASTEXITCODE -ne 0 -or [string]::IsNullOrWhiteSpace([string]$ticket)) {
            throw 'desktop_review_signer_unavailable: signer failed or private key/ACL is inaccessible'
        }
        return [string]$ticket
    }
    throw 'desktop_review_signer_missing: configured InstallRoot does not contain the signer'
}
