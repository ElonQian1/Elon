function Get-NodeAgentPublishSshOptions {
    return @(
        '-o', 'BatchMode=yes',
        '-o', 'ConnectionAttempts=1',
        '-o', 'ConnectTimeout=10',
        '-o', 'ServerAliveInterval=5',
        '-o', 'ServerAliveCountMax=2',
        '-o', 'IPQoS=none',
        '-o', 'ProxyCommand=none',
        '-o', 'ProxyJump=none'
    )
}

function Invoke-NodeAgentPublishSshRaw {
    param(
        [Parameter(Mandatory)] [string]$Server,
        [Parameter(Mandatory)] [string]$Command,
        [ValidateRange(1, 600)] [int]$TimeoutSeconds = 45
    )

    return Invoke-ElonNativeCommand -FilePath 'ssh.exe' `
        -ArgumentList ((Get-NodeAgentPublishSshOptions) + @($Server, $Command)) `
        -TimeoutSeconds $TimeoutSeconds -Label 'node-agent remote command'
}

function Invoke-NodeAgentPublishSsh {
    param(
        [Parameter(Mandatory)] [string]$Server,
        [Parameter(Mandatory)] [string]$Command,
        [ValidateRange(1, 600)] [int]$TimeoutSeconds = 45
    )

    $result = Invoke-NodeAgentPublishSshRaw -Server $Server -Command $Command -TimeoutSeconds $TimeoutSeconds
    if ($result.ExitCode -ne 0) {
        throw "Node-agent SSH failed ($($result.ExitCode)): $($result.Stderr)"
    }
    return ([string]$result.Stdout).Trim()
}

function Invoke-NodeAgentPublishScp {
    param(
        [Parameter(Mandatory)] [string]$Source,
        [Parameter(Mandatory)] [string]$Destination,
        [ValidateRange(1, 600)] [int]$TimeoutSeconds = 240
    )

    $result = Invoke-ElonNativeCommand -FilePath 'scp.exe' `
        -ArgumentList ((Get-NodeAgentPublishSshOptions) + @($Source, $Destination)) `
        -TimeoutSeconds $TimeoutSeconds -Label 'node-agent artifact upload'
    if ($result.ExitCode -ne 0) {
        throw "Node-agent SCP failed ($($result.ExitCode)): $($result.Stderr)"
    }
}
