#requires -Version 7.0

param(
    [string]$MainBase = "",
    [string]$OutputPath = "",
    [string[]]$RuntimePaths = @("server"),
    [string[]]$RuntimeExcludePaths = @("server/tests", "server/src/*_tests.rs", "server/src/**/*_tests.rs"),
    [int]$RequestTimeoutSec = 30,
    [switch]$SelfTest
)

$ErrorActionPreference = "Stop"

if (-not $MainBase) {
    $MainBase = $env:ELON_MAIN_BASE
}
if (-not $MainBase) {
    $MainBase = "http://43.139.149.158:8080"
}
$MainBase = $MainBase.TrimEnd("/")

function Set-Fb2DeployDirectNetwork {
    foreach ($name in @("HTTP_PROXY", "HTTPS_PROXY", "ALL_PROXY", "http_proxy", "https_proxy", "all_proxy")) {
        [System.Environment]::SetEnvironmentVariable($name, $null, "Process")
    }
    [System.Environment]::SetEnvironmentVariable("NO_PROXY", "*", "Process")
    [System.Environment]::SetEnvironmentVariable("no_proxy", "*", "Process")
}

function Invoke-Fb2DeployDirectRest {
    param(
        [string]$Uri,
        [int]$TimeoutSec
    )

    Invoke-RestMethod -Method Get -Uri $Uri -TimeoutSec $TimeoutSec -NoProxy
}

function Get-Fb2DeployRepoRoot {
    Split-Path -Parent $PSScriptRoot
}

function Resolve-Fb2DeployPath {
    param(
        [string]$Path,
        [string]$Root
    )

    if ([string]::IsNullOrWhiteSpace($Path)) {
        return ""
    }
    if ([System.IO.Path]::IsPathRooted($Path)) {
        return $Path
    }
    return (Join-Path $Root $Path)
}

function Add-Fb2DeployCheck {
    param(
        [System.Collections.ArrayList]$Checks,
        [string]$Name,
        [bool]$Passed,
        [string]$Details = ""
    )

    [void]$Checks.Add([ordered]@{
        name = $Name
        passed = $Passed
        details = $Details
    })
}

function Test-Fb2DeployShaContainsRuntime {
    param(
        [string]$LatestRuntimeSha,
        [string]$DeployedSha,
        [string]$Root
    )

    if ([string]::IsNullOrWhiteSpace($LatestRuntimeSha) -or [string]::IsNullOrWhiteSpace($DeployedSha)) {
        return $false
    }

    & git -C $Root merge-base --is-ancestor $LatestRuntimeSha $DeployedSha 2>$null
    return ($LASTEXITCODE -eq 0)
}

