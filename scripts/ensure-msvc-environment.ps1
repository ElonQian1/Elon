<#
.SYNOPSIS
    Load the installed Visual Studio C++ build environment into this process.

.DESCRIPTION
    Windows PowerShell sessions do not always inherit the environment created by
    Visual Studio Installer. This helper keeps local scripts and release scripts
    deterministic by loading VsDevCmd when link.exe is installed but absent from
    PATH. It only changes the current PowerShell process; it does not install or
    modify machine-wide software.
#>

function Find-ElonVsDevCmd {
    $vswhereCandidates = @(
        (Join-Path ${env:ProgramFiles(x86)} "Microsoft Visual Studio\Installer\vswhere.exe"),
        (Join-Path $env:ProgramFiles "Microsoft Visual Studio\Installer\vswhere.exe")
    ) | Where-Object { -not [string]::IsNullOrWhiteSpace($_) -and (Test-Path -LiteralPath $_ -PathType Leaf) }

    foreach ($vswhere in $vswhereCandidates) {
        $installationPath = (& $vswhere -latest -products * -requires Microsoft.VisualStudio.Component.VC.Tools.x86.x64 -property installationPath 2>$null | Select-Object -First 1)
        if (-not [string]::IsNullOrWhiteSpace($installationPath)) {
            $candidate = Join-Path $installationPath "Common7\Tools\VsDevCmd.bat"
            if (Test-Path -LiteralPath $candidate -PathType Leaf) { return $candidate }
        }
    }

    $fallbackRoots = @(
        (Join-Path ${env:ProgramFiles(x86)} "Microsoft Visual Studio\2022\BuildTools"),
        (Join-Path $env:ProgramFiles "Microsoft Visual Studio\2022\BuildTools")
    ) | Where-Object { -not [string]::IsNullOrWhiteSpace($_) }
    foreach ($root in $fallbackRoots) {
        $candidate = Join-Path $root "Common7\Tools\VsDevCmd.bat"
        if (Test-Path -LiteralPath $candidate -PathType Leaf) { return $candidate }
    }

    return $null
}

function Ensure-ElonMsvcEnvironment {
    [CmdletBinding()]
    param()

    $existingLink = Get-Command link.exe -CommandType Application -ErrorAction SilentlyContinue | Select-Object -First 1
    if ($null -ne $existingLink) {
        return $existingLink.Source
    }

    $vsDevCmd = Find-ElonVsDevCmd
    if ([string]::IsNullOrWhiteSpace($vsDevCmd)) {
        throw "Visual Studio C++ Build Tools are not installed. Install the Desktop development with C++ workload."
    }

    $command = 'call "{0}" -arch=x64 -host_arch=x64 && set' -f $vsDevCmd
    $environmentLines = @(cmd.exe /d /s /c $command 2>$null)
    if ($LASTEXITCODE -ne 0) {
        throw "Visual Studio developer environment could not be loaded from '$vsDevCmd'."
    }

    foreach ($line in $environmentLines) {
        if ($line -notmatch '^([^=]+)=(.*)$') { continue }
        [Environment]::SetEnvironmentVariable($Matches[1], $Matches[2], "Process")
    }

    $loadedLink = Get-Command link.exe -CommandType Application -ErrorAction SilentlyContinue | Select-Object -First 1
    if ($null -eq $loadedLink) {
        throw "Visual Studio environment loaded, but link.exe is still unavailable."
    }
    return $loadedLink.Source
}
