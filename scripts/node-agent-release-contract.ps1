function Get-NodeAgentReleaseIdentity {
    param(
        [Parameter(Mandatory = $true)][string]$Version,
        [string]$GitSha = ""
    )

    $versionValue = $Version.Trim()
    $gitShaValue = $GitSha.Trim()
    if ([string]::IsNullOrWhiteSpace($gitShaValue)) {
        return $versionValue
    }
    return "${versionValue}+${gitShaValue}"
}

function Test-NodeAgentPublishHandshakeReady {
    param(
        [Parameter(Mandatory = $true)]$Node,
        [Parameter(Mandatory = $true)][string]$TargetReleaseIdentity
    )

    if (-not $Node.public_dev_handshake_ready) {
        return $false
    }
    $reportedIdentity = [string]$Node.agent_version
    return [string]::Equals(
        $reportedIdentity.Trim(),
        $TargetReleaseIdentity.Trim(),
        [System.StringComparison]::OrdinalIgnoreCase
    )
}

function Get-WindowsIconBitmapSha256 {
    param(
        [Parameter(Mandatory = $true)]$Icon
    )

    $bitmap = $Icon.ToBitmap()
    $stream = New-Object System.IO.MemoryStream
    $sha256 = [System.Security.Cryptography.SHA256]::Create()
    try {
        $bitmap.Save($stream, [System.Drawing.Imaging.ImageFormat]::Png)
        return ([System.BitConverter]::ToString($sha256.ComputeHash($stream.ToArray()))).Replace('-', '').ToLowerInvariant()
    } finally {
        $sha256.Dispose()
        $stream.Dispose()
        $bitmap.Dispose()
    }
}

function Get-WindowsFileIcon {
    param(
        [Parameter(Mandatory = $true)][string]$FilePath
    )

    Add-Type -AssemblyName System.Drawing
    if (-not ("Elon.NodeAgentReleaseContract.NativeIconMethods" -as [type])) {
        Add-Type -TypeDefinition @"
using System;
using System.Runtime.InteropServices;

namespace Elon.NodeAgentReleaseContract
{
    public static class NativeIconMethods
    {
        [DllImport("user32.dll", CharSet = CharSet.Unicode)]
        public static extern uint PrivateExtractIcons(
            string fileName,
            int iconIndex,
            int iconWidth,
            int iconHeight,
            IntPtr[] iconHandles,
            uint[] iconIds,
            uint iconCount,
            uint flags);

        [DllImport("user32.dll")]
        [return: MarshalAs(UnmanagedType.Bool)]
        public static extern bool DestroyIcon(IntPtr iconHandle);
    }
}
"@
    }

    $handles = New-Object System.IntPtr[] 1
    $iconIds = New-Object System.UInt32[] 1
    $count = [Elon.NodeAgentReleaseContract.NativeIconMethods]::PrivateExtractIcons(
        $FilePath,
        0,
        32,
        32,
        $handles,
        $iconIds,
        1,
        0
    )
    if ($count -ne 1 -or $handles[0] -eq [System.IntPtr]::Zero) {
        throw "Windows 文件没有可提取的 32px 图标：$FilePath"
    }

    try {
        $icon = [System.Drawing.Icon]::FromHandle($handles[0])
        return $icon.Clone()
    } finally {
        [void][Elon.NodeAgentReleaseContract.NativeIconMethods]::DestroyIcon($handles[0])
    }
}

function Get-WindowsExecutableAssociatedIconSha256 {
    param(
        [Parameter(Mandatory = $true)][string]$ExecutablePath
    )

    $fullPath = [System.IO.Path]::GetFullPath($ExecutablePath)
    if (-not (Test-Path -LiteralPath $fullPath -PathType Leaf)) {
        throw "Windows 可执行文件不存在：$fullPath"
    }
    $icon = Get-WindowsFileIcon -FilePath $fullPath
    try {
        return Get-WindowsIconBitmapSha256 -Icon $icon
    } finally {
        $icon.Dispose()
    }
}

function Get-WindowsBrandIconAssetSha256 {
    param(
        [Parameter(Mandatory = $true)][string]$IconPath
    )

    $fullPath = [System.IO.Path]::GetFullPath($IconPath)
    if (-not (Test-Path -LiteralPath $fullPath -PathType Leaf)) {
        throw "Windows 品牌 ICO 不存在：$fullPath"
    }
    $icon = Get-WindowsFileIcon -FilePath $fullPath
    try {
        return Get-WindowsIconBitmapSha256 -Icon $icon
    } finally {
        $icon.Dispose()
    }
}

function Assert-WindowsExecutableBrandIcon {
    param(
        [Parameter(Mandatory = $true)][string]$ExecutablePath,
        [Parameter(Mandatory = $true)][string]$ExpectedIconPath
    )

    $actual = Get-WindowsExecutableAssociatedIconSha256 -ExecutablePath $ExecutablePath
    $expected = Get-WindowsBrandIconAssetSha256 -IconPath $ExpectedIconPath
    if (-not [string]::Equals($actual, $expected, [System.StringComparison]::OrdinalIgnoreCase)) {
        throw "Windows AssociatedIcon 与品牌 ICO 不一致：exe=$ExecutablePath actual=$actual expected=$expected"
    }
    Write-Host "WINDOWS_BRAND_ICON_VERIFIED=$([System.IO.Path]::GetFileName($ExecutablePath));sha256=$actual" -ForegroundColor Green
    return [pscustomobject]@{
        executable = [System.IO.Path]::GetFullPath($ExecutablePath)
        associatedIconSha256 = $actual
        expectedIconSha256 = $expected
    }
}
