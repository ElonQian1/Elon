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

function Assert-NodeAgentBackgroundGitLaunchPolicy {
    param(
        [Parameter(Mandatory = $true)][string]$RepoRoot
    )

    $root = [System.IO.Path]::GetFullPath($RepoRoot)
    $sourceRoots = @(
        (Join-Path $root "server\src"),
        (Join-Path $root "server\pc-dev-runtime\src")
    )
    $bareGitPattern = '(?m)\b(?:[A-Za-z_][A-Za-z0-9_]*\s*::\s*)*[A-Za-z_]*Command\s*::\s*new\s*\(\s*"git"\s*\)'
    $cmdGitPattern = '(?i)\bcmd(?:\.exe)?\s+/(?:c|k)\s+git(?:\.exe)?\b'
    $violations = New-Object System.Collections.Generic.List[string]
    $sourceCount = 0

    foreach ($sourceRoot in $sourceRoots) {
        if (-not (Test-Path -LiteralPath $sourceRoot -PathType Container)) {
            continue
        }
        foreach ($source in Get-ChildItem -LiteralPath $sourceRoot -Recurse -Filter "*.rs" -File) {
            $sourceCount++
            $body = [System.IO.File]::ReadAllText($source.FullName)
            foreach ($match in [System.Text.RegularExpressions.Regex]::Matches($body, $bareGitPattern)) {
                $line = 1 + ($body.Substring(0, $match.Index).Split("`n").Count - 1)
                $relative = $source.FullName.Substring($root.Length).TrimStart('\', '/')
                $violations.Add("${relative}:${line}: $($match.Value)")
            }
            foreach ($match in [System.Text.RegularExpressions.Regex]::Matches($body, $cmdGitPattern)) {
                $line = 1 + ($body.Substring(0, $match.Index).Split("`n").Count - 1)
                $relative = $source.FullName.Substring($root.Length).TrimStart('\', '/')
                $violations.Add("${relative}:${line}: $($match.Value)")
            }
        }
    }

    if ($violations.Count -gt 0) {
        throw "检测到绕过统一入口的裸 Git 启动。请使用 elon_pc_dev_runtime::git_command()：$($violations -join '; ')"
    }

    $commandProbePath = Join-Path $root "server\pc-dev-runtime\src\command_probe.rs"
    $serverWrapperPath = Join-Path $root "server\src\git_command_error.rs"
    if (-not (Test-Path -LiteralPath $commandProbePath -PathType Leaf) -or
        -not (Test-Path -LiteralPath $serverWrapperPath -PathType Leaf)) {
        throw "缺少统一 Git 启动入口源码，无法验证后台 Git 策略"
    }
    $commandProbe = [System.IO.File]::ReadAllText($commandProbePath)
    foreach ($requiredMarker in @(
        'pub fn git_command() -> Command',
        'CREATE_NO_WINDOW',
        'GIT_TERMINAL_PROMPT',
        'GCM_INTERACTIVE',
        'SSH_ASKPASS_REQUIRE',
        'stdin(Stdio::null())'
    )) {
        if (-not $commandProbe.Contains($requiredMarker)) {
            throw "统一 Git 启动入口缺少策略标记：$requiredMarker"
        }
    }
    $serverWrapper = [System.IO.File]::ReadAllText($serverWrapperPath)
    if (-not $serverWrapper.Contains('elon_pc_dev_runtime::git_command()')) {
        throw "server/node-agent Git 包装器未路由到统一入口"
    }

    $routeEvidence = @{
        "server\src\node_agent_exec.rs" = 'hide_tokio_command_window(&mut cmd)'
        "server\src\node_agent_cli_prompt_runner.rs" = 'hide_tokio_command_window(&mut cmd)'
        "server\src\node_agent_cli_pipe_sidecar_runner.rs" = 'hide_tokio_command_window(&mut command)'
        "server\src\node_agent_cli_sidecar_runner.rs" = 'creation_flags(CREATE_NO_WINDOW)'
        "server\src\node_agent_cli_pty.rs" = 'native_pty_system()'
    }
    foreach ($entry in $routeEvidence.GetEnumerator()) {
        $path = Join-Path $root $entry.Key
        if (-not (Test-Path -LiteralPath $path -PathType Leaf) -or
            -not [System.IO.File]::ReadAllText($path).Contains($entry.Value)) {
            throw "Route A/Exec 无窗口边界证据缺失：$($entry.Key) -> $($entry.Value)"
        }
    }

    # Codex CLI 内部自行创建的 Git 是外部程序的后代进程，不在一龙源码扫描范围内。
    # 一龙可强制的宿主边界是 pipe/direct/Exec 无窗口，以及 PTY 使用 Windows ConPTY。
    Write-Output "CODEX_CLI_GIT_BOUNDARY=external_descendant;host_pipe_direct_exec=hidden;host_pty=conpty"
    Write-Output "NODE_AGENT_BACKGROUND_GIT_GATE=passed;source_files=$sourceCount"
}
