Set-StrictMode -Version Latest

Import-Module (Join-Path $PSScriptRoot 'rust-cache\RustCache.Runtime.psm1') -Force -DisableNameChecking

function Invoke-NodeAgentPcFrontendBuild {
    if (-not (Test-Path (Join-Path $PcFrontendDir 'package.json'))) {
        throw "PC frontend project is missing: $PcFrontendDir"
    }
    if (-not (Get-Command 'npm' -ErrorAction SilentlyContinue)) {
        throw 'npm is not available for the bundled PC frontend build'
    }

    Write-Host '[2.4/4] Building bundled PC frontend...' -ForegroundColor Yellow
    Push-Location $PcFrontendDir
    try {
        $lockFile = Join-Path $PcFrontendDir 'package-lock.json'
        $nmDir = Join-Path $PcFrontendDir 'node_modules'
        $nmInstalled = Join-Path $nmDir '.npm-installed-sha'
        $lockHash = if (Test-Path $lockFile) {
            Get-NodeAgentFileSha256 -Path $lockFile
        } else { '' }
        $prevHash = if (Test-Path $nmInstalled) { Get-Content $nmInstalled -Raw } else { '' }
        if ((-not (Test-Path $nmDir)) -or ($lockHash -ne $prevHash.Trim())) {
            Write-Host '   Installing/updating frontend dependencies (npm ci)...' -ForegroundColor Gray
            $installExit = Invoke-LoggedCmd -Command 'npm ci'
            if ($installExit -ne 0) { throw "npm ci failed, exit=$installExit" }
            $lockHash | Set-Content $nmInstalled -NoNewline
        }
        Reset-PcFrontendBuildArtifacts -FrontendDir $PcFrontendDir
        $buildExit = Invoke-LoggedCmd -Command 'npm run build'
        if ($buildExit -ne 0) { throw "npm run build failed, exit=$buildExit" }
    } catch {
        $primaryBuildError = $_
        try {
            Reset-PcFrontendBuildArtifacts -FrontendDir $PcFrontendDir
            Invoke-PcFrontendLocalBuild -FrontendDir $PcFrontendDir
        } catch {
            try {
                Invoke-PcFrontendPnpmBuild -FrontendDir $PcFrontendDir
            } catch {
                throw "PC frontend build failed: $primaryBuildError; fallback: $_"
            }
        }
    } finally {
        Pop-Location
    }
    if (-not (Test-Path (Join-Path $PcDistDir 'index.html'))) {
        throw "PC frontend dist is missing index.html: $PcDistDir"
    }
    Write-Host "   PC frontend dist ready: $PcDistDir" -ForegroundColor Green
}

function Resolve-RipgrepExe {
    $candidates = @()
    $cmd = Get-Command rg -ErrorAction SilentlyContinue
    if ($cmd -and $cmd.Source) { $candidates += $cmd.Source }
    if ($env:LOCALAPPDATA) {
        $candidates += Join-Path $env:LOCALAPPDATA 'ElonNode\tools\ripgrep\bin\rg.exe'
        foreach ($root in @(
            (Join-Path $env:LOCALAPPDATA 'OpenAI\Codex\bin'),
            (Join-Path $env:LOCALAPPDATA 'ElonNode\tools\ripgrep')
        )) {
            if (-not (Test-Path -LiteralPath $root -PathType Container)) { continue }
            Get-ChildItem -LiteralPath $root -Recurse -Filter 'rg.exe' -File -ErrorAction SilentlyContinue |
                ForEach-Object { $candidates += $_.FullName }
        }
    }
    foreach ($candidate in $candidates) {
        if (-not [string]::IsNullOrWhiteSpace($candidate) -and
            (Test-Path -LiteralPath $candidate -PathType Leaf)) {
            return [System.IO.Path]::GetFullPath($candidate)
        }
    }
    return $null
}

function Get-NodeAgentReleaseInputHash {
    param(
        [Parameter(Mandatory = $true)][string]$RepoRoot,
        [Parameter(Mandatory = $true)][string]$GitSha,
        [Parameter(Mandatory = $true)][string[]]$GitPaths,
        [string[]]$ToolVersions = @(),
        [string[]]$EnvironmentValues = @()
    )

    $parts = New-Object System.Collections.Generic.List[string]
    foreach ($path in $GitPaths) {
        # Windows PowerShell 5.1 does not reliably propagate LASTEXITCODE from a
        # native command wrapped inside a parenthesized pipeline. Capture the
        # command and its exit code first, then select the bounded output.
        $previousPreference = $ErrorActionPreference
        $ErrorActionPreference = 'Continue'
        try {
            $identityLines = @(& git -C $RepoRoot rev-parse "$GitSha`:$path" 2>$null)
            $gitExit = $LASTEXITCODE
        } finally {
            $ErrorActionPreference = $previousPreference
        }
        $identity = @($identityLines | Select-Object -First 1)
        $identityText = if ($identity.Count -gt 0) { [string]$identity[0] } else { '' }
        if ($gitExit -ne 0 -or [string]::IsNullOrWhiteSpace($identityText)) {
            throw "Cannot calculate release cache input: $GitSha`:$path"
        }
        $parts.Add("git:$path=$($identityText.Trim())")
    }
    foreach ($version in $ToolVersions) {
        $parts.Add("tool=$(([string]$version).Trim())")
    }
    foreach ($value in @($EnvironmentValues | Sort-Object)) {
        $parts.Add("env=$value")
    }
    $bytes = [System.Text.Encoding]::UTF8.GetBytes(($parts -join "`n"))
    $algorithm = [System.Security.Cryptography.SHA256]::Create()
    try {
        return ([BitConverter]::ToString($algorithm.ComputeHash($bytes))).Replace('-', '').ToLowerInvariant()
    } finally {
        $algorithm.Dispose()
    }
}

