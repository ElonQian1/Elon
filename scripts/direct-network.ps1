# Shared direct/no-proxy network helpers for project verification scripts and
# the bounded GitHub SSH fallback used by Windows workflow entrypoints.

$script:ElonProxyEnvironmentNames = @(
    "HTTP_PROXY", "HTTPS_PROXY", "ALL_PROXY", "http_proxy", "https_proxy", "all_proxy",
    "NO_PROXY", "no_proxy", "GIT_SSH_COMMAND", "GIT_SSH_VARIANT", "SHELL"
)
$script:ElonDefaultGitHubSshProxy = "http://127.0.0.1:17891"

function Set-ElonProjectDirectNetwork {
    foreach ($name in @("HTTP_PROXY", "HTTPS_PROXY", "ALL_PROXY", "http_proxy", "https_proxy", "all_proxy")) {
        [System.Environment]::SetEnvironmentVariable($name, $null, "Process")
    }
    [System.Environment]::SetEnvironmentVariable("NO_PROXY", "*", "Process")
    [System.Environment]::SetEnvironmentVariable("no_proxy", "*", "Process")
}

function Get-ElonProcessEnvironmentSnapshot {
    $snapshot = @()
    foreach ($name in $script:ElonProxyEnvironmentNames) {
        $snapshot += [pscustomobject]@{
            Name = $name
            Value = [System.Environment]::GetEnvironmentVariable($name, "Process")
        }
    }
    return $snapshot
}

function Restore-ElonProcessEnvironment {
    param([Parameter(Mandatory = $true)][object[]]$Snapshot)

    foreach ($entry in $Snapshot) {
        [System.Environment]::SetEnvironmentVariable($entry.Name, $entry.Value, "Process")
    }
}

function New-ElonGenericDirectSshCommand {
    return "ssh -o BatchMode=yes -o ConnectionAttempts=1 -o ConnectTimeout=10 -o ServerAliveInterval=5 -o ServerAliveCountMax=1 -o ProxyCommand=none -o ProxyJump=none"
}

function New-ElonGitHubDirectSshCommand {
    return "$(New-ElonGenericDirectSshCommand) -o HostName=ssh.github.com -p 443 -o StrictHostKeyChecking=accept-new"
}

function Set-ElonProjectDirectGitSsh {
    param(
        [string]$RepoPath = ".",
        [string]$RemoteName = "origin"
    )

    $originUrl = ""
    try {
        $originUrl = [string](& git -C $RepoPath remote get-url $RemoteName 2>$null)
    } catch {
        $originUrl = ""
    }

    if (Test-ElonGitHubSshRemote -RemoteUrl $originUrl) {
        [System.Environment]::SetEnvironmentVariable(
            "GIT_SSH_COMMAND",
            (New-ElonGitHubDirectSshCommand),
            "Process"
        )
    }
}

function Test-ElonGitHubSshRemote {
    param([AllowEmptyString()][string]$RemoteUrl)

    $value = ([string]$RemoteUrl).Trim()
    return $value -match '^(?:ssh://)?(?:[^/@:]+@)?(?:github\.com|ssh\.github\.com)(?::\d+)?[/:]'
}

