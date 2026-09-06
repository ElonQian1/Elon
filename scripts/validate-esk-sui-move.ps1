#requires -Version 7.0
[CmdletBinding()]
param(
    [Parameter(Mandatory = $true)]
    [string]$SuiPath,

    [Parameter(Mandatory = $true)]
    [string]$FrameworkArchivePath,

    [string]$EvidenceDirectory = "",

    [string]$MoveHome = ""
)

$ErrorActionPreference = "Stop"
$PSNativeCommandUseErrorActionPreference = $false

$script:ExplicitDependencyNote = '[NOTE] Dependencies on Sui, MoveStdlib, Bridge, DeepBook, and SuiSystem are automatically added, but this feature is disabled for your package because you have explicitly included dependencies on Sui. Consider removing these dependencies from `Move.toml`.'
$script:AnsiPattern = [regex]::new("`e\[[0-?]*[ -/]*[@-~]", [System.Text.RegularExpressions.RegexOptions]::CultureInvariant)
Import-Module (Join-Path $PSScriptRoot "esk-sui-toolchain-ci\validation-helpers.psm1") `
    -Force -ErrorAction Stop

function Stop-MoveValidation {
    param([string]$Message)
    throw "ESK Sui Move validation failed: $Message"
}

function Get-Sha256Label {
    param([string]$Path)
    return "sha256:$((Get-FileHash -LiteralPath $Path -Algorithm SHA256).Hash.ToLowerInvariant())"
}

function Write-Utf8Lf {
    param([string]$Path, [string[]]$Lines)
    $text = if ($Lines.Count -eq 0) { "" } else { "$($Lines -join "`n")`n" }
    [System.IO.File]::WriteAllText($Path, $text, [System.Text.UTF8Encoding]::new($false))
}

function Write-SafeCommandFailure {
    param([string]$OutputPath, [string]$Label, [int]$ExitCode, [string]$Reason)
    $safeLabel = $Label -replace '[^A-Za-z0-9_.-]', '_'
    Write-Utf8Lf -Path $OutputPath -Lines @(
        "COMMAND_STATUS=failed",
        "COMMAND_LABEL=$safeLabel",
        "COMMAND_EXIT_CODE=$ExitCode",
        "COMMAND_FAILURE_REASON=$Reason",
        "COMMAND_OUTPUT=not_persisted"
    )
}

function Test-ApprovedOutputLine {
    param([string]$Line, [string]$Profile)
    switch ($Profile) {
        "move-build" {
            return $Line -ceq $script:ExplicitDependencyNote -or
                $Line -cmatch '^(?:INCLUDING DEPENDENCY|BUILDING) [A-Za-z0-9_]+$'
        }
        "move-test" {
            return $Line -ceq $script:ExplicitDependencyNote -or
                $Line -cmatch '^(?:INCLUDING DEPENDENCY|BUILDING) [A-Za-z0-9_]+$' -or
                $Line -ceq 'Running Move unit tests' -or
                $Line -cmatch '^\[ PASS\s+\] [A-Za-z0-9_:]+$' -or
                $Line -cmatch '^Test result: OK\. Total tests: \d+; passed: \d+; failed: 0$'
        }
        "genesis-contract" {
            return $Line -cmatch '^ESK_SUI_[A-Z0-9_]+=(?:passed|local_verified|not_performed_by_this_script)$'
        }
        "allocation-contract" {
            return $Line -cin @(
                'PASS allocation policy schema and synthetic fixture',
                'PASS six-bucket conservation, role mapping, and source binding',
                'PASS one-shot allocation and immutable-beneficiary vesting boundaries',
                'PASS 13 Move scenarios are present and cannot fake expected failures',
                'PASS local verification is recorded without chain or real-holder claims'
            )
        }
        "artifact-verification" {
            return $Line -ceq 'ESK_SUI_ARTIFACTS=verified currency=3/3 participation=13/13'
        }
        default { return $false }
    }
}

function ConvertTo-ApprovedOutput {
    param([object[]]$RawLines, [string]$Profile)
    $rawText = ($RawLines | ForEach-Object { $_.ToString() }) -join "`n"
    $normalized = $script:AnsiPattern.Replace($rawText, '').Replace("`r`n", "`n").Replace("`r", "`n")
    $approved = [System.Collections.Generic.List[string]]::new()
    foreach ($candidate in $normalized.Split("`n")) {
        $line = $candidate.TrimEnd()
        if ([string]::IsNullOrWhiteSpace($line)) { continue }
        if ($line -cmatch '[^\x09\x20-\x7E]' -or -not (Test-ApprovedOutputLine -Line $line -Profile $Profile)) {
            return $null
        }
        $approved.Add($line)
    }
    if ($approved.Count -eq 0) { return $null }
    return $approved.ToArray()
}

