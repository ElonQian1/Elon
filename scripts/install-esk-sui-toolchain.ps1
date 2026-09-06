#requires -Version 7.0
[CmdletBinding()]
param(
    [Parameter(Mandatory = $true)]
    [string]$InstallRoot,

    [string]$ArchivePath = "",

    [string]$FrameworkArchivePath = ""
)

$ErrorActionPreference = "Stop"
$ProgressPreference = "SilentlyContinue"

function Stop-ToolchainInstall {
    param([string]$Message)
    throw "ESK Sui toolchain install failed: $Message"
}

function Get-Sha256Label {
    param([string]$Path)
    if (-not (Test-Path -LiteralPath $Path -PathType Leaf)) {
        Stop-ToolchainInstall "required file is missing: $Path"
    }
    return "sha256:$((Get-FileHash -LiteralPath $Path -Algorithm SHA256).Hash.ToLowerInvariant())"
}

function Assert-Sha256 {
    param(
        [string]$Path,
        [string]$Expected,
        [string]$Label
    )
    $actual = Get-Sha256Label -Path $Path
    if ($actual -cne $Expected) {
        Stop-ToolchainInstall "$Label digest mismatch; expected $Expected, observed $actual"
    }
}

function Assert-FileLength {
    param(
        [string]$Path,
        [long]$Expected,
        [string]$Label
    )
    if (-not (Test-Path -LiteralPath $Path -PathType Leaf)) {
        Stop-ToolchainInstall "$Label is missing"
    }
    $actual = (Get-Item -LiteralPath $Path).Length
    if ($actual -ne $Expected) {
        Stop-ToolchainInstall "$Label length mismatch; expected $Expected, observed $actual"
    }
}

function Assert-NotReparsePoint {
    param(
        [string]$Path,
        [string]$Label
    )
    if (-not (Test-Path -LiteralPath $Path)) {
        Stop-ToolchainInstall "$Label is missing"
    }
    $item = Get-Item -LiteralPath $Path -Force
    if (($item.Attributes -band [System.IO.FileAttributes]::ReparsePoint) -ne 0) {
        Stop-ToolchainInstall "$Label cannot be a reparse point"
    }
}

function Assert-NoReparsePathChain {
    param(
        [string]$Path,
        [string]$Label
    )
    $full = [System.IO.Path]::GetFullPath($Path)
    $volume = [System.IO.Path]::GetPathRoot($full)
    $relative = $full.Substring($volume.Length)
    $current = $volume
    foreach ($segment in $relative.Split(
        [System.IO.Path]::DirectorySeparatorChar,
        [System.StringSplitOptions]::RemoveEmptyEntries
    )) {
        $current = Join-Path $current $segment
        if (Test-Path -LiteralPath $current) {
            Assert-NotReparsePoint -Path $current -Label $Label
        }
    }
}

function Assert-FixedToolchainLayout {
    param(
        [string]$Path,
        [string]$Label
    )
    if (-not (Test-Path -LiteralPath $Path -PathType Container)) {
        Stop-ToolchainInstall "$Label directory is missing"
    }
    $entries = @(Get-ChildItem -LiteralPath $Path -Force)
    $names = @($entries | ForEach-Object { $_.Name } | Sort-Object)
    if (($names -join "`n") -cne "framework-source.tar.gz`nsui.exe" -or
        @($entries | Where-Object { -not $_.PSIsContainer -and $_.Name -cin @(
            "framework-source.tar.gz", "sui.exe"
        ) }).Count -ne 2) {
        Stop-ToolchainInstall "$Label must contain exactly the two fixed artifacts"
    }
}

function Assert-ChildPath {
    param(
        [string]$Parent,
        [string]$Child,
        [string]$Label
    )
    $parentFull = [System.IO.Path]::GetFullPath($Parent).TrimEnd([System.IO.Path]::DirectorySeparatorChar)
    $childFull = [System.IO.Path]::GetFullPath($Child)
    $prefix = "$parentFull$([System.IO.Path]::DirectorySeparatorChar)"
    if (-not $childFull.StartsWith($prefix, [System.StringComparison]::OrdinalIgnoreCase)) {
        Stop-ToolchainInstall "$Label must stay below the explicit install root"
    }
}

