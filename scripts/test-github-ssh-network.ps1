[CmdletBinding()]
param()

$ErrorActionPreference = "Stop"
$repoRoot = [System.IO.Path]::GetFullPath((Join-Path $PSScriptRoot ".."))
. (Join-Path $PSScriptRoot "direct-network.ps1")

function Assert-True {
    param([bool]$Condition, [string]$Message)
    if (-not $Condition) { throw "ASSERTION FAILED: $Message" }
}

function Assert-Equal {
    param($Expected, $Actual, [string]$Message)
    if ([string]$Expected -ne [string]$Actual) {
        throw "ASSERTION FAILED: $Message expected=<$Expected> actual=<$Actual>"
    }
}

function Get-CallLines {
    param([string]$Path)
    if (-not (Test-Path -LiteralPath $Path -PathType Leaf)) { return @() }
    return @(Get-Content -LiteralPath $Path | Where-Object { -not [string]::IsNullOrWhiteSpace($_) })
}

$fixtureRoot = Join-Path ([System.IO.Path]::GetTempPath()) ("elon-github-ssh-test-" + [Guid]::NewGuid().ToString("N"))
$stubPath = Join-Path $fixtureRoot "fake-git.ps1"
$stubCommandPath = Join-Path $fixtureRoot "fake-git.cmd"
$statePath = Join-Path $fixtureRoot "calls.log"
$gitRoot = Join-Path $fixtureRoot "Git Install With Spaces"
$connectPath = Join-Path $gitRoot "mingw64\bin\connect.exe"
$bashPath = Join-Path $gitRoot "bin\bash.exe"
$githubRemote = "git@github.com:owner/repository.git"

[System.IO.Directory]::CreateDirectory((Split-Path $stubPath -Parent)) | Out-Null
[System.IO.Directory]::CreateDirectory((Split-Path $connectPath -Parent)) | Out-Null
[System.IO.Directory]::CreateDirectory((Split-Path $bashPath -Parent)) | Out-Null
[System.IO.File]::WriteAllBytes($connectPath, [byte[]]@())
[System.IO.File]::WriteAllBytes($bashPath, [byte[]]@())

$stub = @'
param([Parameter(ValueFromRemainingArguments = $true)][string[]]$Arguments)
$route = if ($env:GIT_SSH_COMMAND -like "*ProxyCommand=none*") {
    "direct"
} elseif ($env:GIT_SSH_COMMAND -like "*connect.exe*") {
    "proxy"
} else {
    "unknown"
}
[System.IO.File]::AppendAllText(
    $env:ELON_TEST_GIT_STATE,
    "$route|$env:GIT_SSH_COMMAND|$env:SHELL`r`n",
    [System.Text.Encoding]::UTF8
)
switch ($env:ELON_TEST_GIT_SCENARIO) {
    "direct_success" {
        [Console]::Out.WriteLine("remote-result")
        exit 0
    }
    "network_then_success" {
        if ($route -eq "direct") {
            [Console]::Error.WriteLine("ssh: connect to host ssh.github.com port 443: Connection timed out. fatal: Could not read from remote repository.")
            exit 128
        }
        [Console]::Out.WriteLine("proxy-result")
        exit 0
    }
    "network_failure" {
        [Console]::Error.WriteLine("Connection reset by peer; unexpected EOF")
        exit 128
    }
    "authentication_failure" {
        [Console]::Error.WriteLine("git@github.com: Permission denied (publickey). Could not read from remote repository.")
        exit 128
    }
    "host_key_failure" {
        [Console]::Error.WriteLine("Host key verification failed. known_hosts entry differs.")
        exit 128
    }
    "business_failure" {
        [Console]::Error.WriteLine("! [rejected] HEAD -> main (non-fast-forward)")
        exit 1
    }
    default {
        [Console]::Error.WriteLine("unknown fixture scenario")
        exit 2
    }
}
'@
[System.IO.File]::WriteAllText($stubPath, $stub, [System.Text.UTF8Encoding]::new($true))
$stubCommand = "@echo off`r`npowershell.exe -NoProfile -ExecutionPolicy Bypass -File `"%~dp0fake-git.ps1`"`r`nexit /b %errorlevel%`r`n"
[System.IO.File]::WriteAllText($stubCommandPath, $stubCommand, [System.Text.Encoding]::ASCII)

function Reset-Case {
    param([string]$Scenario)
    if (Test-Path -LiteralPath $statePath) { Remove-Item -LiteralPath $statePath -Force }
    $env:ELON_TEST_GIT_STATE = $statePath
    $env:ELON_TEST_GIT_SCENARIO = $Scenario
}

function Invoke-TestGit {
    param(
        [string[]]$Arguments,
        [string]$RemoteUrl = $githubRemote,
        [string]$ProxyUrl = "http://127.0.0.1:17891",
        [string]$ConnectPath = $connectPath,
        [string]$ShellPath = $bashPath
    )
    return Invoke-ElonGitHubGitWithProxyFallback -RepoPath $repoRoot -GitArgs $Arguments `
        -RemoteName "origin" -RemoteUrl $RemoteUrl -ProxyUrl $ProxyUrl `
        -ConnectExecutablePath $ConnectPath -GitShellPath $ShellPath `
        -GitExecutable $stubCommandPath
}