function Get-ElonGitRemoteUrl {
    param(
        [Parameter(Mandatory = $true)][string]$RepoPath,
        [Parameter(Mandatory = $true)][string]$RemoteName,
        [Parameter(Mandatory = $true)][string]$Operation,
        [string]$GitExecutable = "git",
        [string[]]$GitExecutablePrefixArguments = @()
    )

    $urlArgs = @("-C", $RepoPath, "remote", "get-url")
    if ($Operation -eq "push") { $urlArgs += "--push" }
    $urlArgs += $RemoteName
    $result = Invoke-ElonNativeCapture -FilePath $GitExecutable `
        -Arguments ($GitExecutablePrefixArguments + $urlArgs)
    if ($result.ExitCode -ne 0) { return $RemoteName }
    return ($result.Output | Select-Object -First 1)
}

function Get-ElonGitHubFailureClass {
    param([AllowEmptyString()][string]$Output)

    $text = [string]$Output
    if ($text -match '(?i)(permission denied|authentication failed|repository not found|publickey|access denied|invalid username|invalid password)') {
        return "authentication"
    }
    if ($text -match '(?i)(host key verification failed|remote host identification has changed|known_hosts|no matching host key|no matching key exchange|protocol error|unsupported protocol|non-fast-forward|fetch first|rejected|remote rejected|protected branch|pre-receive hook declined)') {
        return "policy_or_protocol"
    }
    if ($text -match '(?i)(could not resolve host(?:name)?|temporary failure in name resolution|name or service not known|failed to connect|connection timed out|connect timeout|operation timed out|connection reset|connection refused|connection closed|connection aborted|network is unreachable|no route to host|broken pipe|transport endpoint|kex_exchange_identification.*(?:closed|reset)|ssh_exchange_identification.*(?:closed|reset)|unexpected eof|early eof|remote end hung up unexpectedly)') {
        return "network"
    }
    return "other"
}

function Get-ElonGitFailureHint {
    param(
        [AllowEmptyString()][string]$Output,
        [string]$Operation = "Git"
    )

    switch (Get-ElonGitHubFailureClass -Output $Output) {
        "authentication" { return "$Operation failed because GitHub authentication or repository access was denied." }
        "policy_or_protocol" { return "$Operation was rejected by host-key, protocol, or remote business policy; proxy fallback was not attempted." }
        "network" { return "$Operation encountered a GitHub DNS or transport connection error." }
        default { return "$Operation failed with an unclassified error." }
    }
}

function Resolve-ElonLoopbackHttpProxy {
    param([AllowEmptyString()][string]$ProxyUrl)

    if ([string]::IsNullOrWhiteSpace($ProxyUrl)) {
        return [pscustomobject]@{ Valid = $false; Reason = "missing"; Host = ""; Port = 0; Display = "" }
    }
    try {
        $uri = [Uri]$ProxyUrl
    } catch {
        return [pscustomobject]@{ Valid = $false; Reason = "invalid_uri"; Host = ""; Port = 0; Display = "" }
    }
    if ($uri.Scheme -ne "http") {
        return [pscustomobject]@{ Valid = $false; Reason = "http_required"; Host = ""; Port = 0; Display = "" }
    }
    if (-not [string]::IsNullOrEmpty($uri.UserInfo)) {
        return [pscustomobject]@{ Valid = $false; Reason = "credentials_forbidden"; Host = ""; Port = 0; Display = "" }
    }
    if ($uri.AbsolutePath -ne "/" -or -not [string]::IsNullOrEmpty($uri.Query) -or -not [string]::IsNullOrEmpty($uri.Fragment)) {
        return [pscustomobject]@{ Valid = $false; Reason = "endpoint_only"; Host = ""; Port = 0; Display = "" }
    }

    $proxyHost = $uri.DnsSafeHost
    $isLoopback = $proxyHost -eq "localhost"
    if (-not $isLoopback) {
        $address = $null
        if ([System.Net.IPAddress]::TryParse($proxyHost, [ref]$address)) {
            $isLoopback = [System.Net.IPAddress]::IsLoopback($address)
        }
    }
    if (-not $isLoopback -or $uri.Port -lt 1 -or $uri.Port -gt 65535) {
        return [pscustomobject]@{ Valid = $false; Reason = "loopback_required"; Host = ""; Port = 0; Display = "" }
    }

    $hostText = if ($proxyHost -match ':') { "[$proxyHost]" } else { $proxyHost }
    return [pscustomobject]@{
        Valid = $true
        Reason = "ok"
        Host = $proxyHost
        Port = $uri.Port
        Display = "${hostText}:$($uri.Port)"
    }
}

function Get-ElonGitHubSshProxy {
    param([AllowEmptyString()][string]$ExplicitProxyUrl)

    if (-not [string]::IsNullOrWhiteSpace($ExplicitProxyUrl)) {
        return Resolve-ElonLoopbackHttpProxy -ProxyUrl $ExplicitProxyUrl
    }
    $configured = [System.Environment]::GetEnvironmentVariable("ELON_GITHUB_SSH_FALLBACK_PROXY", "Process")
    if (-not [string]::IsNullOrWhiteSpace($configured)) {
        return Resolve-ElonLoopbackHttpProxy -ProxyUrl $configured
    }
    foreach ($name in @("HTTPS_PROXY", "https_proxy", "HTTP_PROXY", "http_proxy", "ALL_PROXY", "all_proxy")) {
        $candidate = [System.Environment]::GetEnvironmentVariable($name, "Process")
        if ([string]::IsNullOrWhiteSpace($candidate)) { continue }
        $resolved = Resolve-ElonLoopbackHttpProxy -ProxyUrl $candidate
        if ($resolved.Valid) { return $resolved }
    }
    return Resolve-ElonLoopbackHttpProxy -ProxyUrl $script:ElonDefaultGitHubSshProxy
}

function Resolve-ElonGitConnectExecutable {
    param([AllowEmptyString()][string]$ExplicitPath)

    if (-not [string]::IsNullOrWhiteSpace($ExplicitPath)) {
        $candidate = [System.IO.Path]::GetFullPath($ExplicitPath)
        if ((Split-Path -Leaf $candidate) -ieq "connect.exe" -and (Test-Path -LiteralPath $candidate -PathType Leaf)) {
            return $candidate
        }
        return ""
    }

    $gitCommand = Get-Command git.exe -ErrorAction SilentlyContinue | Select-Object -First 1
    if (-not $gitCommand) { return "" }
    $gitRoot = Split-Path (Split-Path $gitCommand.Source -Parent) -Parent
    $candidate = Join-Path $gitRoot "mingw64\bin\connect.exe"
    if (Test-Path -LiteralPath $candidate -PathType Leaf) { return $candidate }
    return ""
}

function Resolve-ElonGitBashExecutable {
    param(
        [AllowEmptyString()][string]$ExplicitPath,
        [AllowEmptyString()][string]$ConnectExecutable
    )

    if (-not [string]::IsNullOrWhiteSpace($ExplicitPath)) {
        $candidate = [System.IO.Path]::GetFullPath($ExplicitPath)
        if ((Split-Path -Leaf $candidate) -ieq "bash.exe" -and (Test-Path -LiteralPath $candidate -PathType Leaf)) {
            return $candidate
        }
        return ""
    }
    if ([string]::IsNullOrWhiteSpace($ConnectExecutable)) { return "" }
    $gitRoot = Split-Path (Split-Path (Split-Path $ConnectExecutable -Parent) -Parent) -Parent
    foreach ($relative in @("bin\bash.exe", "usr\bin\bash.exe")) {
        $candidate = Join-Path $gitRoot $relative
        if (Test-Path -LiteralPath $candidate -PathType Leaf) { return $candidate }
    }
    return ""
}

function New-ElonGitHubProxySshCommand {
    param(
        [Parameter(Mandatory = $true)][string]$ConnectExecutable,
        [Parameter(Mandatory = $true)]$Proxy
    )

    $connectPath = ([System.IO.Path]::GetFullPath($ConnectExecutable) -replace '\\', '/')
    if ($connectPath.Contains("'") -or $connectPath.Contains('"')) {
        throw "connect.exe path contains unsupported quote characters."
    }
    $proxyCommand = "`"$connectPath`" -H $($Proxy.Display) %h %p"
    return "ssh -o BatchMode=yes -o ConnectionAttempts=1 -o ConnectTimeout=10 -o ServerAliveInterval=5 -o ServerAliveCountMax=1 -o 'ProxyCommand=$proxyCommand' -o ProxyJump=none -o HostName=ssh.github.com -p 443 -o StrictHostKeyChecking=accept-new"
}

