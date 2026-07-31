param(
    [string]$SourcePath = '',
    [string]$ServerHost = 'root@43.139.149.158',
    [string]$RemotePath = '/opt/elon/data/mobile-pwa/web_page.html',
    [switch]$PlanOnly
)

$ErrorActionPreference = 'Stop'
. (Join-Path $PSScriptRoot 'native-command-timeout.ps1')
. (Join-Path $PSScriptRoot 'direct-network.ps1')
Set-ElonProjectDirectNetwork

$repoRoot = (& git -C $PSScriptRoot rev-parse --show-toplevel 2>$null).Trim()
if ($LASTEXITCODE -ne 0 -or [string]::IsNullOrWhiteSpace($repoRoot)) {
    throw 'Run this script inside the repository.'
}
if ([string]::IsNullOrWhiteSpace($SourcePath)) {
    $SourcePath = Join-Path $repoRoot 'server\src\assets\web_page.html'
}
$SourcePath = [System.IO.Path]::GetFullPath($SourcePath)
if (-not (Test-Path -LiteralPath $SourcePath -PathType Leaf)) {
    throw "Mobile PWA template does not exist: $SourcePath"
}

$sourceSha = (& git -C $repoRoot rev-parse HEAD).Trim()
$localHash = (Get-FileHash -LiteralPath $SourcePath -Algorithm SHA256).Hash.ToLowerInvariant()
$remoteDirectory = Split-Path $RemotePath -Parent
$stagingPath = "$RemotePath.$sourceSha.tmp"

Write-Host 'MOBILE_PWA_STATIC_POLICY=atomic_template_without_server_rebuild'
Write-Host "MOBILE_PWA_STATIC_SOURCE_SHA=$sourceSha"
Write-Host "MOBILE_PWA_STATIC_SHA256=$localHash"
if ($PlanOnly) {
    Write-Host 'MOBILE_PWA_STATIC_RESULT=planned'
    exit 0
}

$sshOptions = @(
    '-n', '-o', 'ProxyCommand=none', '-o', 'ProxyJump=none',
    '-o', 'BatchMode=yes', '-o', 'ConnectTimeout=10',
    '-o', 'ServerAliveInterval=5', '-o', 'ServerAliveCountMax=1'
)
$scpOptions = @(
    '-o', 'ProxyCommand=none', '-o', 'ProxyJump=none',
    '-o', 'BatchMode=yes', '-o', 'ConnectTimeout=10',
    '-o', 'ServerAliveInterval=5', '-o', 'ServerAliveCountMax=1'
)

$prepare = Invoke-ElonNativeCommand -FilePath 'ssh.exe' -TimeoutSeconds 30 -Label 'mobile-pwa-prepare' `
    -ArgumentList ($sshOptions + @($ServerHost, "mkdir -p '$remoteDirectory'"))
Assert-ElonNativeCommand -Result $prepare -FailureMessage 'Unable to prepare mobile PWA directory.'

$upload = Invoke-ElonNativeCommand -FilePath 'scp.exe' -TimeoutSeconds 180 -Label 'mobile-pwa-upload' `
    -ArgumentList ($scpOptions + @($SourcePath, "${ServerHost}:${stagingPath}"))
Assert-ElonNativeCommand -Result $upload -FailureMessage 'Unable to upload mobile PWA template.'

$swapCommand = "set -eu; actual=`$(sha256sum '$stagingPath' | awk '{print `$1}'); " +
    "[ `"`$actual`" = '$localHash' ]; chmod 0644 '$stagingPath'; mv -f '$stagingPath' '$RemotePath'"
$swap = Invoke-ElonNativeCommand -FilePath 'ssh.exe' -TimeoutSeconds 30 -Label 'mobile-pwa-swap' `
    -ArgumentList ($sshOptions + @($ServerHost, $swapCommand))
Assert-ElonNativeCommand -Result $swap -FailureMessage 'Unable to atomically publish mobile PWA template.'

$verifyCommand = "printf '%s %s' `"`$(sha256sum '$RemotePath' | awk '{print `$1}')`" `"`$(stat -c %s '$RemotePath')`""
$verify = Invoke-ElonNativeCommand -FilePath 'ssh.exe' -TimeoutSeconds 30 -Label 'mobile-pwa-verify' `
    -ArgumentList ($sshOptions + @($ServerHost, $verifyCommand))
Assert-ElonNativeCommand -Result $verify -FailureMessage 'Unable to verify mobile PWA template.'
$parts = $verify.Stdout.Trim() -split '\s+'
$expectedSize = (Get-Item -LiteralPath $SourcePath).Length
if ($parts.Count -lt 2 -or $parts[0] -ne $localHash -or [int64]$parts[1] -ne $expectedSize) {
    throw "Mobile PWA verification mismatch: $($verify.Stdout.Trim())"
}

try {
    $response = Invoke-WebRequest -Uri 'http://43.139.149.158:8080/' -UseBasicParsing -TimeoutSec 15
    $runtimeSource = [string]$response.Headers['X-Elon-Mobile-Pwa-Source']
    if ($response.StatusCode -ne 200 -or $runtimeSource -ne 'runtime') {
        throw "HTTP runtime source is '$runtimeSource' with status $($response.StatusCode)"
    }
} catch {
    throw "Published template is present but runtime activation check failed: $_"
}

Write-Host "MOBILE_PWA_STATIC_DURATION_SECONDS=$([Math]::Round(($prepare.DurationSeconds + $upload.DurationSeconds + $swap.DurationSeconds + $verify.DurationSeconds), 1))"
Write-Host 'MOBILE_PWA_STATIC_RESULT=passed'