function Remove-IsolatedDirectory {
    param(
        [string]$Root,
        [string]$Path
    )
    if (-not (Test-Path -LiteralPath $Path)) { return }
    Assert-ChildPath -Parent $Root -Child $Path -Label "temporary directory"
    Assert-NoReparsePathChain -Path $Root -Label "install root path"
    Assert-NoReparsePathChain -Path $Path -Label "temporary directory path"
    Remove-Item -LiteralPath $Path -Recurse -Force
}

function Invoke-SuiVersion {
    param(
        [string]$BinaryPath,
        [string]$ConfigDirectory
    )
    $start = [System.Diagnostics.ProcessStartInfo]::new()
    $start.FileName = $BinaryPath
    $start.ArgumentList.Add("--version")
    $start.UseShellExecute = $false
    $start.CreateNoWindow = $true
    $start.RedirectStandardOutput = $true
    $start.RedirectStandardError = $true
    $start.Environment["SUI_CONFIG_DIR"] = $ConfigDirectory
    $process = [System.Diagnostics.Process]::new()
    $process.StartInfo = $start
    $cancellation = [System.Threading.CancellationTokenSource]::new()
    $cancellation.CancelAfter(30000)
    try {
        if (-not $process.Start()) { Stop-ToolchainInstall "could not start the CLI version probe" }
        $stdoutTask = $process.StandardOutput.ReadToEndAsync()
        $stderrTask = $process.StandardError.ReadToEndAsync()
        try {
            $null = $process.WaitForExitAsync($cancellation.Token).GetAwaiter().GetResult()
        } catch [System.OperationCanceledException] {
            try { $process.Kill($true) } catch { }
            try { $process.WaitForExit(5000) | Out-Null } catch { }
            Stop-ToolchainInstall "CLI version probe timed out"
        }
        $stdout = $stdoutTask.GetAwaiter().GetResult()
        $null = $stderrTask.GetAwaiter().GetResult()
        if ($process.ExitCode -ne 0) {
            Stop-ToolchainInstall "CLI version probe exited $($process.ExitCode)"
        }
        return $stdout.Trim()
    } finally {
        $cancellation.Dispose()
        $process.Dispose()
    }
}

