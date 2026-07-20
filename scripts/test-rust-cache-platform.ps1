$ErrorActionPreference = "Stop"

$RepoRoot = (git -C $PSScriptRoot rev-parse --show-toplevel).Trim()
$ModulesRoot = Join-Path $PSScriptRoot "rust-cache"
Import-Module "$ModulesRoot\RustCache.Paths.psm1" -Force -DisableNameChecking
Import-Module "$ModulesRoot\RustCache.Policy.psm1" -Force -DisableNameChecking
Import-Module "$ModulesRoot\RustCache.Inventory.psm1" -Force -DisableNameChecking
Import-Module "$ModulesRoot\RustCache.Legacy.psm1" -Force -DisableNameChecking
Import-Module "$ModulesRoot\RustCache.Install.psm1" -Force -DisableNameChecking
Import-Module "$ModulesRoot\RustCache.Runtime.psm1" -Force -DisableNameChecking
Import-Module "$ModulesRoot\RustCache.Sccache.psm1" -Force -DisableNameChecking

$script:Assertions = 0
function Assert-True {
    param([bool]$Condition, [string]$Message)
    $script:Assertions++
    if (-not $Condition) { throw "ASSERT FAILED: $Message" }
}

function Assert-Equal {
    param($Expected, $Actual, [string]$Message)
    $script:Assertions++
    if ($Expected -ne $Actual) { throw "ASSERT FAILED: $Message expected='$Expected' actual='$Actual'" }
}

$TempRoot = Join-Path $env:TEMP ("elon-rust-cache-test-{0}" -f [Guid]::NewGuid().ToString("N"))
$null = New-Item -ItemType Directory -Force -Path $TempRoot
$TempRoot = (Get-Item -LiteralPath $TempRoot -Force).FullName
$ProjectRoot = Join-Path $TempRoot "registered-project"
$UnknownRoot = Join-Path $TempRoot "unknown-project"
$CacheRoot = Join-Path $TempRoot "cache"
New-Item -ItemType Directory -Force -Path $ProjectRoot, $UnknownRoot, $CacheRoot | Out-Null

