<#
.SYNOPSIS
    Check toolchain prerequisites before running native builds.

.DESCRIPTION
    This guard reports missing machine tools before Cargo or Gradle starts a long
    build. It does not install software or mutate the machine environment.
#>
param(
    [ValidateSet("Server", "Android", "All")]
    [string]$Scope = "All"
)

$ErrorActionPreference = "Stop"
. (Join-Path $PSScriptRoot "ensure-msvc-environment.ps1")

function Stop-BuildPrerequisiteCheck {
    param([string]$Message)
    Write-Error $Message
    exit 1
}

function Get-RepoRoot {
    $root = (& git rev-parse --show-toplevel).Trim()
    if ($LASTEXITCODE -ne 0 -or [string]::IsNullOrWhiteSpace($root)) {
        Stop-BuildPrerequisiteCheck "Current directory is not inside a git repository."
    }
    return $root
}

function Test-ServerPrerequisites {
    $rust = Get-Command rustc.exe -CommandType Application -ErrorAction SilentlyContinue | Select-Object -First 1
    if ($null -eq $rust) {
        Stop-BuildPrerequisiteCheck "Rust compiler is not available in PATH. Install the pinned Rust toolchain first."
    }

    try {
        $linkPath = Ensure-ElonMsvcEnvironment
    } catch {
        Stop-BuildPrerequisiteCheck $_.Exception.Message
    }
    $link = Get-Command link.exe -CommandType Application -ErrorAction SilentlyContinue | Select-Object -First 1
    Write-Host "SERVER_BUILD_PREREQUISITES=passed rust=$($rust.Source) link=$($link.Source)"
}

function Test-AndroidPrerequisites {
    $java = Get-Command java.exe -CommandType Application -ErrorAction SilentlyContinue | Select-Object -First 1
    if ($null -eq $java) {
        Stop-BuildPrerequisiteCheck "Java is not available in PATH. Android builds require JDK 17 or newer."
    }

    $previousErrorAction = $ErrorActionPreference
    try {
        $ErrorActionPreference = "Continue"
        $javaVersion = (& $java.Source -version 2>&1 | Out-String)
    } finally {
        $ErrorActionPreference = $previousErrorAction
    }
    if ($javaVersion -notmatch 'version\s+"(\d+)') {
        Stop-BuildPrerequisiteCheck "Unable to determine the installed Java major version."
    }
    $javaMajor = [int]$Matches[1]
    if ($javaMajor -lt 17) {
        Stop-BuildPrerequisiteCheck "Android builds require JDK 17 or newer; detected Java $javaMajor."
    }

    $sdkRoot = $env:ANDROID_HOME
    if ([string]::IsNullOrWhiteSpace($sdkRoot)) {
        $sdkRoot = $env:ANDROID_SDK_ROOT
    }
    if ([string]::IsNullOrWhiteSpace($sdkRoot) -or -not (Test-Path -LiteralPath $sdkRoot -PathType Container)) {
        Stop-BuildPrerequisiteCheck "ANDROID_HOME or ANDROID_SDK_ROOT must point to an Android SDK."
    }

    $platform = Join-Path $sdkRoot "platforms\android-34"
    $buildTools = Join-Path $sdkRoot "build-tools\34.0.0"
    if (-not (Test-Path -LiteralPath $platform -PathType Container)) {
        Stop-BuildPrerequisiteCheck "Android SDK platform android-34 is missing under $sdkRoot."
    }
    if (-not (Test-Path -LiteralPath $buildTools -PathType Container)) {
        Stop-BuildPrerequisiteCheck "Android SDK build-tools 34.0.0 is missing under $sdkRoot."
    }
    Write-Host "ANDROID_BUILD_PREREQUISITES=passed java=$javaMajor sdk=$sdkRoot"
}

$repoRoot = Get-RepoRoot
Set-Location $repoRoot

if ($Scope -eq "Server" -or $Scope -eq "All") {
    Test-ServerPrerequisites
}
if ($Scope -eq "Android" -or $Scope -eq "All") {
    Test-AndroidPrerequisites
}

Write-Host "BUILD_PREREQUISITES=passed scope=$Scope"