function Receive-FixedArchive {
    param(
        [uri]$InitialUri,
        [string]$Destination,
        [long]$ExpectedLength,
        [string[]]$InitialHosts,
        [string[]]$RedirectHosts,
        [string]$Label
    )
    if ($ExpectedLength -le 0 -or $ExpectedLength -eq [long]::MaxValue) {
        Stop-ToolchainInstall "$Label has an invalid expected length"
    }
    $handler = [System.Net.Http.HttpClientHandler]::new()
    $handler.AllowAutoRedirect = $false
    $client = [System.Net.Http.HttpClient]::new($handler)
    $client.Timeout = [System.Threading.Timeout]::InfiniteTimeSpan
    $client.DefaultRequestHeaders.UserAgent.ParseAdd("Yilong-ESK-Sui-CI/1")
    $cancellation = [System.Threading.CancellationTokenSource]::new()
    $cancellation.CancelAfter([System.TimeSpan]::FromMinutes(10))
    $current = $InitialUri
    $redirects = 0
    try {
        while ($true) {
            $allowedHosts = if ($redirects -eq 0) {
                $InitialHosts
            } else {
                $RedirectHosts
            }
            if (-not $current.IsAbsoluteUri -or $current.Scheme -cne "https" -or
                -not $current.IsDefaultPort -or -not [string]::IsNullOrEmpty($current.UserInfo) -or
                $current.Host -cnotin $allowedHosts) {
                Stop-ToolchainInstall "$Label download attempted an unapproved origin"
            }
            try {
                $response = $client.GetAsync(
                    $current,
                    [System.Net.Http.HttpCompletionOption]::ResponseHeadersRead,
                    $cancellation.Token
                ).GetAwaiter().GetResult()
            } catch [System.OperationCanceledException] {
                Stop-ToolchainInstall "$Label download timed out"
            } catch {
                Stop-ToolchainInstall "$Label download transport failed"
            }
            $status = [int]$response.StatusCode
            if ($status -in @(301, 302, 303, 307, 308)) {
                if ($redirects -ge 3 -or $null -eq $response.Headers.Location) {
                    $response.Dispose()
                    Stop-ToolchainInstall "$Label download exceeded the redirect limit"
                }
                try {
                    $next = if ($response.Headers.Location.IsAbsoluteUri) {
                        $response.Headers.Location
                    } else {
                        [uri]::new($current, $response.Headers.Location)
                    }
                } catch {
                    $response.Dispose()
                    Stop-ToolchainInstall "$Label download returned an invalid redirect"
                }
                $response.Dispose()
                $redirects++
                $current = $next
                continue
            }
            if ($status -ne 200) {
                $response.Dispose()
                Stop-ToolchainInstall "$Label download returned HTTP $status"
            }
            try {
                $declaredLength = $response.Content.Headers.ContentLength
                if ($null -ne $declaredLength -and [long]$declaredLength -ne $ExpectedLength) {
                    Stop-ToolchainInstall "$Label Content-Length differs from the fixed contract"
                }
                try {
                    $source = $response.Content.ReadAsStreamAsync($cancellation.Token).GetAwaiter().GetResult()
                } catch [System.OperationCanceledException] {
                    Stop-ToolchainInstall "$Label download timed out"
                } catch {
                    Stop-ToolchainInstall "$Label response stream could not be opened"
                }
                $target = [System.IO.FileStream]::new(
                    $Destination,
                    [System.IO.FileMode]::CreateNew,
                    [System.IO.FileAccess]::Write,
                    [System.IO.FileShare]::None,
                    131072,
                    [System.IO.FileOptions]::Asynchronous
                )
                try {
                    $buffer = [byte[]]::new(131072)
                    $total = 0L
                    while ($true) {
                        $remainingWithSentinel = ($ExpectedLength + 1L) - $total
                        if ($remainingWithSentinel -le 0) {
                            Stop-ToolchainInstall "$Label exceeded the fixed length"
                        }
                        $readLength = [int][Math]::Min([long]$buffer.Length, $remainingWithSentinel)
                        try {
                            $read = $source.ReadAsync(
                                $buffer, 0, $readLength, $cancellation.Token
                            ).GetAwaiter().GetResult()
                        } catch [System.OperationCanceledException] {
                            Stop-ToolchainInstall "$Label download timed out"
                        } catch {
                            Stop-ToolchainInstall "$Label download read failed"
                        }
                        if ($read -eq 0) { break }
                        $total += $read
                        if ($total -gt $ExpectedLength) {
                            Stop-ToolchainInstall "$Label exceeded the fixed length"
                        }
                        try {
                            $null = $target.WriteAsync(
                                $buffer, 0, $read, $cancellation.Token
                            ).GetAwaiter().GetResult()
                        } catch [System.OperationCanceledException] {
                            Stop-ToolchainInstall "$Label download timed out"
                        } catch {
                            Stop-ToolchainInstall "$Label download write failed"
                        }
                    }
                    if ($total -ne $ExpectedLength) {
                        Stop-ToolchainInstall "$Label ended before the fixed length"
                    }
                    try {
                        $null = $target.FlushAsync($cancellation.Token).GetAwaiter().GetResult()
                    } catch [System.OperationCanceledException] {
                        Stop-ToolchainInstall "$Label download timed out"
                    } catch {
                        Stop-ToolchainInstall "$Label download flush failed"
                    }
                } finally {
                    $target.Dispose()
                    $source.Dispose()
                }
            } finally {
                $response.Dispose()
            }
            return
        }
    } finally {
        $cancellation.Dispose()
        $client.Dispose()
        $handler.Dispose()
    }
}

function Get-TrustedSystemTar {
    $systemDirectory = [Environment]::GetFolderPath(
        [Environment+SpecialFolder]::System)
    if ([string]::IsNullOrWhiteSpace($systemDirectory)) {
        Stop-ToolchainInstall "Windows system directory is unavailable"
    }
    $systemDirectory = [System.IO.Path]::GetFullPath($systemDirectory)
    $tarPath = Join-Path $systemDirectory "tar.exe"
    foreach ($candidate in @(
        @{ Path = $systemDirectory; Label = "Windows system directory" },
        @{ Path = $tarPath; Label = "Windows system tar.exe" }
    )) {
        Assert-NotReparsePoint -Path $candidate.Path -Label $candidate.Label
    }
    if (-not (Test-Path -LiteralPath $tarPath -PathType Leaf)) {
        Stop-ToolchainInstall "Windows system tar.exe is missing"
    }
    $signature = Get-AuthenticodeSignature -LiteralPath $tarPath
    if ($signature.Status.ToString() -cne "Valid" -or $null -eq $signature.SignerCertificate -or
        $signature.SignerCertificate.Subject -notmatch '(?:^|,\s*)O=Microsoft Corporation(?:,|$)') {
        Stop-ToolchainInstall "Windows system tar.exe does not have a valid Microsoft signature"
    }
    return $tarPath
}

