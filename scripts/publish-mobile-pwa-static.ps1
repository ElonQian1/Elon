param(
    [string]$SourcePath = '',
    [string]$ServerHost = 'root@43.139.149.158',
    [string]$RemotePath = '/opt/elon/data/mobile-pwa/web_page.html',
    [switch]$VerifyOnly,
    [switch]$PlanOnly
)

$ErrorActionPreference = 'Stop'
. (Join-Path $PSScriptRoot 'native-command-timeout.ps1')
. (Join-Path $PSScriptRoot 'direct-network.ps1')
. (Join-Path $PSScriptRoot 'app-ui-change-scope.ps1')
. (Join-Path $PSScriptRoot 'mobile-pwa-runtime-template.ps1')
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
$assetDirectory = Split-Path -Parent $SourcePath
$stylesPath = Join-Path $assetDirectory 'project_plaza.css'
$themeStylesPath = Join-Path $assetDirectory 'orbital_mobile_theme.css'
$cacheScriptPath = Join-Path $assetDirectory 'project_plaza_cache.js'
$scriptPath = Join-Path $assetDirectory 'project_plaza.js'
$runtimeTemplatePath = Join-Path $repoRoot ".ai-tmp\mobile-pwa-static\web_page.$sourceSha.$PID.html"
$runtimeTemplate = New-ElonMobilePwaRuntimeTemplate -TemplatePath $SourcePath `
    -StylesPath $stylesPath -ThemeStylesPath $themeStylesPath -CacheScriptPath $cacheScriptPath `
    -ScriptPath $scriptPath -OutputPath $runtimeTemplatePath
$localHash = (Get-FileHash -LiteralPath $runtimeTemplate.FullName -Algorithm SHA256).Hash.ToLowerInvariant()
$remoteSeparator = $RemotePath.LastIndexOf('/')
if ($remoteSeparator -le 0) { throw "RemotePath must be an absolute POSIX path: $RemotePath" }
$remoteDirectory = $RemotePath.Substring(0, $remoteSeparator)
$stagingPath = "$RemotePath.$sourceSha.tmp"
$metadataPath = "$RemotePath.release.v1"
$metadataStagingPath = "$metadataPath.$sourceSha.tmp"

Write-Host 'MOBILE_PWA_STATIC_POLICY=atomic_template_without_server_rebuild'
Write-Host 'MOBILE_PWA_STATIC_TEMPLATE=self_contained_runtime_assets'
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

function Invoke-MobilePwaRemoteVerification {
    $verifyCommand = "set -eu; printf '%s %s' `"`$(sha256sum '$RemotePath' | awk '{print `$1}')`" `"`$(stat -c %s '$RemotePath')`""
    $verify = Invoke-ElonNativeCommand -FilePath 'ssh.exe' -TimeoutSeconds 30 -Label 'mobile-pwa-verify' `
        -ArgumentList ($sshOptions + @($ServerHost, $verifyCommand))
    Assert-ElonNativeCommand -Result $verify -FailureMessage 'Unable to verify mobile PWA template.'
    $verifyText = ([string]$verify.Stdout).Trim()
    $parts = $verifyText -split '\s+'
    $expectedSize = $runtimeTemplate.Length
    if ($parts.Count -lt 2 -or $parts[0] -ne $localHash -or [int64]$parts[1] -ne $expectedSize) {
        throw "Mobile PWA verification mismatch: $verifyText"
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
    $verify
}

if ($VerifyOnly) {
    $verified = Invoke-MobilePwaRemoteVerification
    Write-Host "MOBILE_PWA_STATIC_DURATION_SECONDS=$([Math]::Round($verified.DurationSeconds, 1))"
    Write-Host 'MOBILE_PWA_STATIC_RESULT=verified_existing'
    exit 0
}

$fetch = Invoke-ElonGitHubGitWithProxyFallback -RepoPath $repoRoot -GitArgs @('fetch', 'origin', 'main')
Write-Host "GITHUB_SSH_ROUTE=$($fetch.Route)"
if ($fetch.ExitCode -ne 0) { throw "Unable to refresh origin/main before mobile PWA publish. $($fetch.Hint)" }
$mainSha = (& git -C $repoRoot rev-parse origin/main).Trim()
& git -C $repoRoot merge-base --is-ancestor $sourceSha $mainSha 2>$null
if ($LASTEXITCODE -ne 0) { throw "Mobile PWA candidate is not an ancestor of origin/main: $sourceSha" }
$staticInputs = @(Get-ElonStaticMobilePwaInputPaths)
$newerInputChanges = @(& git -C $repoRoot diff --name-only "$sourceSha..$mainSha" -- @staticInputs 2>$null)
if ($newerInputChanges.Count -gt 0) {
    Write-Host "MOBILE_PWA_STATIC_SUPERSEDED_BY=$mainSha"
    Write-Host "MOBILE_PWA_STATIC_SUPERSEDED_INPUTS=$($newerInputChanges.Count)"
    Write-Host 'MOBILE_PWA_STATIC_RESULT=skipped_newer_generation'
    exit 0
}

$prepare = Invoke-ElonNativeCommand -FilePath 'ssh.exe' -TimeoutSeconds 30 -Label 'mobile-pwa-prepare' `
    -ArgumentList ($sshOptions + @($ServerHost, "mkdir -p '$remoteDirectory'"))