function Invoke-ElonNativeCapture {
    param(
        [Parameter(Mandatory = $true)][string]$FilePath,
        [Parameter(Mandatory = $true)][string[]]$Arguments
    )

    $oldPreference = $ErrorActionPreference
    $ErrorActionPreference = "Continue"
    try {
        $output = @(& $FilePath @Arguments 2>&1)
        $exitCode = $LASTEXITCODE
    } catch {
        $output = @($_.Exception.Message)
        $exitCode = 127
    } finally {
        $ErrorActionPreference = $oldPreference
    }
    return [pscustomobject]@{
        ExitCode = $exitCode
        Output = $output
        Text = (($output | ForEach-Object { [string]$_ }) -join "`n").Trim()
    }
}

function Invoke-ElonGitHubGitWithProxyFallback {
    param(
        [Parameter(Mandatory = $true)][string]$RepoPath,
        [Parameter(Mandatory = $true)][string[]]$GitArgs,
        [string]$RemoteName = "origin",
        [AllowEmptyString()][string]$RemoteUrl = "",
        [AllowEmptyString()][string]$ProxyUrl = "",
        [AllowEmptyString()][string]$ConnectExecutablePath = "",
        [AllowEmptyString()][string]$GitShellPath = "",
        [string]$GitExecutable = "git",
        [string[]]$GitExecutablePrefixArguments = @()
    )

    if ($GitArgs.Count -eq 0 -or $GitArgs[0] -notin @("fetch", "ls-remote", "push")) {
        throw "GitHub SSH fallback only supports fetch, ls-remote, and push."
    }
    $operation = $GitArgs[0]
    $resolvedRemoteUrl = $RemoteUrl
    if ([string]::IsNullOrWhiteSpace($resolvedRemoteUrl)) {
        $resolvedRemoteUrl = Get-ElonGitRemoteUrl -RepoPath $RepoPath -RemoteName $RemoteName `
            -Operation $operation -GitExecutable $GitExecutable `
            -GitExecutablePrefixArguments $GitExecutablePrefixArguments
    }
    $isGitHubSsh = Test-ElonGitHubSshRemote -RemoteUrl $resolvedRemoteUrl
    $proxy = Get-ElonGitHubSshProxy -ExplicitProxyUrl $ProxyUrl
    $connectExecutable = Resolve-ElonGitConnectExecutable -ExplicitPath $ConnectExecutablePath
    $gitShell = Resolve-ElonGitBashExecutable -ExplicitPath $GitShellPath -ConnectExecutable $connectExecutable
    $snapshot = Get-ElonProcessEnvironmentSnapshot
    $baseArgs = @("-C", $RepoPath, "-c", "http.proxy=", "-c", "https.proxy=") + $GitArgs

    try {
        Set-ElonProjectDirectNetwork
        if ($isGitHubSsh) {
            [System.Environment]::SetEnvironmentVariable("GIT_SSH_COMMAND", (New-ElonGitHubDirectSshCommand), "Process")
            [System.Environment]::SetEnvironmentVariable("GIT_SSH_VARIANT", "ssh", "Process")
        } else {
            [System.Environment]::SetEnvironmentVariable("GIT_SSH_COMMAND", (New-ElonGenericDirectSshCommand), "Process")
            [System.Environment]::SetEnvironmentVariable("GIT_SSH_VARIANT", "ssh", "Process")
        }
        $direct = Invoke-ElonNativeCapture -FilePath $GitExecutable `
            -Arguments ($GitExecutablePrefixArguments + $baseArgs)
        $failureClass = Get-ElonGitHubFailureClass -Output $direct.Text
        if ($direct.ExitCode -eq 0 -or -not $isGitHubSsh -or $failureClass -ne "network") {
            return [pscustomobject]@{
                ExitCode = $direct.ExitCode
                Output = $direct.Output
                Text = $direct.Text
                Route = "direct"
                ProxyAttempted = $false
                FailureClass = $failureClass
                Hint = Get-ElonGitFailureHint -Output $direct.Text -Operation $operation
            }
        }
        if (-not $proxy.Valid -or [string]::IsNullOrWhiteSpace($connectExecutable) -or [string]::IsNullOrWhiteSpace($gitShell)) {
            $reason = if (-not $proxy.Valid) {
                "unsafe_or_missing_proxy"
            } elseif ([string]::IsNullOrWhiteSpace($connectExecutable)) {
                "connect_exe_unavailable"
            } else {
                "git_bash_unavailable"
            }
            return [pscustomobject]@{
                ExitCode = $direct.ExitCode
                Output = $direct.Output
                Text = $direct.Text
                Route = "direct"
                ProxyAttempted = $false
                FailureClass = "network"
                Hint = "$(Get-ElonGitFailureHint -Output $direct.Text -Operation $operation) Safe proxy fallback unavailable: $reason."
            }
        }

        [System.Environment]::SetEnvironmentVariable("SHELL", $gitShell, "Process")
        [System.Environment]::SetEnvironmentVariable(
            "GIT_SSH_COMMAND",
            (New-ElonGitHubProxySshCommand -ConnectExecutable $connectExecutable -Proxy $proxy),
            "Process"
        )
        $fallback = Invoke-ElonNativeCapture -FilePath $GitExecutable `
            -Arguments ($GitExecutablePrefixArguments + $baseArgs)
        $fallbackClass = Get-ElonGitHubFailureClass -Output $fallback.Text
        return [pscustomobject]@{
            ExitCode = $fallback.ExitCode
            Output = $fallback.Output
            Text = $fallback.Text
            Route = "proxy"
            ProxyAttempted = $true
            FailureClass = $fallbackClass
            Hint = if ($fallback.ExitCode -eq 0) { "GitHub SSH proxy fallback succeeded via $($proxy.Display)." } else { "GitHub SSH proxy fallback failed once via $($proxy.Display)." }
        }
    } finally {
        Restore-ElonProcessEnvironment -Snapshot $snapshot
    }
}

