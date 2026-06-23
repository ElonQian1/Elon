#requires -Version 7.0

param(
    [string]$Root = "",
    [string]$OutputPath = "",
    [switch]$SelfTest
)

$ErrorActionPreference = "Stop"

function Get-ProjectDirectPolicyRepoRoot {
    Split-Path -Parent $PSScriptRoot
}

function Resolve-ProjectDirectPolicyPath {
    param(
        [string]$Path,
        [string]$Base
    )

    if ([string]::IsNullOrWhiteSpace($Path)) {
        return ""
    }
    if ([System.IO.Path]::IsPathRooted($Path)) {
        return $Path
    }
    return (Join-Path $Base $Path)
}

function Read-ProjectDirectPolicyText {
    param([string]$Path)

    if (-not (Test-Path -LiteralPath $Path)) {
        return $null
    }
    Get-Content -LiteralPath $Path -Raw
}

function Add-ProjectDirectPolicyCheck {
    param(
        [System.Collections.ArrayList]$Checks,
        [string]$Name,
        [bool]$Passed,
        [string]$Details = ""
    )

    [void]$Checks.Add([ordered]@{
        name = $Name
        passed = [bool]$Passed
        details = $Details
    })
}

function Test-ProjectDirectPolicyContainsAll {
    param(
        [string]$Text,
        [string[]]$Required
    )

    if ($null -eq $Text) {
        return $false
    }
    foreach ($item in $Required) {
        if ($Text -notlike ("*" + $item + "*")) {
            return $false
        }
    }
    return $true
}

function Add-ProjectDirectPolicyFileCheck {
    param(
        [System.Collections.ArrayList]$Checks,
        [string]$Root,
        [string]$RelativePath,
        [string[]]$Required,
        [string]$Purpose
    )

    $path = Join-Path $Root $RelativePath
    $text = Read-ProjectDirectPolicyText -Path $path
    Add-ProjectDirectPolicyCheck $Checks "$RelativePath exists" ($null -ne $text) $Purpose
    foreach ($item in $Required) {
        Add-ProjectDirectPolicyCheck $Checks "$RelativePath contains $item" ($null -ne $text -and $text -like ("*" + $item + "*")) $Purpose
    }
}

function Save-ProjectDirectPolicyEnv {
    param([string[]]$Names)

    $snapshots = [System.Collections.ArrayList]::new()
    foreach ($name in $Names) {
        [void]$snapshots.Add([ordered]@{
            name = $name
            exists = [System.Environment]::GetEnvironmentVariable($name, "Process") -ne $null
            value = [System.Environment]::GetEnvironmentVariable($name, "Process")
        })
    }
    $snapshots
}

function Restore-ProjectDirectPolicyEnv {
    param([object[]]$Snapshots)

    foreach ($snapshot in $Snapshots) {
        if ([bool]$snapshot.exists) {
            [System.Environment]::SetEnvironmentVariable([string]$snapshot.name, [string]$snapshot.value, "Process")
        } else {
            [System.Environment]::SetEnvironmentVariable([string]$snapshot.name, $null, "Process")
        }
    }
}