try {
    @'
{
  "schema_version": 1,
  "project_id": "test-project",
  "default_domain": "dev-host"
}
'@ | Set-Content -LiteralPath (Join-Path $ProjectRoot "rust-cache.project.json") -Encoding UTF8
    "[package]`nname='test-project'`nversion='0.1.0'`nedition='2021'`n" | Set-Content -LiteralPath (Join-Path $ProjectRoot "Cargo.toml") -Encoding UTF8

    $context = Resolve-RustCacheInvocation -ProjectRoot $ProjectRoot -CacheRoot $CacheRoot -CargoArgs @("check") -ToolchainEpoch "rustc-test"
    Assert-True $context.registered "project manifest should register the project"
    Assert-Equal "test-project" $context.project_id "registered project id"
    Assert-True ($context.build_dir -like "*\build\rustc-test\test-project\dev-host\*") "registered build path should be compatibility-scoped"
    Assert-Equal (Join-Path $context.project_root "target") $context.target_dir "final artifacts should remain workspace-local by default"

    $unknown = Resolve-RustCacheInvocation -ProjectRoot $UnknownRoot -CacheRoot $CacheRoot -CargoArgs @("check") -ToolchainEpoch "rustc-test"
    Assert-True (-not $unknown.registered) "unknown project should not enter the registered pool"
    Assert-True ($unknown.build_dir -like "*\quarantine\*") "unknown project should route to quarantine"

    $nestedContext = Resolve-RustCacheInvocation -ProjectRoot $ProjectRoot -CacheRoot $CacheRoot -CargoArgs @("check", "--manifest-path", "server\Cargo.toml") -ToolchainEpoch "rustc-test"
    $baseDirs = Get-RustCacheSccacheBaseDirs -ProjectRoot $nestedContext.project_root -WorkspaceRoot $nestedContext.workspace_root
    $expectedBaseDirs = "{0}{1}{2}" -f $nestedContext.workspace_root, [System.IO.Path]::PathSeparator, $nestedContext.project_root
    Assert-Equal $expectedBaseDirs $baseDirs "sccache should strip checkout-specific workspace and project roots"
    $sccacheConfig = Get-RustCacheSccacheConfigContent -BaseDirs @($ProjectRoot, $UnknownRoot)
    Assert-True ($sccacheConfig -match '^# Generated' -and $sccacheConfig -match 'basedirs = \[') "sccache config should be generated from registered roots"
    $normalizedProjectRoot = ConvertTo-RustCacheSccacheConfigPath -Path $ProjectRoot
    Assert-True ($sccacheConfig -match [regex]::Escape($normalizedProjectRoot)) "sccache config should normalize project paths"
    $pendingSccache = Sync-RustCacheSccacheConfiguration -CacheRoot $CacheRoot -AdditionalBaseDirs @($ProjectRoot, $UnknownRoot)
    Assert-True $pendingSccache.restart_pending "a changed sccache config should remain pending until a server reload is requested"
    Assert-True (Test-Path -LiteralPath $pendingSccache.state_path) "sccache pending state should be durable"

    $release = Resolve-RustCacheInvocation -ProjectRoot $ProjectRoot -CacheRoot $CacheRoot -CargoArgs @("build", "--release") -ToolchainEpoch "rustc-test"
    Assert-True $release.release "release invocation should be detected"

    $EnvironmentCapture = Join-Path $TempRoot "cargo-environment.txt"
    $FakeCargo = Join-Path $TempRoot "fake-cargo.cmd"
    @"
@echo off
set CARGO_BUILD_BUILD_DIR>%RUST_CACHE_TEST_ENV%
set CARGO_TARGET_DIR>>%RUST_CACHE_TEST_ENV%
set CARGO_INCREMENTAL>>%RUST_CACHE_TEST_ENV%
echo CARGO_CWD=%CD%>>%RUST_CACHE_TEST_ENV%
exit /b 0
"@ | Set-Content -LiteralPath $FakeCargo -Encoding ASCII
    $env:RUST_CACHE_TEST_ENV = $EnvironmentCapture
    Invoke-RustCacheCargo -ProjectRoot $ProjectRoot -CacheRoot $CacheRoot -Domain "release-host" -DisableSccache -CargoCommand $FakeCargo -ToolchainEpoch "rustc-test" -CargoArgs @("build", "--release")
    Assert-Equal 0 $LASTEXITCODE "fake Cargo invocation should succeed"
    $captured = Get-Content -Raw -LiteralPath $EnvironmentCapture
    Assert-True ($captured -match 'CARGO_BUILD_BUILD_DIR=.*test-project\\release-host') "Cargo should receive the managed build-dir"
    Assert-True ($captured -match 'CARGO_TARGET_DIR=.*registered-project\\target') "Cargo should receive the local target-dir"
    Assert-True ($captured -match 'CARGO_INCREMENTAL=0') "release should force incremental off"
    Assert-True ($captured -match "CARGO_CWD=$([regex]::Escape($release.project_root))") "Cargo should execute from the declared project root"
    Assert-True ([string]::IsNullOrWhiteSpace($env:CARGO_BUILD_BUILD_DIR)) "Cargo environment should be restored after execution"

    $staleBuildDir = Join-Path $CacheRoot "build\rustc-test\test-project\stale\0123456789abcdef"
    $staleLock = Join-Path $staleBuildDir ".rust-cache.lockdir"
    New-Item -ItemType Directory -Force -Path $staleLock | Out-Null
    '{"pid":2147483000,"started_utc":"2000-01-01T00:00:00Z"}' | Set-Content -LiteralPath (Join-Path $staleLock "owner.json") -Encoding UTF8
    $acquired = Enter-RustCacheLock -CacheRoot $CacheRoot -BuildDir $staleBuildDir -WorkspaceRoot $ProjectRoot -TimeoutSeconds 2
    $owner = Get-RustCacheLockOwner -LockPath $acquired
    Assert-Equal $PID ([int]$owner.pid) "stale lock should be replaced by the current owner"
    Exit-RustCacheLock -LockPath $acquired

    $oldPartition = Join-Path $CacheRoot "build\rustc-old\test-project\dev-host\aaaaaaaaaaaaaaaa"
    New-Item -ItemType Directory -Force -Path $oldPartition | Out-Null
    '{"last_used_utc":"2000-01-01T00:00:00Z"}' | Set-Content -LiteralPath (Join-Path $oldPartition ".last-used.json") -Encoding UTF8
    Set-Content -LiteralPath (Join-Path $oldPartition "artifact.bin") -Value "old"
    $gc = Invoke-RustCacheGc -CacheRoot $CacheRoot -RepoRoot $ProjectRoot -ForceAged
    $oldAction = $gc.actions | Where-Object { $_.path -eq $oldPartition } | Select-Object -First 1
    Assert-Equal "would-delete" $oldAction.action "dry-run GC should select an old toolchain partition"
    Assert-True (Test-Path -LiteralPath $oldPartition) "dry-run GC must not delete files"
    Assert-Equal 0 (Get-RustCacheDirectorySize -Path (Join-Path $CacheRoot "missing-partition")) "a concurrently removed partition should have advisory size zero"
    $deletionPartition = Join-Path $CacheRoot "build\rustc-old\test-project\delete-host\bbbbbbbbbbbbbbbb"
    $longSegment = "incremental-" + ("x" * 120)
    $longLeaf = Join-Path (Join-Path $deletionPartition $longSegment) $longSegment
    $longFsLeaf = if ($env:OS -eq "Windows_NT") { "\\?\$longLeaf" } else { $longLeaf }
    $longFile = Join-Path $longLeaf "artifact.bin"
    $longFsFile = if ($env:OS -eq "Windows_NT") { "\\?\$longFile" } else { $longFile }
    [System.IO.Directory]::CreateDirectory($longFsLeaf) | Out-Null
    [System.IO.File]::WriteAllBytes($longFsFile, [byte[]](1, 2, 3, 4))
    Assert-True ((Get-RustCacheDirectorySize -Path $deletionPartition) -ge 0) "long-path size enumeration should remain advisory"
    $inventoryModule = Get-Module RustCache.Inventory | Select-Object -First 1
    Assert-True ($null -ne $inventoryModule) "inventory module should be loaded for partition removal regression"
    & $inventoryModule { param($PartitionPath) Remove-RustCachePartition -Path $PartitionPath } $deletionPartition
    Assert-True (-not (Test-Path -LiteralPath $deletionPartition)) "managed long-path partition removal should tolerate PowerShell traversal failures"
    $quarantineLeaf = Join-Path $CacheRoot "quarantine\ab\cdef0123456789"
    New-Item -ItemType Directory -Force -Path $quarantineLeaf | Out-Null
    Set-Content -LiteralPath (Join-Path $quarantineLeaf "artifact.bin") -Value "quarantine"
    $quarantinePartition = Get-RustCachePartitions -CacheRoot $CacheRoot | Where-Object { $_.path -eq $quarantineLeaf } | Select-Object -First 1
    Assert-Equal "abcdef0123456789" $quarantinePartition.workspace_hash "Cargo-sharded quarantine should be a leaf partition"
    $outsideRejected = $false
    try { Assert-RustCacheManagedPath -CacheRoot $CacheRoot -CandidatePath $ProjectRoot } catch { $outsideRejected = $true }
    Assert-True $outsideRejected "managed-path guard should reject external directories"

    $CargoConfig = Join-Path $TempRoot ".cargo\config.toml"
    New-Item -ItemType Directory -Force -Path (Split-Path $CargoConfig -Parent) | Out-Null
    @'
[build]
target-dir = "D:/old/shared-target"
jobs = 12
rustflags = ["-C", "target-cpu=native"]

[net]
retry = 3
'@ | Set-Content -LiteralPath $CargoConfig -Encoding UTF8
    $proposal = Set-RustCacheParentCargoConfig -CargoConfigPath $CargoConfig -IncludeConfigPath (Join-Path $CacheRoot "config\cargo-cache.toml")
    Assert-True ($proposal.content -match '^include = ') "Cargo activation should add an include"
    Assert-True ($proposal.content -notmatch 'target-dir\s*=') "Cargo activation should remove universal target-dir"
    Assert-True ($proposal.content -notmatch 'rustflags\s*=') "Cargo activation should remove machine-wide rustflags"
    Assert-True ($proposal.content -match 'jobs = 12') "Cargo activation should preserve unrelated build settings"
    Assert-True ($proposal.content -match '\[net\]') "Cargo activation should preserve other sections"

    $install = Install-RustCachePlatform -SourceScriptsRoot $PSScriptRoot -CacheRoot (Join-Path $TempRoot "installed") -RepoRoot $ProjectRoot
    Assert-True (Test-Path -LiteralPath $install.entry_path) "installer should copy the entry script"
    Assert-True (Test-Path -LiteralPath $install.cargo_include_path) "installer should generate Cargo include config"
    Assert-True (Test-Path -LiteralPath $install.sccache_config_path) "installer should generate managed sccache config"
    $include = Get-Content -Raw -LiteralPath $install.cargo_include_path
    Assert-True ($include -match 'build-dir = .*quarantine/.+workspace-path-hash') "fallback Cargo route should use workspace quarantine"
    if (-not [string]::IsNullOrWhiteSpace([string]$install.sccache_path)) {
        Assert-True (Test-Path -LiteralPath $install.sccache_wrapper_path) "installer should generate the managed sccache wrapper when sccache is available"
        Assert-True ($include -match [regex]::Escape($install.sccache_wrapper_path.Replace('\', '/'))) "Cargo include should select the managed sccache wrapper"
        $wrapperHeader = [System.IO.File]::ReadAllBytes($install.sccache_wrapper_path)
        Assert-True ($wrapperHeader.Length -gt 2 -and $wrapperHeader[0] -eq 0x4d -and $wrapperHeader[1] -eq 0x5a) "sccache wrapper should be a native Windows executable"
    } else {
        Assert-True ([string]::IsNullOrWhiteSpace([string]$install.sccache_wrapper_path)) "missing sccache should leave the managed wrapper explicitly disabled"
        Assert-True ($include -notmatch '^\s*rustc-wrapper\s*=') "Cargo include must not reference a missing sccache wrapper"
    }
    $wrapperSource = Get-Content -Raw -LiteralPath (Join-Path $ModulesRoot "native\rustc_sccache_wrapper.rs")
    Assert-True ($wrapperSource -match 'env_remove\("CARGO_BUILD_BUILD_DIR"\)' -and $wrapperSource -match 'env_remove\("CARGO_TARGET_DIR"\)') "sccache wrapper should remove Cargo-only routing variables"
    Assert-True ($wrapperSource -match '\.args\(args\)' -and $wrapperSource -match 'SCCACHE_CONF') "sccache wrapper should pin managed configuration and forward compiler arguments"
    $bashAdapter = Get-Content -Raw -LiteralPath (Join-Path $PSScriptRoot "cargo-dev.sh")
    Assert-True ($bashAdapter -match 'powershell\.exe' -and $bashAdapter -match 'ps_script=.*cargo-dev\.ps1') "Git Bash should delegate to the PowerShell cache platform on Windows"
    Assert-True ($bashAdapter -notmatch 'ELON_DEV_CARGO_TARGET_DIR') "Git Bash must not route back to the retired shared target"
    $installedEntry = $install.entry_path
    $legacyTarget = Join-Path $TempRoot "legacy-target"
    New-Item -ItemType Directory -Force -Path $legacyTarget | Out-Null
    Set-Content -LiteralPath (Join-Path $legacyTarget "artifact.bin") -Value "legacy"
    & $installedEntry register-legacy -ProjectRoot $ProjectRoot -CacheRoot $install.cache_root -LegacyPath $legacyTarget -Label "test-legacy" -Retired
    Assert-Equal 0 $LASTEXITCODE "installed entry should execute register-legacy"
    $installedPolicy = Get-Content -Raw -LiteralPath (Join-Path $install.cache_root "config\policy.json") | ConvertFrom-Json
    Assert-Equal "test-legacy" @($installedPolicy.legacy_caches)[0].label "installed legacy registration"
    $purgePlan = & $installedEntry purge-legacy -ProjectRoot $ProjectRoot -CacheRoot $install.cache_root -LegacyPath $legacyTarget
    Assert-Equal "would-delete" $purgePlan.action "legacy purge should default to dry-run"
    Assert-True (Test-Path -LiteralPath $legacyTarget) "legacy purge dry-run must preserve files"
    $unregisteredRejected = $false
    try { Invoke-RustCacheLegacyPurge -CacheRoot $CacheRoot -RepoRoot $ProjectRoot -LegacyPath (Join-Path $TempRoot "unregistered-target") } catch { $unregisteredRejected = $true }
    Assert-True $unregisteredRejected "legacy purge should reject unregistered paths"

    Write-Host "PASS: Rust cache platform tests ($script:Assertions assertions)." -ForegroundColor Green
} finally {
    Remove-Item Env:RUST_CACHE_TEST_ENV -ErrorAction SilentlyContinue
    Remove-Item -LiteralPath $TempRoot -Recurse -Force -ErrorAction SilentlyContinue
}
