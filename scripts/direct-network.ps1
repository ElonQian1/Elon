# Shared direct/no-proxy network helpers for project verification scripts.

function Set-ElonProjectDirectNetwork {
    foreach ($name in @("HTTP_PROXY", "HTTPS_PROXY", "ALL_PROXY", "http_proxy", "https_proxy", "all_proxy")) {
        [System.Environment]::SetEnvironmentVariable($name, $null, "Process")
    }
    [System.Environment]::SetEnvironmentVariable("NO_PROXY", "*", "Process")
    [System.Environment]::SetEnvironmentVariable("no_proxy", "*", "Process")
}

function Set-ElonProjectDirectGitSsh {
    $originUrl = ""
    try {
        $originUrl = [string](& git remote get-url origin 2>$null)
    } catch {
        $originUrl = ""
    }

    if ($originUrl -match "github\.com[:/]") {
        [System.Environment]::SetEnvironmentVariable(
            "GIT_SSH_COMMAND",
            "ssh -o ProxyCommand=none -o ProxyJump=none -o HostName=ssh.github.com -p 443",
            "Process"
        )
    }
}

function Add-ElonProjectDirectRequestParameters {
    param(
        [Parameter(Mandatory = $true)][hashtable]$Params,
        [Parameter(Mandatory = $true)][ValidateSet("Invoke-RestMethod", "Invoke-WebRequest")][string]$CommandName
    )

    Set-ElonProjectDirectNetwork
    $command = Get-Command $CommandName -ErrorAction Stop
    if ($command.Parameters.ContainsKey("NoProxy")) {
        $Params["NoProxy"] = $true
    }
    return $Params
}
