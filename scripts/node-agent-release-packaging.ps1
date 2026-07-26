Set-StrictMode -Version Latest

function New-NodeAgentWindowsClientPackage {
    param(
        [Parameter(Mandatory = $true)][string]$RepoRoot,
        [Parameter(Mandatory = $true)][string]$TargetDir,
        [Parameter(Mandatory = $true)][string]$PackageVersion,
        [Parameter(Mandatory = $true)][string]$GitSha,
        [Parameter(Mandatory = $true)][string]$ReleaseChangelog,
        [Parameter(Mandatory = $true)][string]$WindowsDownloadUrl,
        [Parameter(Mandatory = $true)][string]$WindowsClientDownloadUrl,
        [Parameter(Mandatory = $true)][string]$RipgrepDownloadUrl,
        [Parameter(Mandatory = $true)][string]$WinBin,
        [Parameter(Mandatory = $true)][string]$WinSha256,
        [Parameter(Mandatory = $true)][string]$DesktopShellBin,
        [Parameter(Mandatory = $true)][string]$BrandIcon,
        [Parameter(Mandatory = $true)][string]$PcDistDir,
        [Parameter(Mandatory = $true)][string]$LauncherDir,
        [Parameter(Mandatory = $true)][string]$WindowsClientPackageName,
        [Parameter(Mandatory = $true)][string]$RipgrepPackageName,
        [Parameter(Mandatory = $true)][string]$ClientFileName,
        [Parameter(Mandatory = $true)][string]$UninstallFileName,
        [switch]$IncludeLinux
    )

    $packageRoot = Join-Path ([System.IO.Path]::GetTempPath()) (
        'elon-node-agent-windows-' + [Guid]::NewGuid().ToString('N')
    )
    $packageInternal = Join-Path $packageRoot '_internal'
    $windowsClientPackage = Join-Path $TargetDir "release\$WindowsClientPackageName"
    $ripgrepPackage = Join-Path $TargetDir "release\$RipgrepPackageName"
    $ripgrepZipSha256 = ''
    $ripgrepZipFileSize = 0

    Write-Host '  Packaging optional portable ripgrep...' -ForegroundColor DarkGray
    $ripgrepExe = Resolve-RipgrepExe
    if ($ripgrepExe) {
        $ripgrepRoot = Join-Path ([System.IO.Path]::GetTempPath()) (
            'elon-ripgrep-windows-' + [Guid]::NewGuid().ToString('N')
        )
        $ripgrepBinDir = Join-Path $ripgrepRoot 'bin'
        New-Item -ItemType Directory -Force -Path $ripgrepBinDir | Out-Null
        try {
            Copy-Item -LiteralPath $ripgrepExe -Destination (Join-Path $ripgrepBinDir 'rg.exe') -Force
            Compress-ArchiveWithRetry -Path (Join-Path $ripgrepRoot '*') -DestinationPath $ripgrepPackage
            $ripgrepZipSha256 = Get-NodeAgentFileSha256 -Path $ripgrepPackage
            $ripgrepZipFileSize = (Get-Item -LiteralPath $ripgrepPackage).Length
            Write-Host "  ripgrep package sha256 = $ripgrepZipSha256" -ForegroundColor DarkGray
        } finally {
            Remove-Item -LiteralPath $ripgrepRoot -Recurse -Force -ErrorAction SilentlyContinue
        }
    } else {
        Write-Host '  rg.exe not found; portable ripgrep package skipped and repair will use winget.' `
            -ForegroundColor DarkYellow
    }

    New-Item -ItemType Directory -Force -Path $packageRoot, $packageInternal | Out-Null
    try {
        $packageClient = Join-Path $packageRoot $ClientFileName
        $packageUninstall = Join-Path $packageRoot $UninstallFileName
        Copy-Item -LiteralPath $WinBin -Destination $packageClient -Force
        Copy-Item -LiteralPath $WinBin -Destination $packageUninstall -Force
        Assert-WindowsExecutableBrandIcon -ExecutablePath $packageClient `
            -ExpectedIconPath $BrandIcon | Out-Null
        Assert-WindowsExecutableBrandIcon -ExecutablePath $packageUninstall `
            -ExpectedIconPath $BrandIcon | Out-Null
        Copy-Item -LiteralPath $DesktopShellBin `
            -Destination (Join-Path $packageInternal 'elon-desktop.exe') -Force
        foreach ($name in @('node-agent.env.example', 'README.txt')) {
            Copy-Item -LiteralPath (Join-Path $LauncherDir $name) `
                -Destination (Join-Path $packageInternal $name) -Force
        }
        foreach ($name in @('desktop-review-credential.ps1', 'new-desktop-review-ticket.ps1')) {
            Copy-Item -LiteralPath (Join-Path $RepoRoot "scripts\$name") `
                -Destination (Join-Path $packageInternal $name) -Force
        }
        $packagePcDist = Join-Path $packageInternal 'pc-next-dist'
        New-Item -ItemType Directory -Force -Path $packagePcDist | Out-Null
        Copy-Item -Path (Join-Path $PcDistDir '*') -Destination $packagePcDist -Recurse -Force
        $packageVersionInfo = [ordered]@{
            version = $PackageVersion
            gitSha = $GitSha
            changelog = $ReleaseChangelog
            updated_at = (Get-Date).ToString('o')
            downloadUrl = $WindowsDownloadUrl
            windowsClientDownloadUrl = $WindowsClientDownloadUrl
            sha256 = $WinSha256
            fileSha256 = $WinSha256
            linuxPublished = $false
            linuxPublishRequested = [bool]$IncludeLinux
            ripgrepZipUrl = $RipgrepDownloadUrl
            ripgrepZipSha256 = $ripgrepZipSha256
            ripgrepZipFileSize = [int64]$ripgrepZipFileSize
        }
        Write-Utf8NoBom -Path (Join-Path $packageInternal 'node-agent-version.json') `
            -Content ($packageVersionInfo | ConvertTo-Json -Depth 4)
        Compress-ArchiveWithRetry -Path (Join-Path $packageRoot '*') `
            -DestinationPath $windowsClientPackage
    } finally {
        Remove-Item -LiteralPath $packageRoot -Recurse -Force -ErrorAction SilentlyContinue
    }
    if (-not (Test-Path -LiteralPath $windowsClientPackage -PathType Leaf)) {
        throw "Windows client package does not exist: $windowsClientPackage"
    }

    [pscustomobject]@{
        WindowsClientPackage = $windowsClientPackage
        WindowsClientSha256 = Get-NodeAgentFileSha256 -Path $windowsClientPackage
        RipgrepPackage = $ripgrepPackage
        RipgrepZipSha256 = $ripgrepZipSha256
        RipgrepZipFileSize = [int64]$ripgrepZipFileSize
    }
}