function Get-NodeAgentReleaseEnvironmentValues {
    param([string]$Prefix)

    return @(Get-ChildItem Env: |
        Where-Object { $_.Name.StartsWith($Prefix, [StringComparison]::OrdinalIgnoreCase) } |
        Sort-Object Name |
        ForEach-Object { "$($_.Name)=$($_.Value)" })
}

function Invoke-NodeAgentCachedFileBuild {
    param(
        [Parameter(Mandatory = $true)][string]$Kind,
        [Parameter(Mandatory = $true)][string]$InputHash,
        [Parameter(Mandatory = $true)][string]$CacheRoot,
        [Parameter(Mandatory = $true)][string]$OutputPath,
        [Parameter(Mandatory = $true)][scriptblock]$Build
    )

    $entry = Join-Path $CacheRoot (Join-Path $Kind $InputHash)
    $cached = Join-Path $entry ([System.IO.Path]::GetFileName($OutputPath))
    if (Test-Path -LiteralPath $cached -PathType Leaf) {
        New-Item -ItemType Directory -Force -Path (Split-Path $OutputPath -Parent) | Out-Null
        Copy-Item -LiteralPath $cached -Destination $OutputPath -Force
        Write-Output "NODE_AGENT_BUILD_CACHE_KIND=$Kind"
        Write-Output 'NODE_AGENT_BUILD_CACHE_HIT=true'
        Write-Output "NODE_AGENT_BUILD_CACHE_KEY=$InputHash"
        return $true
    }

    & $Build
    if (-not (Test-Path -LiteralPath $OutputPath -PathType Leaf)) {
        throw "$Kind build did not produce the expected file: $OutputPath"
    }
    $temporary = "$entry.$PID.tmp"
    New-Item -ItemType Directory -Force -Path $temporary | Out-Null
    try {
        Copy-Item -LiteralPath $OutputPath -Destination (Join-Path $temporary ([System.IO.Path]::GetFileName($OutputPath))) -Force
        New-Item -ItemType Directory -Force -Path (Split-Path $entry -Parent) | Out-Null
        if (-not (Test-Path -LiteralPath $entry)) {
            Move-Item -LiteralPath $temporary -Destination $entry
        }
    } finally {
        Remove-Item -LiteralPath $temporary -Recurse -Force -ErrorAction SilentlyContinue
    }
    Write-Output "NODE_AGENT_BUILD_CACHE_KIND=$Kind"
    Write-Output 'NODE_AGENT_BUILD_CACHE_HIT=false'
    Write-Output "NODE_AGENT_BUILD_CACHE_KEY=$InputHash"
    return $false
}

function Invoke-NodeAgentCachedDirectoryBuild {
    param(
        [Parameter(Mandatory = $true)][string]$Kind,
        [Parameter(Mandatory = $true)][string]$InputHash,
        [Parameter(Mandatory = $true)][string]$CacheRoot,
        [Parameter(Mandatory = $true)][string]$OutputDirectory,
        [Parameter(Mandatory = $true)][string]$RequiredRelativePath,
        [Parameter(Mandatory = $true)][scriptblock]$Build
    )

    $entry = Join-Path $CacheRoot (Join-Path $Kind $InputHash)
    $cachedOutput = Join-Path $entry 'output'
    if (Test-Path -LiteralPath (Join-Path $cachedOutput $RequiredRelativePath) -PathType Leaf) {
        Remove-Item -LiteralPath $OutputDirectory -Recurse -Force -ErrorAction SilentlyContinue
        New-Item -ItemType Directory -Force -Path $OutputDirectory | Out-Null
        Copy-Item -Path (Join-Path $cachedOutput '*') -Destination $OutputDirectory -Recurse -Force
        Write-Output "NODE_AGENT_BUILD_CACHE_KIND=$Kind"
        Write-Output 'NODE_AGENT_BUILD_CACHE_HIT=true'
        Write-Output "NODE_AGENT_BUILD_CACHE_KEY=$InputHash"
        return $true
    }

    & $Build
    if (-not (Test-Path -LiteralPath (Join-Path $OutputDirectory $RequiredRelativePath) -PathType Leaf)) {
        throw "$Kind build did not produce the expected file: $RequiredRelativePath"
    }
    $temporary = "$entry.$PID.tmp"
    $temporaryOutput = Join-Path $temporary 'output'
    New-Item -ItemType Directory -Force -Path $temporaryOutput | Out-Null
    try {
        Copy-Item -Path (Join-Path $OutputDirectory '*') -Destination $temporaryOutput -Recurse -Force
        New-Item -ItemType Directory -Force -Path (Split-Path $entry -Parent) | Out-Null
        if (-not (Test-Path -LiteralPath $entry)) {
            Move-Item -LiteralPath $temporary -Destination $entry
        }
    } finally {
        Remove-Item -LiteralPath $temporary -Recurse -Force -ErrorAction SilentlyContinue
    }
    Write-Output "NODE_AGENT_BUILD_CACHE_KIND=$Kind"
    Write-Output 'NODE_AGENT_BUILD_CACHE_HIT=false'
    Write-Output "NODE_AGENT_BUILD_CACHE_KEY=$InputHash"
    return $false
}