function Test-ProjectDirectPolicyRuntime {
    param([string]$Root)

    $helper = Join-Path $Root "scripts\direct-network.ps1"
    if (-not (Test-Path -LiteralPath $helper)) {
        return [ordered]@{
            success = $false
            reason = "missing direct-network.ps1"
        }
    }

    $envNames = @(
        "HTTP_PROXY", "HTTPS_PROXY", "ALL_PROXY", "http_proxy", "https_proxy", "all_proxy",
        "NO_PROXY", "no_proxy", "GIT_SSH_COMMAND"
    )
    $snapshots = @(Save-ProjectDirectPolicyEnv -Names $envNames)
    try {
        . $helper
        foreach ($name in @("HTTP_PROXY", "HTTPS_PROXY", "ALL_PROXY", "http_proxy", "https_proxy", "all_proxy")) {
            [System.Environment]::SetEnvironmentVariable($name, "http://127.0.0.1:9", "Process")
        }
        [System.Environment]::SetEnvironmentVariable("NO_PROXY", "", "Process")
        [System.Environment]::SetEnvironmentVariable("no_proxy", "", "Process")
        [System.Environment]::SetEnvironmentVariable("GIT_SSH_COMMAND", "", "Process")

        Set-ElonProjectDirectNetwork
        $proxyValues = @("HTTP_PROXY", "HTTPS_PROXY", "ALL_PROXY", "http_proxy", "https_proxy", "all_proxy") |
            ForEach-Object { [System.Environment]::GetEnvironmentVariable($_, "Process") }
        $proxyCleared = (@($proxyValues | Where-Object { -not [string]::IsNullOrEmpty([string]$_) }).Count -eq 0)
        $noProxySet = (
            [System.Environment]::GetEnvironmentVariable("NO_PROXY", "Process") -eq "*" -and
            [System.Environment]::GetEnvironmentVariable("no_proxy", "Process") -eq "*"
        )

        $requestParams = @{ Uri = "http://127.0.0.1:9"; Method = "Get" }
        $requestParams = Add-ElonProjectDirectRequestParameters -Params $requestParams -CommandName "Invoke-RestMethod"
        $requestNoProxy = [bool]$requestParams.ContainsKey("NoProxy")

        Push-Location $Root
        try {
            Set-ElonProjectDirectGitSsh
        } finally {
            Pop-Location
        }
        $gitSsh = [System.Environment]::GetEnvironmentVariable("GIT_SSH_COMMAND", "Process")
        $gitSshDirect = (
            $gitSsh -like "*ProxyCommand=none*" -and
            $gitSsh -like "*ProxyJump=none*" -and
            $gitSsh -like "*ssh.github.com*" -and
            $gitSsh -like "*-p 443*"
        )

        return [ordered]@{
            success = [bool]($proxyCleared -and $noProxySet -and $requestNoProxy -and $gitSshDirect)
            proxy_cleared = [bool]$proxyCleared
            no_proxy_set = [bool]$noProxySet
            request_no_proxy = [bool]$requestNoProxy
            git_ssh_direct = [bool]$gitSshDirect
        }
    } catch {
        return [ordered]@{
            success = $false
            reason = [string]$_
        }
    } finally {
        Restore-ProjectDirectPolicyEnv -Snapshots $snapshots
    }
}