if (-not $IsWindows -or [System.Runtime.InteropServices.RuntimeInformation]::OSArchitecture -ne
    [System.Runtime.InteropServices.Architecture]::X64) {
    Stop-ToolchainInstall "V1 only supports Windows x86_64"
}

$contractPath = Join-Path $PSScriptRoot "esk-sui-toolchain-ci\toolchain-v1.json"
if (-not (Test-Path -LiteralPath $contractPath -PathType Leaf)) {
    Stop-ToolchainInstall "toolchain contract is missing"
}
$contract = Get-Content -LiteralPath $contractPath -Raw | ConvertFrom-Json
if ($contract.schema -cne "yilong.esk.sui.reproducible_toolchain.v1") {
    Stop-ToolchainInstall "toolchain contract schema is unsupported"
}
if ([string]::IsNullOrWhiteSpace([string]$contract.framework.archive_url) -or
    [long]$contract.framework.archive_size -le 0 -or
    [string]$contract.framework.archive_sha256 -cnotmatch '^sha256:[0-9a-f]{64}$') {
    Stop-ToolchainInstall "Framework archive contract is incomplete"
}

$rootFull = [System.IO.Path]::GetFullPath($InstallRoot)
$volumeRoot = [System.IO.Path]::GetPathRoot($rootFull).TrimEnd([System.IO.Path]::DirectorySeparatorChar)
if ($rootFull.TrimEnd([System.IO.Path]::DirectorySeparatorChar) -eq $volumeRoot) {
    Stop-ToolchainInstall "InstallRoot cannot be a volume root"
}
Assert-NoReparsePathChain -Path $rootFull -Label "install root path"
New-Item -ItemType Directory -Path $rootFull -Force | Out-Null
Assert-NoReparsePathChain -Path $rootFull -Label "install root path"

$targetDirectory = Join-Path $rootFull "$($contract.cli.release)\$($contract.cli.platform)"
$binaryPath = Join-Path $targetDirectory "sui.exe"
$installedFrameworkArchive = Join-Path $targetDirectory "framework-source.tar.gz"
$probeDirectory = Join-Path $rootFull ".version-check-$([guid]::NewGuid().ToString('N'))"
$temporaryDirectory = Join-Path $rootFull ".install-$([guid]::NewGuid().ToString('N'))"
Assert-ChildPath -Parent $rootFull -Child $targetDirectory -Label "toolchain target"