$originalEnvironment = Get-ElonProcessEnvironmentSnapshot
try {
    foreach ($operation in @("fetch", "ls-remote", "push")) {
        Reset-Case "direct_success"
        $argsForOperation = switch ($operation) {
            "fetch" { @("fetch", "origin", "main") }
            "ls-remote" { @("ls-remote", "origin", "HEAD") }
            "push" { @("push", "origin", "HEAD:main") }
        }
        $result = Invoke-TestGit -Arguments $argsForOperation
        Assert-Equal 0 $result.ExitCode "$operation direct success: $($result.Text)"
        Assert-Equal "direct" $result.Route "$operation direct route"
        Assert-Equal 1 (Get-CallLines $statePath).Count "$operation must execute once"
    }

    Reset-Case "network_then_success"
    $result = Invoke-TestGit -Arguments @("fetch", "origin", "main")
    $calls = Get-CallLines $statePath
    Assert-Equal 0 $result.ExitCode "network fallback success"
    Assert-Equal "proxy" $result.Route "network fallback route"
    Assert-True $result.ProxyAttempted "network failure must attempt proxy once"
    Assert-Equal 2 $calls.Count "network fallback must have exactly two total attempts"
    Assert-True ($calls[0] -like "direct|*") "first attempt must be direct"
    Assert-True ($calls[1] -like "proxy|*") "second attempt must be proxy"
    Assert-True ($calls[1].Contains('"' + ($connectPath -replace '\\', '/') + '"')) "connect.exe path with spaces must be quoted"

    Reset-Case "network_failure"
    $missingConnect = Join-Path $fixtureRoot "missing\connect.exe"
    $result = Invoke-TestGit -Arguments @("fetch", "origin") -ConnectPath $missingConnect
    Assert-True ($result.ExitCode -ne 0) "unavailable proxy must preserve failure"
    Assert-True (-not $result.ProxyAttempted) "unavailable proxy must not start a second command"
    Assert-Equal 1 (Get-CallLines $statePath).Count "unavailable proxy must execute once"

    Reset-Case "authentication_failure"
    $result = Invoke-TestGit -Arguments @("ls-remote", "origin", "HEAD")
    Assert-Equal "authentication" $result.FailureClass "authentication classification"
    Assert-True (-not $result.ProxyAttempted) "authentication failure must not fall back"
    Assert-Equal 1 (Get-CallLines $statePath).Count "authentication failure must execute once"

    Reset-Case "host_key_failure"
    $result = Invoke-TestGit -Arguments @("fetch", "origin")
    Assert-Equal "policy_or_protocol" $result.FailureClass "host key classification"
    Assert-True (-not $result.ProxyAttempted) "host key failure must not fall back"

    Reset-Case "business_failure"
    $result = Invoke-TestGit -Arguments @("push", "origin", "HEAD:main")
    Assert-Equal "policy_or_protocol" $result.FailureClass "push rejection classification"
    Assert-True (-not $result.ProxyAttempted) "business failure must not fall back"

    Reset-Case "network_failure"
    $result = Invoke-TestGit -Arguments @("fetch", "origin") -RemoteUrl "git@example.com:owner/repository.git"
    Assert-True (-not $result.ProxyAttempted) "non-GitHub remote must stay direct"
    Assert-Equal 1 (Get-CallLines $statePath).Count "non-GitHub failure must execute once"

    Reset-Case "network_failure"
    $secretProxy = "http://proxy-user:proxy-secret@127.0.0.1:17891"
    $result = Invoke-TestGit -Arguments @("fetch", "origin") -ProxyUrl $secretProxy
    Assert-True (-not $result.ProxyAttempted) "credential-bearing proxy must be rejected"
    $diagnostic = "$($result.Text)`n$($result.Hint)"
    Assert-True (-not $diagnostic.Contains("proxy-user")) "proxy username must be redacted"
    Assert-True (-not $diagnostic.Contains("proxy-secret")) "proxy password must be redacted"

    foreach ($unsafeProxy in @("http://192.0.2.1:17891", "https://127.0.0.1:17891")) {
        Reset-Case "network_failure"
        $result = Invoke-TestGit -Arguments @("fetch", "origin") -ProxyUrl $unsafeProxy
        Assert-True (-not $result.ProxyAttempted) "non-loopback or non-HTTP proxy must be rejected"
        Assert-Equal 1 (Get-CallLines $statePath).Count "unsafe proxy must execute direct only"
    }

    $sentinels = @(
        [pscustomobject]@{ Name = "HTTP_PROXY"; Value = "http://sentinel.invalid:1111" },
        [pscustomobject]@{ Name = "HTTPS_PROXY"; Value = "http://sentinel.invalid:2222" },
        [pscustomobject]@{ Name = "ALL_PROXY"; Value = "http://sentinel.invalid:3333" },
        [pscustomobject]@{ Name = "http_proxy"; Value = "http://sentinel.invalid:1111" },
        [pscustomobject]@{ Name = "https_proxy"; Value = "http://sentinel.invalid:2222" },
        [pscustomobject]@{ Name = "all_proxy"; Value = "http://sentinel.invalid:3333" },
        [pscustomobject]@{ Name = "NO_PROXY"; Value = "sentinel-no-proxy" },
        [pscustomobject]@{ Name = "no_proxy"; Value = "sentinel-no-proxy" },
        [pscustomobject]@{ Name = "GIT_SSH_COMMAND"; Value = "sentinel-ssh" },
        [pscustomobject]@{ Name = "GIT_SSH_VARIANT"; Value = "sentinel-variant" },
        [pscustomobject]@{ Name = "SHELL"; Value = "sentinel-shell" }
    )
    foreach ($sentinel in $sentinels) {
        [System.Environment]::SetEnvironmentVariable($sentinel.Name, $sentinel.Value, "Process")
    }
    Reset-Case "network_failure"
    $result = Invoke-TestGit -Arguments @("fetch", "origin")
    Assert-True ($result.ExitCode -ne 0 -and $result.ProxyAttempted) "environment restoration fixture must cover failed fallback"
    foreach ($sentinel in $sentinels) {
        Assert-Equal $sentinel.Value ([System.Environment]::GetEnvironmentVariable($sentinel.Name, "Process")) "restore $($sentinel.Name)"
    }

    foreach ($relativePath in @(
        "scripts\ai-task-preflight.ps1",
        "scripts\finish-ai-task.ps1",
        "scripts\check-task-complete.ps1",
        "scripts\publish-node-agent.ps1"
    )) {
        $content = Get-Content -Raw -LiteralPath (Join-Path $repoRoot $relativePath)
        Assert-True ($content.Contains("Invoke-ElonGitHubGitWithProxyFallback")) "$relativePath must use the shared GitHub SSH executor"
    }

    Write-Host "GITHUB_SSH_NETWORK_TEST=passed"
    Write-Host "POWERSHELL_VERSION=$($PSVersionTable.PSVersion)"
} finally {
    Restore-ElonProcessEnvironment -Snapshot $originalEnvironment
    foreach ($name in @("ELON_TEST_GIT_STATE", "ELON_TEST_GIT_SCENARIO")) {
        [System.Environment]::SetEnvironmentVariable($name, $null, "Process")
    }
    $tempRoot = [System.IO.Path]::GetFullPath([System.IO.Path]::GetTempPath()).TrimEnd('\') + '\'
    $resolvedFixture = [System.IO.Path]::GetFullPath($fixtureRoot)
    if ($resolvedFixture.StartsWith($tempRoot, [System.StringComparison]::OrdinalIgnoreCase) -and
        (Split-Path -Leaf $resolvedFixture) -like "elon-github-ssh-test-*") {
        Remove-Item -LiteralPath $resolvedFixture -Recurse -Force -ErrorAction SilentlyContinue
    }
}