Assert-ElonNativeCommand -Result $prepare -FailureMessage 'Unable to prepare mobile PWA directory.'

$readMetadataCommand = "set -eu; if [ -f '$metadataPath' ]; then cat '$metadataPath'; fi"
$readMetadata = Invoke-ElonNativeCommand -FilePath 'ssh.exe' -TimeoutSeconds 30 -Label 'mobile-pwa-read-generation' `
    -ArgumentList ($sshOptions + @($ServerHost, $readMetadataCommand))
Assert-ElonNativeCommand -Result $readMetadata -FailureMessage 'Unable to read mobile PWA release generation.'
$expectedMetadata = ([string]$readMetadata.Stdout).Trim()
if (-not [string]::IsNullOrWhiteSpace($expectedMetadata)) {
    $metadataParts = $expectedMetadata -split '\s+'
    if ($metadataParts.Count -ne 3 -or $metadataParts[0] -notmatch '^[0-9a-f]{40}$' -or
        $metadataParts[1] -notmatch '^[0-9a-f]{64}$' -or $metadataParts[2] -notmatch '^\d+$') {
        throw "Invalid remote mobile PWA release metadata: $expectedMetadata"
    }
    $remoteSourceSha = $metadataParts[0]
    $remoteIntegrityCommand = "set -eu; [ `"`$(sha256sum '$RemotePath' | awk '{print `$1}')`" = '$($metadataParts[1])' ]; [ `"`$(stat -c %s '$RemotePath')`" = '$($metadataParts[2])' ]"
    $remoteIntegrity = Invoke-ElonNativeCommand -FilePath 'ssh.exe' -TimeoutSeconds 30 -Label 'mobile-pwa-generation-integrity' `
        -ArgumentList ($sshOptions + @($ServerHost, $remoteIntegrityCommand))
    Assert-ElonNativeCommand -Result $remoteIntegrity -FailureMessage 'Remote mobile PWA generation metadata does not match its template.'
    & git -C $repoRoot cat-file -e "$remoteSourceSha^{commit}" 2>$null
    if ($LASTEXITCODE -eq 0) {
        & git -C $repoRoot merge-base --is-ancestor $sourceSha $remoteSourceSha 2>$null
        if ($LASTEXITCODE -eq 0) {
            if ($remoteSourceSha -eq $sourceSha -and
                ($metadataParts[1] -ne $localHash -or [int64]$metadataParts[2] -ne $runtimeTemplate.Length)) {
                throw 'Remote mobile PWA uses the same source SHA with different runtime inputs.'
            }
            Write-Host "MOBILE_PWA_STATIC_COVERED_BY=$remoteSourceSha"
            Write-Host 'MOBILE_PWA_STATIC_RESULT=already_covered'
            exit 0
        }
        & git -C $repoRoot merge-base --is-ancestor $remoteSourceSha $sourceSha 2>$null
        if ($LASTEXITCODE -ne 0) {
            throw "Remote mobile PWA generation diverged from candidate: $remoteSourceSha"
        }
    } elseif ($remoteSourceSha -ne $sourceSha) {
        throw "Remote mobile PWA source commit is unavailable locally: $remoteSourceSha"
    }
}

$upload = Invoke-ElonNativeCommand -FilePath 'scp.exe' -TimeoutSeconds 180 -Label 'mobile-pwa-upload' `
    -ArgumentList ($scpOptions + @($runtimeTemplate.FullName, "${ServerHost}:${stagingPath}"))
Assert-ElonNativeCommand -Result $upload -FailureMessage 'Unable to upload mobile PWA template.'

$metadataLine = "$sourceSha $localHash $($runtimeTemplate.Length)"
$escapedExpectedMetadata = $expectedMetadata
$swapCommand = "set -eu; exec 9>'$metadataPath.lock'; flock -w 30 9; " +
    "current=`$(if [ -f '$metadataPath' ]; then cat '$metadataPath'; fi); " +
    "[ `"`$current`" = '$escapedExpectedMetadata' ] || exit 73; " +
    "actual=`$(sha256sum '$stagingPath' | awk '{print `$1}'); [ `"`$actual`" = '$localHash' ]; " +
    "printf '%s\n' '$metadataLine' > '$metadataStagingPath'; " +
    "chmod 0644 '$stagingPath' '$metadataStagingPath'; " +
    "mv -f '$stagingPath' '$RemotePath'; mv -f '$metadataStagingPath' '$metadataPath'"
$swap = Invoke-ElonNativeCommand -FilePath 'ssh.exe' -TimeoutSeconds 30 -Label 'mobile-pwa-swap' `
    -ArgumentList ($sshOptions + @($ServerHost, $swapCommand))
Assert-ElonNativeCommand -Result $swap -FailureMessage 'Unable to atomically publish mobile PWA template.'

$verify = Invoke-MobilePwaRemoteVerification

Write-Host "MOBILE_PWA_STATIC_DURATION_SECONDS=$([Math]::Round(($prepare.DurationSeconds + $upload.DurationSeconds + $swap.DurationSeconds + $verify.DurationSeconds), 1))"
Write-Host 'MOBILE_PWA_STATIC_RESULT=passed'