try {
    New-Item -ItemType Directory -Path $probeDirectory -Force | Out-Null
    Assert-NoReparsePathChain -Path $probeDirectory -Label "version probe path"
    if (Test-Path -LiteralPath $targetDirectory) {
        Assert-NoReparsePathChain -Path $targetDirectory -Label "toolchain target path"
    }
    $hasCachedBinary = Test-Path -LiteralPath $binaryPath -PathType Leaf
    $hasCachedFramework = Test-Path -LiteralPath $installedFrameworkArchive -PathType Leaf
    if ($hasCachedBinary -and $hasCachedFramework) {
        Assert-NotReparsePoint -Path $targetDirectory -Label "cached toolchain directory"
        Assert-FixedToolchainLayout -Path $targetDirectory -Label "cached toolchain"
        Assert-NotReparsePoint -Path $binaryPath -Label "cached CLI"
        Assert-NotReparsePoint -Path $installedFrameworkArchive -Label "cached Framework source archive"
        Assert-FileLength -Path $binaryPath -Expected $contract.cli.binary_size -Label "cached CLI"
        Assert-Sha256 -Path $binaryPath -Expected $contract.cli.binary_sha256 -Label "cached CLI"
        Assert-FileLength -Path $installedFrameworkArchive -Expected $contract.framework.archive_size `
            -Label "cached Framework source archive"
        Assert-Sha256 -Path $installedFrameworkArchive -Expected $contract.framework.archive_sha256 `
            -Label "cached Framework source archive"
        $version = Invoke-SuiVersion -BinaryPath $binaryPath -ConfigDirectory $probeDirectory
        if ($version -cne $contract.cli.version) {
            Stop-ToolchainInstall "cached CLI version mismatch; expected '$($contract.cli.version)', observed '$version'"
        }
        Write-Output "SUI_TOOLCHAIN_STATUS=verified source=cache release=$($contract.cli.release)"
        Write-Output "SUI_TOOLCHAIN_PATH=$binaryPath"
        Write-Output "SUI_FRAMEWORK_SOURCE_ARCHIVE_PATH=$installedFrameworkArchive"
        exit 0
    }
    if (Test-Path -LiteralPath $targetDirectory) {
        Stop-ToolchainInstall "toolchain target exists without the complete fixed artifact set"
    }

    New-Item -ItemType Directory -Path $temporaryDirectory -Force | Out-Null
    Assert-NoReparsePathChain -Path $temporaryDirectory -Label "temporary install path"
    $archive = if ([string]::IsNullOrWhiteSpace($ArchivePath)) {
        $download = Join-Path $temporaryDirectory "sui-release.tgz"
        Receive-FixedArchive -InitialUri ([uri]$contract.cli.asset_url) -Destination $download `
            -ExpectedLength ([long]$contract.cli.archive_size) -InitialHosts @("github.com") `
            -RedirectHosts @("release-assets.githubusercontent.com") -Label "official CLI archive"
        $download
    } else {
        [System.IO.Path]::GetFullPath($ArchivePath)
    }
    Assert-FileLength -Path $archive -Expected $contract.cli.archive_size -Label "official release archive"
    Assert-Sha256 -Path $archive -Expected $contract.cli.archive_sha256 -Label "official release archive"

    $frameworkArchive = if ([string]::IsNullOrWhiteSpace($FrameworkArchivePath)) {
        $null
    } else {
        [System.IO.Path]::GetFullPath($FrameworkArchivePath)
    }
    $frameworkTargetMatch = $null -ne $frameworkArchive -and [string]::Equals(
        $frameworkArchive,
        [System.IO.Path]::GetFullPath($installedFrameworkArchive),
        [System.StringComparison]::OrdinalIgnoreCase
    )
    if ($null -eq $frameworkArchive -or ($frameworkTargetMatch -and
        -not (Test-Path -LiteralPath $frameworkArchive -PathType Leaf))) {
        $frameworkDownload = Join-Path $temporaryDirectory "framework-source-download.tar.gz"
        Receive-FixedArchive -InitialUri ([uri]$contract.framework.archive_url) `
            -Destination $frameworkDownload -ExpectedLength ([long]$contract.framework.archive_size) `
            -InitialHosts @("codeload.github.com") -RedirectHosts @("codeload.github.com") `
            -Label "official Framework source archive"
        $frameworkArchive = $frameworkDownload
    } elseif (-not (Test-Path -LiteralPath $frameworkArchive -PathType Leaf)) {
        Stop-ToolchainInstall "explicit Framework source archive is missing"
    }
    Assert-FileLength -Path $frameworkArchive -Expected $contract.framework.archive_size `
        -Label "official Framework source archive"
    Assert-Sha256 -Path $frameworkArchive -Expected $contract.framework.archive_sha256 `
        -Label "official Framework source archive"

    $tarPath = Get-TrustedSystemTar
    $entries = @(& $tarPath -tzf $archive 2>&1)
    if ($LASTEXITCODE -ne 0) { Stop-ToolchainInstall "could not list the official release archive" }
    $binaryEntries = @($entries | ForEach-Object { $_.ToString() } | Where-Object {
        [System.IO.Path]::GetFileName($_.Replace('/', '\')) -ceq "sui.exe"
    })
    if ($binaryEntries.Count -ne 1) {
        Stop-ToolchainInstall "archive must contain exactly one sui.exe; observed $($binaryEntries.Count)"
    }
    $entry = $binaryEntries[0].Replace('\', '/')
    if ($entry -cne "./sui.exe") {
        Stop-ToolchainInstall "archive must contain sui.exe at the fixed root entry"
    }

    $staging = Join-Path $temporaryDirectory "staging"
    New-Item -ItemType Directory -Path $staging -Force | Out-Null
    & $tarPath -xzf $archive -C $staging $entry
    if ($LASTEXITCODE -ne 0) { Stop-ToolchainInstall "could not extract sui.exe" }
    $candidates = @(Get-ChildItem -LiteralPath $staging -Filter "sui.exe" -File -Recurse)
    if ($candidates.Count -ne 1) {
        Stop-ToolchainInstall "extraction must produce exactly one sui.exe"
    }
    Assert-NotReparsePoint -Path $candidates[0].FullName -Label "extracted CLI"
    Assert-FileLength -Path $candidates[0].FullName -Expected $contract.cli.binary_size -Label "extracted CLI"
    Assert-Sha256 -Path $candidates[0].FullName -Expected $contract.cli.binary_sha256 -Label "extracted CLI"
    $version = Invoke-SuiVersion -BinaryPath $candidates[0].FullName -ConfigDirectory $probeDirectory
    if ($version -cne $contract.cli.version) {
        Stop-ToolchainInstall "extracted CLI version mismatch; expected '$($contract.cli.version)', observed '$version'"
    }

    $promoteDirectory = Join-Path $temporaryDirectory "toolchain"
    New-Item -ItemType Directory -Path $promoteDirectory | Out-Null
    Move-Item -LiteralPath $candidates[0].FullName -Destination (Join-Path $promoteDirectory "sui.exe")
    Copy-Item -LiteralPath $frameworkArchive `
        -Destination (Join-Path $promoteDirectory "framework-source.tar.gz")
    Assert-NotReparsePoint -Path (Join-Path $promoteDirectory "sui.exe") -Label "staged CLI"
    Assert-NotReparsePoint -Path (Join-Path $promoteDirectory "framework-source.tar.gz") `
        -Label "staged Framework source archive"
    Assert-FileLength -Path (Join-Path $promoteDirectory "sui.exe") `
        -Expected $contract.cli.binary_size -Label "staged CLI"
    Assert-Sha256 -Path (Join-Path $promoteDirectory "sui.exe") `
        -Expected $contract.cli.binary_sha256 -Label "staged CLI"
    Assert-FileLength -Path (Join-Path $promoteDirectory "framework-source.tar.gz") `
        -Expected $contract.framework.archive_size -Label "staged Framework source archive"
    Assert-Sha256 -Path (Join-Path $promoteDirectory "framework-source.tar.gz") `
        -Expected $contract.framework.archive_sha256 -Label "staged Framework source archive"
    $targetParent = Split-Path -Parent $targetDirectory
    Assert-NoReparsePathChain -Path $targetParent -Label "toolchain parent path"
    New-Item -ItemType Directory -Path $targetParent -Force | Out-Null
    Assert-NoReparsePathChain -Path $targetParent `
        -Label "toolchain parent path"
    Move-Item -LiteralPath $promoteDirectory -Destination $targetDirectory
    Assert-NoReparsePathChain -Path $targetDirectory -Label "installed toolchain path"
    Assert-NotReparsePoint -Path $targetDirectory -Label "installed toolchain directory"
    Assert-FixedToolchainLayout -Path $targetDirectory -Label "installed toolchain"
    Assert-NotReparsePoint -Path $binaryPath -Label "installed CLI"
    Assert-NotReparsePoint -Path $installedFrameworkArchive -Label "installed Framework source archive"
    Assert-FileLength -Path $binaryPath -Expected $contract.cli.binary_size -Label "installed CLI"
    Assert-Sha256 -Path $binaryPath -Expected $contract.cli.binary_sha256 -Label "installed CLI"
    Assert-FileLength -Path $installedFrameworkArchive -Expected $contract.framework.archive_size `
        -Label "installed Framework source archive"
    Assert-Sha256 -Path $installedFrameworkArchive -Expected $contract.framework.archive_sha256 `
        -Label "installed Framework source archive"
    $installedVersion = Invoke-SuiVersion -BinaryPath $binaryPath -ConfigDirectory $probeDirectory
    if ($installedVersion -cne $contract.cli.version) {
        Stop-ToolchainInstall "installed CLI version mismatch; expected '$($contract.cli.version)', observed '$installedVersion'"
    }
    Write-Output "SUI_TOOLCHAIN_STATUS=verified source=official_archive release=$($contract.cli.release)"
    Write-Output "SUI_TOOLCHAIN_PATH=$binaryPath"
    Write-Output "SUI_FRAMEWORK_SOURCE_ARCHIVE_PATH=$installedFrameworkArchive"
} finally {
    Remove-IsolatedDirectory -Root $rootFull -Path $probeDirectory
    Remove-IsolatedDirectory -Root $rootFull -Path $temporaryDirectory
}