function Invoke-CapturedCommand {
    param(
        [string]$FilePath,
        [string[]]$Arguments,
        [string]$OutputPath,
        [string]$Label,
        [string]$Profile
    )
    $rawLines = @(& $FilePath @Arguments 2>&1 | ForEach-Object { $_.ToString() })
    $exitCode = $LASTEXITCODE
    if ($exitCode -ne 0) {
        Write-SafeCommandFailure -OutputPath $OutputPath -Label $Label -ExitCode $exitCode -Reason "nonzero_exit"
        Stop-MoveValidation "$Label exited with a nonzero status; raw child output was not persisted"
    }
    $approved = ConvertTo-ApprovedOutput -RawLines $rawLines -Profile $Profile
    if ($null -eq $approved) {
        Write-SafeCommandFailure -OutputPath $OutputPath -Label $Label -ExitCode $exitCode -Reason "unexpected_output"
        Stop-MoveValidation "$Label emitted output outside the evidence allowlist; raw child output was not persisted"
    }
    Write-Utf8Lf -Path $OutputPath -Lines $approved
    return $approved
}

function Invoke-SuiMoveOperation {
    param(
        [string]$BinaryPath,
        [string]$Operation,
        [string]$PackagePath,
        [string]$ClientConfigPath,
        [string]$OutputPath,
        [string]$PackageId
    )
    $allowedOperations = @("build", "test")
    if ($Operation -cnotin $allowedOperations) {
        Stop-MoveValidation "unsupported Sui Move operation"
    }
    $arguments = @(
        "move", "--client.config", $ClientConfigPath, "--build-env", "testnet",
        "--path", $PackagePath, "--warnings-are-errors", $Operation
    )
    if ($Operation -ceq "test") { $arguments += @("--threads", "1") }
    Invoke-CapturedCommand -FilePath $BinaryPath -Arguments $arguments -OutputPath $OutputPath `
        -Label "sui_move_$($Operation)_$PackageId" -Profile "move-$Operation" | Out-Null
}

function Resolve-ContainedRelativePath {
    param(
        [string]$Parent,
        [string]$RelativePath,
        [string]$Label
    )
    if ([string]::IsNullOrWhiteSpace($RelativePath) -or
        [System.IO.Path]::IsPathRooted($RelativePath) -or $RelativePath.Contains('\')) {
        Stop-MoveValidation "$Label must be a forward-slash repository-relative path"
    }
    $segments = $RelativePath.Split('/')
    $invalidSegments = @($segments | Where-Object {
        $_ -ceq '.' -or $_ -ceq '..' -or $_ -cnotmatch '^[A-Za-z0-9._-]+$'
    })
    if ($segments.Count -eq 0 -or $invalidSegments.Count -ne 0) {
        Stop-MoveValidation "$Label contains an invalid path segment"
    }
    $parentFull = [System.IO.Path]::GetFullPath($Parent).TrimEnd(
        [System.IO.Path]::DirectorySeparatorChar,
        [System.IO.Path]::AltDirectorySeparatorChar)
    $resolved = [System.IO.Path]::GetFullPath((Join-Path $parentFull $RelativePath))
    $prefix = "$parentFull$([System.IO.Path]::DirectorySeparatorChar)"
    if (-not $resolved.StartsWith($prefix, [System.StringComparison]::OrdinalIgnoreCase)) {
        Stop-MoveValidation "$Label escaped its approved parent"
    }
    return $resolved
}

function Get-ManagedInputSnapshot {
    param([string]$RepoRoot, [object]$Contract)
    $paths = @(
        "scripts/esk-sui-toolchain-ci/toolchain-v1.json",
        "contracts/sui/esk-genesis-manifest-v1.fixture.json",
        "contracts/sui/esk-allocation-policy-v1.fixture.json"
    )
    foreach ($package in $Contract.packages) {
        $packageRoot = Resolve-ContainedRelativePath -Parent $RepoRoot `
            -RelativePath $package.path -Label "package path"
        foreach ($name in @("Move.toml", "Move.lock")) {
            $paths += (Join-Path $package.path $name).Replace('\', '/')
        }
        foreach ($directoryName in @("sources", "tests")) {
            $paths += Get-ChildItem -LiteralPath (Join-Path $packageRoot $directoryName) -File -Recurse |
                ForEach-Object { [System.IO.Path]::GetRelativePath($RepoRoot, $_.FullName).Replace('\', '/') }
        }
        $paths += $package.test_evidence.path
    }
    $snapshot = [ordered]@{}
    foreach ($relative in @($paths | Sort-Object -Unique)) {
        $absolute = Resolve-ContainedRelativePath -Parent $RepoRoot `
            -RelativePath $relative -Label "managed input path"
        if (-not (Test-Path -LiteralPath $absolute -PathType Leaf)) {
            Stop-MoveValidation "a managed repository input is missing"
        }
        $snapshot[$relative] = Get-Sha256Label -Path $absolute
    }
    return $snapshot
}

function Copy-PackageInputs {
    param(
        [string]$RepoRoot,
        [string]$BuildRoot,
        [object]$Package,
        [string]$FrameworkPackagePath,
        [string]$SuiSourceCommit
    )
    $source = Resolve-ContainedRelativePath -Parent $RepoRoot `
        -RelativePath $Package.path -Label "package source path"
    $target = Resolve-ContainedRelativePath -Parent $BuildRoot `
        -RelativePath $Package.path -Label "temporary package path"
    if (-not (Test-Path -LiteralPath $source -PathType Container)) {
        Stop-MoveValidation "package source directory is missing"
    }
    $sourceItem = Get-Item -LiteralPath $source -Force
    if (($sourceItem.Attributes -band [System.IO.FileAttributes]::ReparsePoint) -ne 0) {
        Stop-MoveValidation "package source directory cannot be a reparse point"
    }
    foreach ($item in Get-ChildItem -LiteralPath $source -Force -Recurse |
        Where-Object { $_.FullName -notmatch '[\\/]build(?:[\\/]|$)' }) {
        if (($item.Attributes -band [System.IO.FileAttributes]::ReparsePoint) -ne 0) {
            Stop-MoveValidation "package input contains a reparse point"
        }
    }
    New-Item -ItemType Directory -Path $target -Force | Out-Null
    foreach ($name in @("Move.toml", "Move.lock")) {
        Copy-Item -LiteralPath (Join-Path $source $name) -Destination (Join-Path $target $name)
    }
    Copy-Item -LiteralPath (Join-Path $source "sources") -Destination $target -Recurse
    Copy-Item -LiteralPath (Join-Path $source "tests") -Destination $target -Recurse

    $manifestPath = Join-Path $target "Move.toml"
    $manifest = [System.IO.File]::ReadAllText($manifestPath)
    $fixedGitDependency = "Sui = { git = `"https://github.com/MystenLabs/sui.git`", subdir = `"crates/sui-framework/packages/sui-framework`", rev = `"$SuiSourceCommit`" }"
    $first = $manifest.IndexOf($fixedGitDependency, [System.StringComparison]::Ordinal)
    if ($first -lt 0 -or $manifest.IndexOf($fixedGitDependency, $first + 1, [System.StringComparison]::Ordinal) -ge 0) {
        Stop-MoveValidation "package manifest does not contain exactly one fixed Sui dependency"
    }
    $localFramework = $FrameworkPackagePath.Replace('\', '/')
    $manifest = $manifest.Replace($fixedGitDependency, "Sui = { local = `"$localFramework`" }")
    [System.IO.File]::WriteAllText($manifestPath, $manifest, [System.Text.UTF8Encoding]::new($false))
    Remove-Item -LiteralPath (Join-Path $target "Move.lock") -Force
    return $target
}

function Get-PackageBuildInputSnapshot {
    param([string]$PackagePath, [bool]$IncludeLock)
    $paths = [System.Collections.Generic.List[string]]::new()
    $paths.Add("Move.toml")
    if ($IncludeLock) {
        if (-not (Test-Path -LiteralPath (Join-Path $PackagePath "Move.lock") -PathType Leaf)) {
            Stop-MoveValidation "temporary package lockfile is missing"
        }
        $paths.Add("Move.lock")
    }
    foreach ($directoryName in @("sources", "tests")) {
        $directory = Join-Path $PackagePath $directoryName
        foreach ($item in Get-ChildItem -LiteralPath $directory -Force -Recurse) {
            if (($item.Attributes -band [System.IO.FileAttributes]::ReparsePoint) -ne 0) {
                Stop-MoveValidation "temporary package input contains a reparse point"
            }
            if (-not $item.PSIsContainer) {
                $paths.Add([System.IO.Path]::GetRelativePath($PackagePath, $item.FullName).Replace('\', '/'))
            }
        }
    }
    $snapshot = [ordered]@{}
    foreach ($relative in @($paths.ToArray() | Sort-Object -Unique)) {
        $snapshot[$relative] = Get-Sha256Label -Path (Join-Path $PackagePath $relative)
    }
    return $snapshot
}

function Assert-SnapshotEqual {
    param(
        [System.Collections.IDictionary]$Expected,
        [System.Collections.IDictionary]$Actual,
        [string]$Label
    )
    if ($Expected.Count -ne $Actual.Count) { Stop-MoveValidation "$Label input file set changed" }
    foreach ($entry in $Expected.GetEnumerator()) {
        if (-not $Actual.Contains($entry.Key) -or $Actual[$entry.Key] -cne $entry.Value) {
            Stop-MoveValidation "$Label input bytes changed"
        }
    }
}

function Copy-ProductionArtifacts {
    param([string]$BuiltPackagePath, [string]$ArtifactRoot, [object]$Package)
    $source = Join-Path $BuiltPackagePath "build\$($Package.id)\bytecode_modules"
    if (-not (Test-Path -LiteralPath $source -PathType Container)) {
        Stop-MoveValidation "production bytecode directory is missing"
    }
    $actual = @(Get-ChildItem -LiteralPath $source -Filter "*.mv" -File |
        ForEach-Object { $_.BaseName } | Sort-Object)
    $expected = @($Package.build_evidence.modules | Sort-Object)
    if (($actual -join "`n") -cne ($expected -join "`n")) {
        Stop-MoveValidation "production module set changed"
    }
    $target = Join-Path $ArtifactRoot "$($Package.path)\build\$($Package.id)\bytecode_modules"
    New-Item -ItemType Directory -Path $target -Force | Out-Null
    foreach ($module in $expected) {
        Copy-Item -LiteralPath (Join-Path $source "$module.mv") -Destination (Join-Path $target "$module.mv")
    }
}

function Assert-SnapshotUnchanged {
    param([string]$RepoRoot, [System.Collections.IDictionary]$Before)
    foreach ($entry in $Before.GetEnumerator()) {
        $path = Join-Path $RepoRoot $entry.Key
        if (-not (Test-Path -LiteralPath $path -PathType Leaf) -or
            (Get-Sha256Label -Path $path) -cne $entry.Value) {
            Stop-MoveValidation "validation changed a managed repository input"
        }
    }
}

function Assert-IsolatedMoveHome {
    param([string]$MoveHomePath)
    foreach ($item in Get-ChildItem -LiteralPath $MoveHomePath -Force -Recurse) {
        if (($item.Attributes -band [System.IO.FileAttributes]::ReparsePoint) -ne 0) {
            Stop-MoveValidation "isolated MoveHome contains a reparse point"
        }
        $relative = [System.IO.Path]::GetRelativePath($MoveHomePath, $item.FullName).Replace('\', '/')
        if ($item.PSIsContainer) {
            if ($relative -cne "git") {
                Stop-MoveValidation "isolated MoveHome contains an unexpected directory"
            }
        } elseif (-not $relative.StartsWith("git/", [System.StringComparison]::Ordinal) -or
            [System.IO.Path]::GetFileName($relative) -cnotmatch '^\.[A-Za-z0-9_.-]+\.lock$' -or
            $item.Length -ne 0) {
            Stop-MoveValidation "isolated MoveHome contains data other than inert lock files"
        }
    }
}

$repoRoot = [System.IO.Directory]::GetParent($PSScriptRoot).FullName
$binaryPath = [System.IO.Path]::GetFullPath($SuiPath)
$frameworkArchive = [System.IO.Path]::GetFullPath($FrameworkArchivePath)
$contractPath = Join-Path $repoRoot "scripts/esk-sui-toolchain-ci/toolchain-v1.json"
$contract = Get-Content -LiteralPath $contractPath -Raw | ConvertFrom-Json
if ($contract.schema -cne "yilong.esk.sui.reproducible_toolchain.v1") {
    Stop-MoveValidation "toolchain contract schema is unsupported"
}
if ($contract.sui_source_commit -cnotmatch '^[0-9a-f]{40}$' -or
    $contract.framework.archive_root -cne "sui-$($contract.sui_source_commit)" -or
    $contract.framework.archive_sha256 -cnotmatch '^sha256:[0-9a-f]{64}$' -or
    $contract.framework.tracked_content_digest -cnotmatch '^sha256:[0-9a-f]{64}$') {
    Stop-MoveValidation "fixed framework contract is invalid"
}
$expectedTrackedRoots = @(
    "crates/sui-framework/packages/move-stdlib",
    "crates/sui-framework/packages/sui-framework"
)
if ((@($contract.framework.tracked_roots) -join "`n") -cne ($expectedTrackedRoots -join "`n")) {
    Stop-MoveValidation "fixed framework root list is invalid"
}
if ((@($contract.packages.id) -join "`n") -cne "esk_currency`nyilong_participation" -or
    @($contract.packages | Where-Object { $_.id -cnotmatch '^[a-z0-9_]+$' }).Count -ne 0) {
    Stop-MoveValidation "fixed package list is invalid"
}
$expectedPackagePaths = @(
    "contracts/sui/esk_currency",
    "contracts/sui/yilong_participation"
)
$expectedEvidencePaths = @(
    "contracts/sui/yilong_participation/evidence/esk-currency-regression-output-v1.txt",
    "contracts/sui/yilong_participation/evidence/move-test-output-v1.txt"
)
for ($index = 0; $index -lt $contract.packages.Count; $index++) {
    $package = $contract.packages[$index]
    if ($package.path -cne $expectedPackagePaths[$index] -or
        $package.test_evidence.path -cne $expectedEvidencePaths[$index]) {
        Stop-MoveValidation "fixed package or evidence path is invalid"
    }
    Resolve-ContainedRelativePath -Parent $repoRoot -RelativePath $package.path `
        -Label "package path" | Out-Null
    Resolve-ContainedRelativePath -Parent $repoRoot -RelativePath $package.test_evidence.path `
        -Label "test evidence path" | Out-Null
}
Assert-FixedFile -Path $binaryPath -ExpectedSize ([long]$contract.cli.binary_size) `
    -ExpectedDigest $contract.cli.binary_sha256 -Label "Sui CLI binary"
Assert-FixedFile -Path $frameworkArchive -ExpectedSize ([long]$contract.framework.archive_size) `
    -ExpectedDigest $contract.framework.archive_sha256 -Label "Sui framework source archive"
$trustedTar = Resolve-TrustedSystemTar
$nodeCommand = Get-Command node -CommandType Application -ErrorAction SilentlyContinue | Select-Object -First 1
if ($null -eq $nodeCommand) { Stop-MoveValidation "Node.js is required for deterministic contract checks" }
$nodePath = $nodeCommand.Source

$temporaryBase = [System.IO.Path]::GetFullPath([System.IO.Path]::GetTempPath())
$workRoot = Join-Path $temporaryBase "esk-sui-move-$([guid]::NewGuid().ToString('N'))"
$preserveEvidence = -not [string]::IsNullOrWhiteSpace($EvidenceDirectory)
$evidenceRoot = if ($preserveEvidence) {
    [System.IO.Path]::GetFullPath($EvidenceDirectory)
} else {
    Join-Path $workRoot "evidence"
}
$oldConfigDirectory = [System.Environment]::GetEnvironmentVariable("SUI_CONFIG_DIR", "Process")
$oldMoveHome = [System.Environment]::GetEnvironmentVariable("MOVE_HOME", "Process")
$before = Get-ManagedInputSnapshot -RepoRoot $repoRoot -Contract $contract
$binaryBefore = Get-Sha256Label -Path $binaryPath
$archiveBefore = Get-Sha256Label -Path $frameworkArchive

try {
    New-Item -ItemType Directory -Path $workRoot -Force | Out-Null
    if (Test-Path -LiteralPath $evidenceRoot) {
        if (-not (Test-Path -LiteralPath $evidenceRoot -PathType Container) -or
            @(Get-ChildItem -LiteralPath $evidenceRoot -Force).Count -ne 0) {
            Stop-MoveValidation "evidence directory must be absent or empty"
        }
    } else {
        New-Item -ItemType Directory -Path $evidenceRoot -Force | Out-Null
    }
    $isolatedConfig = Join-Path $workRoot "sui-config"
    New-Item -ItemType Directory -Path $isolatedConfig -Force | Out-Null
    $isolatedMoveHome = if ([string]::IsNullOrWhiteSpace($MoveHome)) {
        Join-Path $workRoot "move-home"
    } else {
        [System.IO.Path]::GetFullPath($MoveHome)
    }
    $moveVolumeRoot = [System.IO.Path]::GetPathRoot($isolatedMoveHome).TrimEnd(
        [System.IO.Path]::DirectorySeparatorChar)
    if ($isolatedMoveHome.TrimEnd([System.IO.Path]::DirectorySeparatorChar) -eq $moveVolumeRoot) {
        Stop-MoveValidation "MoveHome cannot be a volume root"
    }
    if (Test-Path -LiteralPath $isolatedMoveHome) {
        if (-not (Test-Path -LiteralPath $isolatedMoveHome -PathType Container) -or
            @(Get-ChildItem -LiteralPath $isolatedMoveHome -Force).Count -ne 0) {
            Stop-MoveValidation "MoveHome must be absent or empty"
        }
    } else {
        New-Item -ItemType Directory -Path $isolatedMoveHome -Force | Out-Null
    }
    $moveHomeItem = Get-Item -LiteralPath $isolatedMoveHome -Force
    if (($moveHomeItem.Attributes -band [System.IO.FileAttributes]::ReparsePoint) -ne 0) {
        Stop-MoveValidation "MoveHome cannot be a reparse point"
    }

    $emptyKeyFile = Join-Path $isolatedConfig "sui.keystore"
    $isolatedClientConfig = Join-Path $isolatedConfig "client.yaml"
    [System.IO.File]::WriteAllText($emptyKeyFile, "[]`n", [System.Text.UTF8Encoding]::new($false))
    $safeKeyPath = $emptyKeyFile.Replace("'", "''")
    $clientYaml = "keystore:`n  File: '$safeKeyPath'`nexternal_keys: null`nenvs: []`nactive_env: null`nactive_address: null`n"
    [System.IO.File]::WriteAllText($isolatedClientConfig, $clientYaml, [System.Text.UTF8Encoding]::new($false))
    $emptyKeyHash = Get-Sha256Label -Path $emptyKeyFile
    $clientConfigHash = Get-Sha256Label -Path $isolatedClientConfig
    [System.Environment]::SetEnvironmentVariable("SUI_CONFIG_DIR", $isolatedConfig, "Process")
    [System.Environment]::SetEnvironmentVariable("MOVE_HOME", $isolatedMoveHome, "Process")

    $versionOutput = @(& $binaryPath --version 2>&1 | ForEach-Object { $_.ToString() })
    if ($LASTEXITCODE -ne 0 -or ($versionOutput -join "`n").Trim() -cne $contract.cli.version) {
        Stop-MoveValidation "Sui CLI version differs from the fixed contract"
    }

    $frameworkExtractRoot = Join-Path $workRoot "framework"
    $frameworkPackagePath = Expand-FixedFrameworkArchive -ArchivePath $frameworkArchive `
        -ExtractRoot $frameworkExtractRoot -TarPath $trustedTar -Contract $contract

    $packagePaths = @{}
    foreach ($package in $contract.packages) {
        $packagePaths[$package.id] = Copy-PackageInputs -RepoRoot $repoRoot -BuildRoot $workRoot `
            -Package $package -FrameworkPackagePath $frameworkPackagePath `
            -SuiSourceCommit $contract.sui_source_commit
    }

    Invoke-CapturedCommand -FilePath $nodePath `
        -Arguments @((Join-Path $repoRoot "scripts/test-esk-sui-genesis-foundation.js")) `
        -OutputPath (Join-Path $evidenceRoot "test-esk-sui-genesis-foundation.log") `
        -Label "genesis_contract" -Profile "genesis-contract" | Out-Null
    Invoke-CapturedCommand -FilePath $nodePath `
        -Arguments @((Join-Path $repoRoot "scripts/test-esk-sui-allocation-vesting.js")) `
        -OutputPath (Join-Path $evidenceRoot "test-esk-sui-allocation-vesting.log") `
        -Label "allocation_contract" -Profile "allocation-contract" | Out-Null

    $outputs = @{}
    $artifactRoot = Join-Path $workRoot "production-artifacts"
    foreach ($package in $contract.packages) {
        $packagePath = $packagePaths[$package.id]
        $beforeBuild = Get-PackageBuildInputSnapshot -PackagePath $packagePath -IncludeLock $false
        Invoke-SuiMoveOperation -BinaryPath $binaryPath -Operation "build" -PackagePath $packagePath `
            -ClientConfigPath $isolatedClientConfig `
            -OutputPath (Join-Path $evidenceRoot "$($package.id)-build.log") -PackageId $package.id
        $afterBuild = Get-PackageBuildInputSnapshot -PackagePath $packagePath -IncludeLock $false
        Assert-SnapshotEqual -Expected $beforeBuild -Actual $afterBuild -Label "build"
        Copy-ProductionArtifacts -BuiltPackagePath $packagePath -ArtifactRoot $artifactRoot -Package $package
        $beforeTest = Get-PackageBuildInputSnapshot -PackagePath $packagePath -IncludeLock $true
        $testOutput = Join-Path $evidenceRoot "$($package.id)-test.log"
        Invoke-SuiMoveOperation -BinaryPath $binaryPath -Operation "test" -PackagePath $packagePath `
            -ClientConfigPath $isolatedClientConfig -OutputPath $testOutput -PackageId $package.id
        $afterTest = Get-PackageBuildInputSnapshot -PackagePath $packagePath -IncludeLock $true
        Assert-SnapshotEqual -Expected $beforeTest -Actual $afterTest -Label "test"
        $outputs[$package.id] = $testOutput
    }

    Assert-IsolatedMoveHome -MoveHomePath $isolatedMoveHome
    Assert-FrameworkTree -ExtractRoot $frameworkExtractRoot -Contract $contract | Out-Null

    $artifactLog = Join-Path $evidenceRoot "artifact-verification.log"
    Invoke-CapturedCommand -FilePath $nodePath -Arguments @(
        (Join-Path $repoRoot "scripts\esk-sui-toolchain-ci\verify-artifacts.js"),
        "--repo", $repoRoot,
        "--build-root", $artifactRoot,
        "--currency-output", $outputs.esk_currency,
        "--participation-output", $outputs.yilong_participation
    ) -OutputPath $artifactLog -Label "artifact_verification" -Profile "artifact-verification" | Out-Null

    Assert-SnapshotUnchanged -RepoRoot $repoRoot -Before $before
    if ((Get-Sha256Label -Path $binaryPath) -cne $binaryBefore -or
        (Get-Sha256Label -Path $frameworkArchive) -cne $archiveBefore) {
        Stop-MoveValidation "fixed toolchain bytes changed during validation"
    }
    if ((Get-Sha256Label -Path $emptyKeyFile) -cne $emptyKeyHash -or
        (Get-Sha256Label -Path $isolatedClientConfig) -cne $clientConfigHash -or
        (Get-Content -LiteralPath $emptyKeyFile -Raw).Trim() -cne "[]") {
        Stop-MoveValidation "isolated no-account configuration changed during validation"
    }
    $statusLines = @(
        "ESK_SUI_TOOLCHAIN_STATUS=verified release=$($contract.cli.release)",
        "ESK_SUI_MOVE_STATUS=verified currency=3/3 participation=13/13",
        "ESK_SUI_FRAMEWORK_SOURCE=verified files=$($contract.framework.tracked_file_count)",
        "ESK_SUI_RPC_STATE=not_configured_or_queried",
        "ESK_SUI_PUBLICATION_STATE=not_performed"
    )
    Write-Utf8Lf -Path (Join-Path $evidenceRoot "validation-status.log") -Lines $statusLines
    $statusLines | Write-Output
} finally {
    [System.Environment]::SetEnvironmentVariable("SUI_CONFIG_DIR", $oldConfigDirectory, "Process")
    [System.Environment]::SetEnvironmentVariable("MOVE_HOME", $oldMoveHome, "Process")
    if (Test-Path -LiteralPath $workRoot) {
        $workFull = [System.IO.Path]::GetFullPath($workRoot)
        if (-not $workFull.StartsWith($temporaryBase, [System.StringComparison]::OrdinalIgnoreCase)) {
            Stop-MoveValidation "temporary work root escaped the system temporary directory"
        }
        Remove-Item -LiteralPath $workFull -Recurse -Force
    }
}