function New-ProjectDirectPolicyValidation {
    param(
        [string]$Root,
        [string]$OutputPath
    )

    if ([string]::IsNullOrWhiteSpace($Root)) {
        $Root = Get-ProjectDirectPolicyRepoRoot
    } else {
        $Root = Resolve-ProjectDirectPolicyPath -Path $Root -Base (Get-ProjectDirectPolicyRepoRoot)
    }
    $Root = [System.IO.Path]::GetFullPath($Root)
    if ([string]::IsNullOrWhiteSpace($OutputPath)) {
        $OutputPath = Join-Path $Root "target\fb2-ai-center\project-direct-network-policy-validation-current.json"
    } else {
        $OutputPath = Resolve-ProjectDirectPolicyPath -Path $OutputPath -Base $Root
    }

    $checks = [System.Collections.ArrayList]::new()

    Add-ProjectDirectPolicyFileCheck $checks $Root "scripts\direct-network.ps1" @(
        "Set-ElonProjectDirectNetwork",
        "HTTP_PROXY",
        "HTTPS_PROXY",
        "ALL_PROXY",
        "NO_PROXY",
        "no_proxy",
        "Set-ElonProjectDirectGitSsh",
        "ProxyCommand=none",
        "ProxyJump=none",
        "ssh.github.com",
        "-p 443",
        "Add-ElonProjectDirectRequestParameters",
        "NoProxy"
    ) "shared helper must clear proxies, force NO_PROXY, and expose HTTP/Git direct helpers"

    Add-ProjectDirectPolicyFileCheck $checks $Root "scripts\publish-server.ps1" @(
        "direct-network.ps1",
        "Set-ElonProjectDirectNetwork",
        "Set-ElonProjectDirectGitSsh",
        "ProxyCommand=none",
        "ProxyJump=none",
        "--noproxy"
    ) "server publish must bypass local proxy for release API, Git, SSH, SCP and live checks"

    Add-ProjectDirectPolicyFileCheck $checks $Root "scripts\publish-apk.ps1" @(
        "direct-network.ps1",
        "Set-ElonProjectDirectNetwork",
        "Set-ElonProjectDirectGitSsh",
        "ProxyCommand=none",
        "--noproxy"
    ) "APK publish must bypass local proxy for release API, Git, SSH, SCP and live checks"

    Add-ProjectDirectPolicyFileCheck $checks $Root "scripts\check-task-complete.ps1" @(
        "direct-network.ps1",
        "Set-ElonProjectDirectNetwork",
        "Set-ElonProjectDirectGitSsh",
        "Add-ElonProjectDirectRequestParameters",
        "http.proxy=",
        "https.proxy="
    ) "completion checks must bypass proxy for Git and HTTP"

    foreach ($script in @(
        "scripts\smoke-fb2-ai-center.ps1",
        "scripts\smoke-fb2-final-acceptance.ps1",
        "scripts\smoke-fb2-visible-chat.ps1"
    )) {
        Add-ProjectDirectPolicyFileCheck $checks $Root $script @(
            "direct-network.ps1",
            "Set-ElonProjectDirectNetwork",
            "Add-ElonProjectDirectRequestParameters"
        ) "fb2 AI Center smoke scripts must use the shared direct HTTP helper"
    }

    foreach ($script in @(
        "scripts\fb2-public-contract-status.ps1",
        "scripts\validate-fb2-main-server-deploy-status.ps1"
    )) {
        Add-ProjectDirectPolicyFileCheck $checks $Root $script @(
            "HTTP_PROXY",
            "HTTPS_PROXY",
            "ALL_PROXY",
            "NO_PROXY",
            "no_proxy",
            "Invoke-RestMethod",
            "-NoProxy"
        ) "public/live status scripts must use direct HTTP checks"
    }

    Add-ProjectDirectPolicyFileCheck $checks $Root "scripts\run-fb2-ai-center-token-bridge.ps1" @(
        "ProxyCommand=none",
        "ProxyJump=none",
        "NO_PROXY",
        "no_proxy",
        "HTTP_PROXY",
        "HTTPS_PROXY",
        "ALL_PROXY",
        "project_network_proxy_policy",
        "direct_no_proxy"
    ) "fb2 token bridge must use direct SSH and direct child process network policy"

    Add-ProjectDirectPolicyFileCheck $checks $Root "scripts\publish-server.sh" @(
        "ProxyCommand=none",
        "curl --noproxy"
    ) "Linux server publish path must bypass proxies"

    Add-ProjectDirectPolicyFileCheck $checks $Root "scripts\publish-apk.sh" @(
        "ProxyCommand=none",
        "--noproxy"
    ) "Linux APK publish path must bypass proxies"

    $runtime = Test-ProjectDirectPolicyRuntime -Root $Root
    Add-ProjectDirectPolicyCheck $checks "direct-network runtime clears proxy env" ([bool]$runtime.proxy_cleared) ($runtime | ConvertTo-Json -Depth 4 -Compress)
    Add-ProjectDirectPolicyCheck $checks "direct-network runtime sets NO_PROXY star" ([bool]$runtime.no_proxy_set) ($runtime | ConvertTo-Json -Depth 4 -Compress)
    Add-ProjectDirectPolicyCheck $checks "direct-network runtime adds NoProxy to PowerShell HTTP" ([bool]$runtime.request_no_proxy) ($runtime | ConvertTo-Json -Depth 4 -Compress)
    Add-ProjectDirectPolicyCheck $checks "direct-network runtime configures direct Git SSH" ([bool]$runtime.git_ssh_direct) ($runtime | ConvertTo-Json -Depth 4 -Compress)

    $failed = @($checks | Where-Object { -not [bool]$_.passed })
    $result = [ordered]@{
        schema = "elon.project.direct_network_policy_validation.v1"
        generated_at_utc = ([datetime]::UtcNow).ToString("o")
        root = $Root
        success = (@($failed).Count -eq 0)
        check_count = @($checks).Count
        failed_count = @($failed).Count
        failed = @($failed)
        checks = @($checks)
        runtime = $runtime
        policy = "direct_no_proxy"
        note = "All main-project/fb2 project access scripts in this gate must bypass local proxy env, use NO_PROXY/no_proxy=*, and use NoProxy or --noproxy for HTTP."
    }

    $parent = Split-Path -Parent $OutputPath
    if (-not [string]::IsNullOrWhiteSpace($parent)) {
        New-Item -ItemType Directory -Force -Path $parent | Out-Null
    }
    $json = $result | ConvertTo-Json -Depth 8
    Set-Content -LiteralPath $OutputPath -Value $json -Encoding UTF8
    $json

    if (-not [bool]$result.success) {
        exit 1
    }
}

function Invoke-ProjectDirectPolicySelfTest {
    $outputPath = Join-Path (Get-ProjectDirectPolicyRepoRoot) "target\fb2-ai-center\project-direct-network-policy-selftest.json"
    $json = New-ProjectDirectPolicyValidation -Root (Get-ProjectDirectPolicyRepoRoot) -OutputPath $outputPath
    $result = $json | ConvertFrom-Json
    if (-not [bool]$result.success) {
        exit 1
    }
    Write-Output "== SelfTest Summary =="
    Write-Output "failed=0"
}

if ($SelfTest) {
    Invoke-ProjectDirectPolicySelfTest
    exit 0
}

New-ProjectDirectPolicyValidation -Root $Root -OutputPath $OutputPath
