<#
.SYNOPSIS
    Build and atomically publish only the React PC frontend.
.DESCRIPTION
    This path does not claim a server version or rebuild the Rust backend. It
    fails closed unless the live server commit is an ancestor of the candidate
    and no server/API contract inputs changed between them. A static release
    marker and remote compare-and-swap lock prevent stale frontend rollback.
#>
[CmdletBinding()]
param(
    [switch]$SkipBuild,
    [switch]$ReuseLiveServer
)

$ErrorActionPreference = 'Stop'
. (Join-Path $PSScriptRoot 'direct-network.ps1')
. (Join-Path $PSScriptRoot 'native-command-timeout.ps1')
. (Join-Path $PSScriptRoot 'publish-server-pc-frontend.ps1')
. (Join-Path $PSScriptRoot 'publish-health-checks.ps1')

Set-ElonProjectDirectNetwork
Set-ElonProjectDirectGitSsh

$RepoRoot = (& git -C $PSScriptRoot rev-parse --show-toplevel).Trim()
$FrontendDir = Join-Path $RepoRoot 'pc-frontend'
$DistDir = Join-Path $FrontendDir 'dist'
$Server = 'root@43.139.149.158'
$SshOpts = @('-o', 'ProxyCommand=none', '-o', 'ProxyJump=none')
$RemotePcDist = '/opt/elon/data/pc-next-dist'
$ServerUrl = 'http://43.139.149.158:8080'

function Stop-PcFrontendPublish([string]$Message) { throw "PC frontend publish refused: $Message" }

function Assert-GitAncestor([string]$Ancestor, [string]$Descendant, [string]$Message) {
    & git -C $RepoRoot merge-base --is-ancestor $Ancestor $Descendant *> $null
    if ($LASTEXITCODE -ne 0) { Stop-PcFrontendPublish $Message }
}

Push-Location $RepoRoot
try {
    if (& git status --porcelain --untracked-files=all) {
        Stop-PcFrontendPublish 'worktree is not clean; commit the exact frontend source first'
    }
    $fetch = Invoke-ElonGitHubGitWithProxyFallback -RepoPath $RepoRoot -GitArgs @('fetch', 'origin', 'main') -RemoteName 'origin'
    Write-Host "GITHUB_SSH_ROUTE=$($fetch.Route)"
    if ($fetch.ExitCode -ne 0) { Stop-PcFrontendPublish "origin/main fetch failed: $($fetch.Text)" }

    $Sha = (& git rev-parse 'HEAD^{commit}').Trim()
    Assert-GitAncestor $Sha 'origin/main' 'candidate commit is not contained in origin/main'
    $serverVersion = Invoke-ElonPublishJsonGet -Uri "$ServerUrl/api/server/version" -TimeoutSec 10
    $serverSha = [string]$serverVersion.gitSha
    if ($serverSha -notmatch '^[0-9a-f]{40}$') { Stop-PcFrontendPublish 'live server did not report a valid Git SHA' }
    & git cat-file -e "$serverSha^{commit}" 2>$null
    if ($LASTEXITCODE -ne 0) { Stop-PcFrontendPublish "live server commit is unavailable locally: $serverSha" }
    Assert-GitAncestor $serverSha $Sha 'candidate is not based on the live server commit'

    $compatibilityMode = 'direct_server_ancestry'
    $blocking = @(& git diff --name-only "$serverSha..$Sha" -- server contracts sdk)
    if ($blocking.Count -gt 0) {
        if (-not $ReuseLiveServer) {
            Stop-PcFrontendPublish "server/API inputs changed since the live server. If this frontend is independent, rerun with -ReuseLiveServer; otherwise use publish-server.ps1: $($blocking -join ', ')"
        }
        $coupledCommits = @(Get-PcFrontendBackendCoupledCommits -RepoRoot $RepoRoot `
            -BaseGitSha $serverSha -CandidateGitSha $Sha)
        if ($coupledCommits.Count -gt 0) {
            Stop-PcFrontendPublish "frontend commits also changed server/API inputs and cannot reuse the live server: $($coupledCommits -join '; ')"
        }
        $compatibilityMode = 'isolated_frontend_commits'
        Write-Host "PC_FRONTEND_COMPATIBILITY=isolated_frontend_commits live_server=$serverSha"
    }

    $currentReleaseSha = Get-PcFrontendReleaseBaseline -RepoRoot $RepoRoot -CandidateGitSha $Sha `
        -RemoteDir $RemotePcDist -SshServer $Server -SshOptions $SshOpts

    if (-not $SkipBuild) {
        if (-not (Get-Command npm.cmd -ErrorAction SilentlyContinue)) {
            Stop-PcFrontendPublish 'npm.cmd is unavailable'
        }
        Push-Location $FrontendDir
        try {
            $lockHash = (Get-FileHash (Join-Path $FrontendDir 'package-lock.json') -Algorithm MD5).Hash
            $installedHashPath = Join-Path $FrontendDir 'node_modules\.npm-installed-sha'
            $installedHash = if (Test-Path $installedHashPath) { (Get-Content $installedHashPath -Raw).Trim() } else { '' }
            if (-not (Test-Path (Join-Path $FrontendDir 'node_modules')) -or $installedHash -ne $lockHash) {
                $installExit = Invoke-LoggedCmd -Command 'npm.cmd ci --no-audit --no-fund'
                if ($installExit -ne 0) { throw "npm ci failed, exit=$installExit" }
                $lockHash | Set-Content -LiteralPath $installedHashPath -NoNewline
            }
            Reset-PcFrontendBuildArtifacts -FrontendDir $FrontendDir
            $buildExit = Invoke-LoggedCmd -Command 'npm.cmd run build'
            if ($buildExit -ne 0) { throw "npm run build failed, exit=$buildExit" }
        } finally {
            Pop-Location
        }
    }
    Publish-PcFrontendRelease -FrontendDir $FrontendDir -DistDir $DistDir -RepoRoot $RepoRoot `
        -GitSha $Sha -CompatibleServerGitSha $serverSha -ReleaseMode frontend_only `
        -CompatibilityMode $compatibilityMode `
        -ServerUrl $ServerUrl -RemoteDir $RemotePcDist -Label '新版 PC 前端 dist' `
        -ExpectedCurrentReleaseSha $currentReleaseSha

    $publishedMarker = Invoke-ElonPublishJsonGet -Uri "$ServerUrl/pc/assets/release.json" -TimeoutSec 10
    if ([string]$publishedMarker.gitSha -ne $Sha -or
        [string]$publishedMarker.compatibleServerGitSha -ne $serverSha) {
        throw 'published PC frontend release marker does not match the candidate'
    }
    $pc = Invoke-ElonPublishTextGet -Uri "$ServerUrl/pc" -TimeoutSec 10
    if ($pc -notmatch '<div id="root"') { throw '/pc did not return the React shell' }

    Write-Host 'PC_FRONTEND_RELEASE_STATUS=published'
    Write-Host "PC_FRONTEND_GIT_SHA=$Sha"
    Write-Host "COMPATIBLE_SERVER_GIT_SHA=$serverSha"
    Write-Host "SERVER_RELEASE_STATUS=compatible_existing:v$($serverVersion.versionName)"
} finally {
    Pop-Location
}
