#requires -Version 7.0

function Start-ChatGptComposerDomLease {
    param([Parameter(Mandatory)]$Runtime)
    Assert-ChatGptWebSmokeTrustedDevice -Runtime $Runtime
    $owner = Invoke-ElonNativeCommand -FilePath $Runtime.adb -ArgumentList @(
        '-s', $Runtime.device_serial, 'shell', 'pidof', 'com.elon.app') -TimeoutSeconds 5 -Label 'composer lease app identity'
    Assert-ElonNativeCommand -Result $owner -FailureMessage 'composer_lease_app_missing'
    $appPid = ([string]$owner.Stdout).Trim()
    if ($appPid -notmatch '^[1-9][0-9]*$') { throw 'composer_lease_app_ambiguous' }
    $forward = Invoke-ElonNativeCommand -FilePath $Runtime.adb -ArgumentList @(
        '-s', $Runtime.device_serial, 'forward', 'tcp:0', "localabstract:webview_devtools_remote_$appPid") `
        -TimeoutSeconds 5 -Label 'composer lease local forward'
    Assert-ElonNativeCommand -Result $forward -FailureMessage 'composer_lease_forward_failed'
    $port = ([string]$forward.Stdout).Trim()
    if ($port -notmatch '^[1-9][0-9]{3,4}$' -or [int]$port -gt 65535) { throw 'composer_lease_port_invalid' }
    $lease = @{endpoint="http://127.0.0.1:$port";port=$port;nonce=[Guid]::NewGuid().ToString('N');
        version=(Resolve-ChatGptWebSmokeExpectedAdapterVersion)}
    try {
        $state = Invoke-ChatGptComposerDomLease -Lease $lease -Action hide
        Assert-ChatGptComposerDomUnavailable -State $state
        return $lease
    } catch {
        $failure = $_
        try { Stop-ChatGptComposerDomLease -Runtime $Runtime -Lease $lease | Out-Null } catch { }
        throw $failure
    }
}

function Invoke-ChatGptComposerDomLease {
    param([Parameter(Mandatory)]$Lease, [ValidateSet('hide','state','diagnose','restore')][string]$Action)
    $node = Get-Command node.exe -CommandType Application | Select-Object -First 1
    $result = Invoke-ElonNativeCommand -FilePath $node.Source `
        -ArgumentList @((Join-Path $PSScriptRoot 'chatgpt-composer-dom-lease.cjs'), $Lease.endpoint,
            $Action, $Lease.nonce, [string]$Lease.version) -TimeoutSeconds 30 -Label "composer lease $Action"
    Assert-ElonNativeCommand -Result $result -FailureMessage "composer_lease_${Action}_failed"
    return ([string]$result.Stdout | ConvertFrom-Json)
}

function Assert-ChatGptComposerDomUnavailable {
    param([Parameter(Mandatory)]$State)
    if ($State.schema -cne 'elon.composer_dom_lease.v1' -or $State.active -ne $true -or
        $State.initial_visible -lt 1 -or $State.visible_now -ne 0 -or $State.samples -lt 1 -or
        $State.visible_samples -ne 0 -or $State.invalid_samples -ne 0) { throw 'composer_lease_not_unavailable' }
}

function Stop-ChatGptComposerDomLease {
    param([Parameter(Mandatory)]$Runtime, [Parameter(Mandatory)]$Lease)
    $restored = $false
    try {
        $state = Invoke-ChatGptComposerDomLease -Lease $Lease -Action restore
        $restored = $state.restored -eq $true
    } finally {
        $removed = Invoke-ElonNativeCommand -FilePath $Runtime.adb -ArgumentList @(
            '-s', $Runtime.device_serial, 'forward', '--remove', "tcp:$($Lease.port)") `
            -TimeoutSeconds 5 -Label 'composer lease remove local forward'
        Assert-ElonNativeCommand -Result $removed -FailureMessage 'composer_lease_forward_cleanup_failed'
    }
    return $restored
}