function Invoke-ElonGitHubGitRequired {
    param(
        [Parameter(Mandatory = $true)][string]$RepoPath,
        [Parameter(Mandatory = $true)][string[]]$GitArgs,
        [string]$RemoteName = "origin"
    )

    $result = Invoke-ElonGitHubGitWithProxyFallback -RepoPath $RepoPath -GitArgs $GitArgs -RemoteName $RemoteName
    if ($result.ProxyAttempted) { Write-Host "GITHUB_SSH_FALLBACK=$($result.Route):$($result.ExitCode)" }
    if ($result.ExitCode -ne 0) {
        throw "git $($GitArgs -join ' ') failed. $($result.Hint)`n$($result.Text)"
    }
    return $result
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

if ($MyInvocation.InvocationName -ne "." -and $args.Count -gt 0) {
    $remoteName = if ($args.Count -gt 1) { [string]$args[1] } else { "origin" }
    $result = Invoke-ElonGitHubGitWithProxyFallback -RepoPath (Get-Location).Path -GitArgs @($args) -RemoteName $remoteName
    if ($result.ProxyAttempted) { Write-Host "GITHUB_SSH_FALLBACK=$($result.Route):$($result.ExitCode)" }
    $result.Output | ForEach-Object { Write-Output $_ }
    if ($result.ExitCode -ne 0) { Write-Error $result.Hint }
    exit $result.ExitCode
}