function Convert-Fb2DeployGitPath {
    param([string]$Path)

    ([string]$Path).Trim().Trim("/\").Replace("\", "/")
}

function Test-Fb2DeployPathExcluded {
    param(
        [string]$Path,
        [string[]]$ExcludePaths
    )

    $normalizedPath = Convert-Fb2DeployGitPath $Path
    foreach ($rawExclude in @($ExcludePaths)) {
        if ([string]::IsNullOrWhiteSpace([string]$rawExclude)) {
            continue
        }
        $exclude = Convert-Fb2DeployGitPath $rawExclude
        if ($exclude -match '[\*\?\[]') {
            $pattern = [System.Management.Automation.WildcardPattern]::new(
                $exclude,
                [System.Management.Automation.WildcardOptions]::IgnoreCase
            )
            if ($pattern.IsMatch($normalizedPath)) {
                return $true
            }
            continue
        }
        if ($normalizedPath -eq $exclude -or $normalizedPath.StartsWith("$exclude/")) {
            return $true
        }
    }
    $false
}

function Get-Fb2DeployChangedRuntimeFiles {
    param(
        [string]$Root,
        [string]$Sha,
        [string[]]$Paths,
        [string[]]$ExcludePaths
    )

    $showArgs = @("-C", $Root, "show", "--name-only", "--format=", $Sha, "--") + $Paths
    $changed = @(& git @showArgs 2>$null |
        ForEach-Object { Convert-Fb2DeployGitPath $_ } |
        Where-Object { -not [string]::IsNullOrWhiteSpace([string]$_) })
    @($changed | Where-Object { -not (Test-Fb2DeployPathExcluded -Path $_ -ExcludePaths $ExcludePaths) })
}

function Get-Fb2DeployLatestRuntimeCommit {
    param(
        [string]$Root,
        [string[]]$Paths,
        [string[]]$ExcludePaths = @()
    )

    $usable = @($Paths |
        Where-Object { -not [string]::IsNullOrWhiteSpace([string]$_) } |
        ForEach-Object { Convert-Fb2DeployGitPath $_ } |
        Where-Object { -not [string]::IsNullOrWhiteSpace([string]$_) })
    if (@($usable).Count -eq 0) {
        throw "RuntimePaths cannot be empty"
    }
    $usableExcludes = @($ExcludePaths |
        Where-Object { -not [string]::IsNullOrWhiteSpace([string]$_) } |
        ForEach-Object { Convert-Fb2DeployGitPath $_ } |
        Where-Object { -not [string]::IsNullOrWhiteSpace([string]$_) } |
        Select-Object -Unique)
    $args = @("-C", $Root, "log", "-200", "--format=%H", "--") + $usable
    $candidates = @(& git @args 2>$null | Where-Object { -not [string]::IsNullOrWhiteSpace([string]$_) })
    foreach ($candidate in $candidates) {
        $runtimeFiles = @(Get-Fb2DeployChangedRuntimeFiles -Root $Root -Sha $candidate -Paths $usable -ExcludePaths $usableExcludes)
        if (@($runtimeFiles).Count -gt 0) {
            return [string]$candidate
        }
    }
    throw "No runtime commit found for paths: $($usable -join ', ') after excludes: $($usableExcludes -join ', ')"
}

function New-Fb2MainServerDeployStatus {
    param(
        [string]$Base,
        [object]$Health,
        [object]$Version,
        [string]$LatestRuntimeSha,
        [bool]$DeployedContainsLatestRuntime,
        [string[]]$RuntimePaths,
        [string[]]$RuntimeExcludePaths
    )

    $healthText = if ($Health -is [string]) { [string]$Health } else { [string]$Health.status }
    $healthOk = ($healthText -eq "OK") -or ($healthText -eq "ok")
    $deployedSha = [string]$Version.gitSha
    $checks = [System.Collections.ArrayList]::new()
    Add-Fb2DeployCheck $checks "main_health_ok" $healthOk $healthText
    Add-Fb2DeployCheck $checks "server_version_git_sha_present" (-not [string]::IsNullOrWhiteSpace($deployedSha)) $deployedSha
    Add-Fb2DeployCheck $checks "latest_runtime_sha_present" (-not [string]::IsNullOrWhiteSpace($LatestRuntimeSha)) $LatestRuntimeSha
    # 允许 HEAD 上只有文档/验收记录提交：只要求线上 SHA 包含最新 server 运行时代码提交。
    Add-Fb2DeployCheck $checks "deployed_contains_latest_runtime_sha" $DeployedContainsLatestRuntime "latest_runtime=$LatestRuntimeSha deployed=$deployedSha"

    $failed = @($checks | Where-Object { -not [bool]$_.passed })
    [ordered]@{
        schema = "fb2.main_project.server_deploy_status.v1"
        generated_at_utc = ([datetime]::UtcNow).ToString("o")
        main_base = $Base
        runtime_paths = @($RuntimePaths)
        runtime_exclude_paths = @($RuntimeExcludePaths)
        server = [ordered]@{
            health = $healthText
            versionName = [string]$Version.versionName
            gitSha = $deployedSha
        }
        latest_runtime_sha = $LatestRuntimeSha
        deployed_contains_latest_runtime_sha = $DeployedContainsLatestRuntime
        success = (@($failed).Count -eq 0)
        failed_count = @($failed).Count
        failed = @($failed)
        checks = @($checks)
        note = "This verifies the deployed main-project server contains the latest runtime commit for RuntimePaths after excluding test-only paths. Later docs/test-only commits do not require a server redeploy."
    }
}

function Invoke-Fb2DeployGit {
    param(
        [string]$Root,
        [string[]]$GitArgs
    )

    & git -C $Root @GitArgs | Out-Null
    if ($LASTEXITCODE -ne 0) {
        throw "git $($GitArgs -join ' ') failed"
    }
}

function Test-Fb2DeployRuntimeExcludeSelfTest {
    $tmpRoot = Join-Path ([System.IO.Path]::GetTempPath()) ("fb2-deploy-runtime-" + [guid]::NewGuid().ToString("N"))
    New-Item -ItemType Directory -Force -Path $tmpRoot | Out-Null
    try {
        Invoke-Fb2DeployGit -Root $tmpRoot -GitArgs @("init")
        Invoke-Fb2DeployGit -Root $tmpRoot -GitArgs @("config", "user.email", "codex@example.invalid")
        Invoke-Fb2DeployGit -Root $tmpRoot -GitArgs @("config", "user.name", "Codex")
        New-Item -ItemType Directory -Force -Path (Join-Path $tmpRoot "server/src") | Out-Null
        Set-Content -LiteralPath (Join-Path $tmpRoot "server/src/main.rs") -Value "fn main() {}" -Encoding UTF8
        Invoke-Fb2DeployGit -Root $tmpRoot -GitArgs @("add", "server/src/main.rs")
        Invoke-Fb2DeployGit -Root $tmpRoot -GitArgs @("commit", "-m", "runtime")
        $runtimeSha = (& git -C $tmpRoot rev-parse HEAD).Trim()
        New-Item -ItemType Directory -Force -Path (Join-Path $tmpRoot "server/tests") | Out-Null
        Set-Content -LiteralPath (Join-Path $tmpRoot "server/tests/pressure.rs") -Value "#[test] fn pressure() {}" -Encoding UTF8
        Invoke-Fb2DeployGit -Root $tmpRoot -GitArgs @("add", "server/tests/pressure.rs")
        Invoke-Fb2DeployGit -Root $tmpRoot -GitArgs @("commit", "-m", "test dir only")
        Set-Content -LiteralPath (Join-Path $tmpRoot "server/src/pressure_tests.rs") -Value "#[test] fn pressure_src() {}" -Encoding UTF8
        Invoke-Fb2DeployGit -Root $tmpRoot -GitArgs @("add", "server/src/pressure_tests.rs")
        Invoke-Fb2DeployGit -Root $tmpRoot -GitArgs @("commit", "-m", "test module only")
        $latest = Get-Fb2DeployLatestRuntimeCommit -Root $tmpRoot -Paths @("server") -ExcludePaths @("server/tests", "server/src/*_tests.rs", "server/src/**/*_tests.rs")
        return ($latest -eq $runtimeSha)
    } finally {
        if (Test-Path -LiteralPath $tmpRoot) {
            Remove-Item -LiteralPath $tmpRoot -Recurse -Force
        }
    }
}

function Invoke-Fb2DeploySelfTest {
    $failed = 0
    $version = [pscustomobject]@{
        versionName = "selftest"
        gitSha = "deploy-sha"
    }
    $ok = New-Fb2MainServerDeployStatus `
        -Base "http://example.invalid" `
        -Health "OK" `
        -Version $version `
        -LatestRuntimeSha "runtime-sha" `
        -DeployedContainsLatestRuntime $true `
        -RuntimePaths @("server") `
        -RuntimeExcludePaths @("server/tests")
    if (-not [bool]$ok.success) {
        $failed++
    }
    $stale = New-Fb2MainServerDeployStatus `
        -Base "http://example.invalid" `
        -Health "OK" `
        -Version $version `
        -LatestRuntimeSha "runtime-sha" `
        -DeployedContainsLatestRuntime $false `
        -RuntimePaths @("server") `
        -RuntimeExcludePaths @("server/tests")
    if ([bool]$stale.success) {
        $failed++
    }
    $badHealth = New-Fb2MainServerDeployStatus `
        -Base "http://example.invalid" `
        -Health "DOWN" `
        -Version $version `
        -LatestRuntimeSha "runtime-sha" `
        -DeployedContainsLatestRuntime $true `
        -RuntimePaths @("server") `
        -RuntimeExcludePaths @("server/tests")
    if ([bool]$badHealth.success) {
        $failed++
    }
    if (-not (Test-Fb2DeployRuntimeExcludeSelfTest)) {
        $failed++
    }

    "== SelfTest Summary =="
    "failed=$failed"
    if ($failed -gt 0) {
        exit 1
    }
}

if ($SelfTest) {
    Invoke-Fb2DeploySelfTest
    exit 0
}

$root = Get-Fb2DeployRepoRoot
$latestRuntimeSha = Get-Fb2DeployLatestRuntimeCommit -Root $root -Paths $RuntimePaths -ExcludePaths $RuntimeExcludePaths
Set-Fb2DeployDirectNetwork
$health = Invoke-Fb2DeployDirectRest -Uri "$MainBase/health" -TimeoutSec $RequestTimeoutSec
$version = Invoke-Fb2DeployDirectRest -Uri "$MainBase/api/server/version" -TimeoutSec $RequestTimeoutSec
$deployedContains = Test-Fb2DeployShaContainsRuntime -LatestRuntimeSha $latestRuntimeSha -DeployedSha ([string]$version.gitSha) -Root $root
$status = New-Fb2MainServerDeployStatus `
    -Base $MainBase `
    -Health $health `
    -Version $version `
    -LatestRuntimeSha $latestRuntimeSha `
    -DeployedContainsLatestRuntime $deployedContains `
    -RuntimePaths $RuntimePaths `
    -RuntimeExcludePaths $RuntimeExcludePaths

if ($OutputPath) {
    $resolved = Resolve-Fb2DeployPath -Path $OutputPath -Root $root
    $parent = Split-Path -Parent $resolved
    if ($parent) {
        New-Item -ItemType Directory -Force -Path $parent | Out-Null
    }
    $status | ConvertTo-Json -Depth 8 | Set-Content -LiteralPath $resolved -Encoding UTF8
}

$status | ConvertTo-Json -Depth 8
if (-not [bool]$status.success) {
    exit 1
}
