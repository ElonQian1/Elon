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
Import-Module "$ModulesRoot\RustCache.Paths.psm1" -Force -DisableNameChecking
Import-Module "$ModulesRoot\RustCache.Registry.psm1" -Force -DisableNameChecking
Import-Module "$ModulesRoot\RustCache.Policy.psm1" -Force -DisableNameChecking
Import-Module "$ModulesRoot\RustCache.Paths.psm1" -Force -DisableNameChecking

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
$previousTaskWorktreeBase = $env:ELON_AI_TASK_WORKTREE_BASE
$env:ELON_AI_TASK_WORKTREE_BASE = Join-Path $TempRoot "wt"

try {
    @'
{
  "schema_version": 1,
  "project_id": "test-project",
  "default_domain": "dev-host",
  "allowed_domains": ["dev-host", "agent-validation", "node-agent-release", "release-host"],
  "unknown_domain_fallback": "agent-validation",
  "shared_partition_domains": {
    "validation-heavy": "agent-validation",
    "validation-light-0": "agent-validation",
    "validation-light-1": "agent-validation",
    "node-agent-windows": "node-agent-release"
  }
}
'@ | Set-Content -LiteralPath (Join-Path $ProjectRoot "rust-cache.project.json") -Encoding UTF8
    "[package]`nname='test-project'`nversion='0.1.0'`nedition='2021'`n" | Set-Content -LiteralPath (Join-Path $ProjectRoot "Cargo.toml") -Encoding UTF8

    $context = Resolve-RustCacheInvocation -ProjectRoot $ProjectRoot -CacheRoot $CacheRoot -CargoArgs @("check") -ToolchainEpoch "rustc-test"
    Assert-True $context.registered "project manifest should register the project"
    Assert-Equal "test-project" $context.project_id "registered project id"
    Assert-Equal "workspace" $context.cache_scope "default registered builds should remain workspace-scoped"
    Assert-Equal $context.workspace_hash $context.cache_partition "workspace scope should use the workspace hash"
    Assert-True ($context.build_dir -like "*\build\rustc-test\test-project\dev-host\*") "registered build path should be compatibility-scoped"
    Assert-Equal (Join-Path $context.project_root "target") $context.target_dir "final artifacts should remain workspace-local by default"
    $unknownDomain = Resolve-RustCacheInvocation -ProjectRoot $ProjectRoot -CacheRoot $CacheRoot -Domain "one-off-task-name" -CargoArgs @("check") -ToolchainEpoch "rustc-test"
    Assert-Equal "agent-validation" $unknownDomain.domain "managed build paths should use the canonical fallback domain"
    Assert-True $unknownDomain.domain_fallback "domain fallback should be observable"
    $validationFromDev = Resolve-RustCacheInvocation -ProjectRoot $ProjectRoot -CacheRoot $CacheRoot -Domain "dev-host" -SharedBuildPartition "validation-heavy" -CargoArgs @("check") -ToolchainEpoch "rustc-test"
    $validationFromAgent = Resolve-RustCacheInvocation -ProjectRoot $ProjectRoot -CacheRoot $CacheRoot -Domain "agent-validation" -SharedBuildPartition "validation-heavy" -CargoArgs @("check") -ToolchainEpoch "rustc-test"
    Assert-Equal "agent-validation" $validationFromDev.domain "reserved shared partitions should override an otherwise allowed caller domain"
    Assert-True $validationFromDev.domain_canonicalized_by_shared_partition "shared partition domain canonicalization should be observable"
    Assert-Equal $validationFromAgent.build_dir $validationFromDev.build_dir "same reserved shared partition must converge across caller domains"
    $unreservedShared = Resolve-RustCacheInvocation -ProjectRoot $ProjectRoot -CacheRoot $CacheRoot -Domain "dev-host" -SharedBuildPartition "generic-shared" -CargoArgs @("check") -ToolchainEpoch "rustc-test"
    Assert-Equal "dev-host" $unreservedShared.domain "unreserved shared partitions should retain an allowed caller domain"
    Assert-True (-not $unreservedShared.domain_canonicalized_by_shared_partition) "unreserved shared partitions should not report canonicalization"

    $InvalidMappingRoot = Join-Path $TempRoot "invalid-shared-mapping-project"
    New-Item -ItemType Directory -Force -Path $InvalidMappingRoot | Out-Null
    @'
{
  "schema_version": 1,
  "project_id": "invalid-shared-mapping",
  "default_domain": "dev-host",
  "allowed_domains": ["dev-host"],
  "shared_partition_domains": {"validation-heavy": "unlisted-domain"}
}
'@ | Set-Content -LiteralPath (Join-Path $InvalidMappingRoot "rust-cache.project.json") -Encoding UTF8
    $invalidMappingRejected = $false
    try { Get-RustCacheProjectManifest -ProjectRoot $InvalidMappingRoot | Out-Null } catch { $invalidMappingRejected = $_.Exception.Message -match "must be listed in allowed_domains" }
    Assert-True $invalidMappingRejected "shared partition mappings must fail closed when their domain is not allowlisted"

    $unknown = Resolve-RustCacheInvocation -ProjectRoot $UnknownRoot -CacheRoot $CacheRoot -CargoArgs @("check") -ToolchainEpoch "rustc-test"
    Assert-True (-not $unknown.registered) "unknown project should not enter the registered pool"
    Assert-True ($unknown.build_dir -like "*\quarantine\*") "unknown project should route to quarantine"
    Assert-Equal "quarantine" $unknown.cache_scope "unknown projects should report quarantine scope"
    $unknownSharedRejected = $false
    try { Resolve-RustCacheInvocation -ProjectRoot $UnknownRoot -CacheRoot $CacheRoot -CargoArgs @("check") -ToolchainEpoch "rustc-test" -SharedBuildPartition "unsafe" | Out-Null } catch { $unknownSharedRejected = $true }
    Assert-True $unknownSharedRejected "unknown projects must not opt into managed shared partitions"

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
    $sharedRelease = Resolve-RustCacheInvocation -ProjectRoot $ProjectRoot -CacheRoot $CacheRoot -Domain "node-agent-release" -CargoArgs @("build", "--release") -ToolchainEpoch "rustc-test" -SharedBuildPartition "node-agent-windows"
    Assert-True ($sharedRelease.build_dir -like "*\build\rustc-test\test-project\node-agent-release\shared-node-agent-windows") "explicit release partition should be stable across isolated worktrees"
    Assert-Equal "shared" $sharedRelease.cache_scope "named build partitions should report shared scope"
    Assert-Equal "shared-node-agent-windows" $sharedRelease.cache_partition "shared partition identity should be observable"
    $SecondProjectRoot = Join-Path $TempRoot "registered-project-second-worktree"
    New-Item -ItemType Directory -Force -Path $SecondProjectRoot | Out-Null
    Copy-Item -LiteralPath (Join-Path $ProjectRoot "rust-cache.project.json") -Destination $SecondProjectRoot
    $secondSharedRelease = Resolve-RustCacheInvocation -ProjectRoot $SecondProjectRoot -CacheRoot $CacheRoot -Domain "node-agent-release" -CargoArgs @("build", "--release") -ToolchainEpoch "rustc-test" -SharedBuildPartition "node-agent-windows"
    Assert-Equal $sharedRelease.build_dir $secondSharedRelease.build_dir "same named partition should converge across different worktree roots"
    Assert-True ($sharedRelease.workspace_hash -ne $secondSharedRelease.workspace_hash) "shared partition regression needs distinct workspace identities"
    $invalidSharedRejected = $false
    try { Resolve-RustCacheInvocation -ProjectRoot $ProjectRoot -CacheRoot $CacheRoot -Domain "node-agent-release" -SharedBuildPartition "Worktree 1" -CargoArgs @("check") -ToolchainEpoch "rustc-test" | Out-Null } catch { $invalidSharedRejected = $_.Exception.Message -match "stable slug" }
    Assert-True $invalidSharedRejected "shared partition names should reject path- or session-like normalization collisions"
    $sharedNoLockRejected = $false
    try { Invoke-RustCacheCargo -ProjectRoot $ProjectRoot -CacheRoot $CacheRoot -Domain "node-agent-release" -SharedBuildPartition "node-agent-windows" -NoLock -DisableSccache -CargoCommand "missing-cargo" -ToolchainEpoch "rustc-test" -CargoArgs @("check") } catch { $sharedNoLockRejected = $_.Exception.Message -match "require.*lock" }
    Assert-True $sharedNoLockRejected "shared partitions must reject lock bypass"

    $registryUpgradeRoot = Join-Path $TempRoot "registry-upgrade"
    New-Item -ItemType Directory -Force -Path (Join-Path $registryUpgradeRoot "state") | Out-Null
    $legacyRegistry = @{ schema_version = 1; workspaces = @(@{ project_id = "test-project"; workspace_hash = "legacy-hash"; domain = "dev-host"; build_dir = $sharedRelease.build_dir }) }
    $legacyRegistry | ConvertTo-Json -Depth 4 | Set-Content -LiteralPath (Join-Path $registryUpgradeRoot "state\registry.json") -Encoding UTF8
    Update-RustCacheRegistry -CacheRoot $registryUpgradeRoot -ProjectId "test-project" -ProjectRoot $ProjectRoot -WorkspaceRoot $ProjectRoot -WorkspaceHash "legacy-hash" -CacheScope "shared" -CachePartition "shared-dev" -Domain "dev-host" -ToolchainEpoch "rustc-test" -BuildDir $sharedRelease.build_dir -TargetDir $sharedRelease.target_dir -Registered $true
    $upgradedRegistry = Read-RustCacheRegistry -CacheRoot $registryUpgradeRoot
    Assert-Equal "shared" $upgradedRegistry.workspaces[0].cache_scope "registry updates should add scope fields to existing v1 rows"
    Assert-Equal "shared-dev" $upgradedRegistry.workspaces[0].cache_partition "registry updates should add partition fields to existing v1 rows"
    Update-RustCacheRegistry -CacheRoot $registryUpgradeRoot -ProjectId "test-project" -ProjectRoot $ProjectRoot -WorkspaceRoot $ProjectRoot -WorkspaceHash "legacy-hash" -CacheScope "shared" -CachePartition "shared-other" -Domain "dev-host" -ToolchainEpoch "rustc-test" -BuildDir (Join-Path $CacheRoot "shared-other") -TargetDir $sharedRelease.target_dir -Registered $true
    $partitionedRegistry = Read-RustCacheRegistry -CacheRoot $registryUpgradeRoot
    Assert-Equal 2 @($partitionedRegistry.workspaces).Count "registry should preserve multiple named partitions for one workspace and domain"
    $missingReadiness = Get-RustCacheSccacheReadiness -Disabled
    Assert-Equal "unavailable" $missingReadiness.status "disabled sccache must be explicit"
    Assert-Equal "disabled_by_caller" $missingReadiness.reason "sccache degradation reason"
    $migrationAdvice = Get-RustCacheMigrationAdvice -CacheRoot $CacheRoot -LowWatermarkPercent 101 -ManagedAlternativeRoot (Join-Path $TempRoot "managed-alternative")
    Assert-True $migrationAdvice.migration_recommended "low-watermark advice should be structured"
    Assert-True (-not $migrationAdvice.destructive_actions_taken) "migration advice must not move or delete caches"
    $legacyPolicyRoot = Join-Path $TempRoot "legacy-policy-cache"
    New-Item -ItemType Directory -Force -Path (Join-Path $legacyPolicyRoot "config") | Out-Null
    $legacyPolicy = Get-DefaultRustCachePolicy
    $legacyPolicy.PSObject.Properties.Remove("orphan_task_grace_hours")
    $legacyPolicy | ConvertTo-Json -Depth 8 | Set-Content -LiteralPath (Join-Path $legacyPolicyRoot "config\policy.json") -Encoding UTF8
    $upgradedPolicy = Get-RustCachePolicy -CacheRoot $legacyPolicyRoot
    Assert-Equal 24 ([int]$upgradedPolicy.orphan_task_grace_hours) "old policy files should receive an in-memory orphan grace default"
    $persistedLegacyPolicy = Get-Content -Raw -LiteralPath (Join-Path $legacyPolicyRoot "config\policy.json") | ConvertFrom-Json
    Assert-True ($null -eq $persistedLegacyPolicy.PSObject.Properties["orphan_task_grace_hours"]) "reading an old policy should not rewrite it implicitly"
    $oldRustRoot = $env:ELON_RUST_CACHE_ROOT; $oldNodeRoot = $env:ELON_NODE_DATA_ROOT
    $oldSharedRoot = $env:RUST_SHARED_BUILD_ROOT; $oldAppData = $env:APPDATA
    try {
        $env:ELON_RUST_CACHE_ROOT = $null
        $env:ELON_NODE_DATA_ROOT = Join-Path $TempRoot "node-data"
        $env:RUST_SHARED_BUILD_ROOT = Join-Path $TempRoot "older-shared"
        $nodePreferred = Resolve-RustCacheRoot -RepoRoot $ProjectRoot
        Assert-True ($nodePreferred -like "$($env:ELON_NODE_DATA_ROOT)*") "node unified data root should precede older shared-root convention"

        $env:ELON_NODE_DATA_ROOT = $null
        $env:APPDATA = Join-Path $TempRoot "appdata"
        $persistedNodeRoot = Join-Path $TempRoot "持久节点缓存"
        $nodeConfigRoot = Join-Path $env:APPDATA "elon-node-agent"
        New-Item -ItemType Directory -Force -Path $nodeConfigRoot, $persistedNodeRoot | Out-Null
        '{}' | Set-Content -LiteralPath (Join-Path $persistedNodeRoot ".elon-node-data-root.json") -Encoding UTF8
        $nodeConfigPath = Join-Path $nodeConfigRoot "node.json"
        $nodeConfigJson = @{ node_data_root = $persistedNodeRoot } | ConvertTo-Json
        [System.IO.File]::WriteAllText($nodeConfigPath, $nodeConfigJson, (New-Object System.Text.UTF8Encoding($false)))
        $nodeConfigBytes = [System.IO.File]::ReadAllBytes($nodeConfigPath)
        Assert-True (-not ($nodeConfigBytes.Length -ge 3 -and $nodeConfigBytes[0] -eq 0xEF -and $nodeConfigBytes[1] -eq 0xBB -and $nodeConfigBytes[2] -eq 0xBF)) "persisted node config fixture should be UTF-8 without BOM"
        $sharedPreferred = Resolve-RustCacheRoot -RepoRoot $ProjectRoot
        Assert-Equal (Join-Path $env:RUST_SHARED_BUILD_ROOT "rust-cache-v2") $sharedPreferred "explicit shared build root should precede persisted node data root"

        $env:RUST_SHARED_BUILD_ROOT = $null
        $persistedRoot = Resolve-RustCacheRoot -RepoRoot $ProjectRoot
        Assert-Equal (Join-Path $persistedNodeRoot "cache\rust-cache-v2") $persistedRoot "persisted owned node data root should supply the Rust cache root"

        $persistedFallback = if (Test-Path -LiteralPath "D:\rust\shared") {
            "D:\rust\shared\rust-cache-v2"
        } else {
            Join-Path $env:LOCALAPPDATA "Elon\rust-cache-v2"
        }

        '{broken' | Set-Content -LiteralPath $nodeConfigPath -Encoding UTF8
        $damagedFallback = Resolve-RustCacheRoot -RepoRoot $ProjectRoot
        Assert-Equal $persistedFallback $damagedFallback "damaged persisted config should use the existing fallback chain"

        [System.IO.File]::WriteAllBytes($nodeConfigPath, [byte[]](0x7B, 0x22, 0x78, 0x22, 0x3A, 0x22, 0xFF, 0x22, 0x7D))
        $invalidUtf8Fallback = Resolve-RustCacheRoot -RepoRoot $ProjectRoot
        Assert-Equal $persistedFallback $invalidUtf8Fallback "invalid UTF-8 persisted config should use the existing fallback chain"

        $unownedNodeRoot = Join-Path $TempRoot "unowned-node-data"
        New-Item -ItemType Directory -Force -Path $unownedNodeRoot | Out-Null
        @{ node_data_root = $unownedNodeRoot } | ConvertTo-Json | Set-Content -LiteralPath $nodeConfigPath -Encoding UTF8
        $unownedFallback = Resolve-RustCacheRoot -RepoRoot $ProjectRoot
        Assert-Equal $persistedFallback $unownedFallback "persisted root without ownership marker should use the existing fallback chain"

        $missingNodeRoot = Join-Path $TempRoot "missing-node-data"
        @{ node_data_root = $missingNodeRoot } | ConvertTo-Json | Set-Content -LiteralPath $nodeConfigPath -Encoding UTF8
        $missingFallback = Resolve-RustCacheRoot -RepoRoot $ProjectRoot
        Assert-Equal $persistedFallback $missingFallback "absolute missing persisted root should use the existing fallback chain"

        @{ node_data_root = "relative-node-data" } | ConvertTo-Json | Set-Content -LiteralPath $nodeConfigPath -Encoding UTF8
        $relativeFallback = Resolve-RustCacheRoot -RepoRoot $ProjectRoot
        Assert-Equal $persistedFallback $relativeFallback "relative persisted root should use the existing fallback chain"
    } finally {
        $env:ELON_RUST_CACHE_ROOT = $oldRustRoot; $env:ELON_NODE_DATA_ROOT = $oldNodeRoot
        $env:RUST_SHARED_BUILD_ROOT = $oldSharedRoot; $env:APPDATA = $oldAppData
    }

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
    Invoke-RustCacheCargo -ProjectRoot $ProjectRoot -CacheRoot $CacheRoot -Domain "agent-validation" -SharedBuildPartition "validation-light-0" -DisableSccache -CargoCommand $FakeCargo -ToolchainEpoch "rustc-test" -CargoArgs @("check")
    $agentCaptured = Get-Content -Raw -LiteralPath $EnvironmentCapture
    Assert-True ($agentCaptured -match 'CARGO_BUILD_BUILD_DIR=.*agent-validation\\shared-validation-light-0') "scheduled validation should receive its stable shared build partition"
    Assert-True ($agentCaptured -match 'CARGO_INCREMENTAL=0') "agent validation should disable incremental"

    $canonicalSharedPartition = Join-Path $CacheRoot "build\rustc-test\test-project\agent-validation\shared-validation-heavy"
    $retiredSharedAlias = Join-Path $CacheRoot "build\rustc-test\test-project\dev-host\shared-validation-heavy"
    $unmigratedSharedAlias = Join-Path $CacheRoot "build\rustc-test\test-project\dev-host\shared-validation-light-1"
    New-Item -ItemType Directory -Force -Path $canonicalSharedPartition, $retiredSharedAlias, $unmigratedSharedAlias | Out-Null
    @{ workspace_root = $ProjectRoot; cache_scope = "shared"; cache_partition = "shared-validation-heavy"; domain = "agent-validation"; last_used_utc = [DateTime]::UtcNow.ToString("o") } |
        ConvertTo-Json | Set-Content -LiteralPath (Join-Path $canonicalSharedPartition ".last-used.json") -Encoding UTF8
    @{ workspace_root = $ProjectRoot; cache_scope = "shared"; cache_partition = "shared-validation-heavy"; domain = "dev-host"; last_used_utc = [DateTime]::UtcNow.ToString("o") } |
        ConvertTo-Json | Set-Content -LiteralPath (Join-Path $retiredSharedAlias ".last-used.json") -Encoding UTF8
    @{ workspace_root = $ProjectRoot; cache_scope = "shared"; cache_partition = "shared-validation-light-1"; domain = "dev-host"; last_used_utc = [DateTime]::UtcNow.ToString("o") } |
        ConvertTo-Json | Set-Content -LiteralPath (Join-Path $unmigratedSharedAlias ".last-used.json") -Encoding UTF8
    $sharedAliasPolicyPath = Get-RustCachePolicyPath -CacheRoot $CacheRoot
    $sharedAliasPolicy = Get-Content -Raw -LiteralPath $sharedAliasPolicyPath | ConvertFrom-Json
    $sharedAliasPolicy.warning_free_percent = 1
    $sharedAliasPolicy.recovery_free_percent = 2
    $sharedAliasPolicy.critical_free_percent = 0
    $sharedAliasPolicy | ConvertTo-Json -Depth 8 | Set-Content -LiteralPath $sharedAliasPolicyPath -Encoding UTF8
    $sharedAliasStatus = Get-RustCacheStatus -CacheRoot $CacheRoot -RepoRoot $ProjectRoot
    Assert-Equal 2 $sharedAliasStatus.retired_shared_alias_count "status should proactively expose duplicate shared aliases"
    $sharedAliasGc = Invoke-RustCacheGc -CacheRoot $CacheRoot -RepoRoot $ProjectRoot -SharedAliasesOnly
    $retiredSharedAction = $sharedAliasGc.actions | Where-Object { $_.path -eq $retiredSharedAlias } | Select-Object -First 1
    Assert-Equal "would-delete" $retiredSharedAction.action "GC should select a duplicate shared alias when the canonical partition is ready"
    Assert-Equal "retired-shared-alias" $retiredSharedAction.reason "duplicate shared aliases should have an explicit reason"
    $unmigratedSharedAction = $sharedAliasGc.actions | Where-Object { $_.path -eq $unmigratedSharedAlias } | Select-Object -First 1
    Assert-Equal "preserve" $unmigratedSharedAction.action "GC should preserve an alias until its canonical partition is ready"
    Assert-Equal "retired-shared-alias-canonical-missing" $unmigratedSharedAction.reason "missing canonical partitions should have a stable preservation reason"
    $sharedAliasApply = Invoke-RustCacheGc -CacheRoot $CacheRoot -RepoRoot $ProjectRoot -SharedAliasesOnly -Apply
    Assert-True (-not (Test-Path -LiteralPath $retiredSharedAlias)) "managed GC should remove the reviewed duplicate alias"
    Assert-True (Test-Path -LiteralPath $canonicalSharedPartition) "managed GC must preserve the canonical shared partition"
    Assert-True (Test-Path -LiteralPath $unmigratedSharedAlias) "managed GC must preserve an alias whose canonical partition is not ready"
    $conflictingSharedAliasFiltersRejected = $false
    try { Invoke-RustCacheGc -CacheRoot $CacheRoot -RepoRoot $ProjectRoot -SharedAliasesOnly -WorkspaceOnly | Out-Null } catch { $conflictingSharedAliasFiltersRejected = $_.Exception.Message -match "cannot be combined" }
    Assert-True $conflictingSharedAliasFiltersRejected "shared alias cleanup must reject broader workspace recovery filters"

    $staleBuildDir = Join-Path $CacheRoot "build\rustc-test\test-project\stale\0123456789abcdef"
    $staleLock = Join-Path $staleBuildDir ".rust-cache.lockdir"
    New-Item -ItemType Directory -Force -Path $staleLock | Out-Null
    '{"pid":2147483000,"started_utc":"2000-01-01T00:00:00Z"}' | Set-Content -LiteralPath (Join-Path $staleLock "owner.json") -Encoding UTF8
    $staleState = Get-RustCacheLockState -LockPath $staleLock
    Assert-Equal "stale" $staleState.state "a lock owned by a missing PID should be classified as stale"
    Assert-True (-not $staleState.active) "a stale lock must not remain an active GC blocker"
    $acquired = Enter-RustCacheLock -CacheRoot $CacheRoot -BuildDir $staleBuildDir -WorkspaceRoot $ProjectRoot -TimeoutSeconds 2
    $owner = Get-RustCacheLockOwner -LockPath $acquired
    Assert-Equal $PID ([int]$owner.pid) "stale lock should be replaced by the current owner"
    $activeState = Get-RustCacheLockState -LockPath $acquired
    Assert-Equal "active" $activeState.state "a current process lock should be classified as active"
    Assert-True $activeState.active "a current process lock must block GC"
    Exit-RustCacheLock -LockPath $acquired
    $absentState = Get-RustCacheLockState -LockPath $acquired
    Assert-Equal "absent" $absentState.state "a released lock should be classified as absent"

    $invalidLock = Join-Path $staleBuildDir ".rust-cache.lockdir"
    New-Item -ItemType Directory -Force -Path $invalidLock | Out-Null
    Set-Content -LiteralPath (Join-Path $invalidLock "owner.json") -Value "{invalid" -Encoding UTF8
    (Get-Item -LiteralPath $invalidLock -Force).LastWriteTimeUtc = [DateTime]::UtcNow.AddMinutes(-5)
    $invalidState = Get-RustCacheLockState -LockPath $invalidLock
    Assert-Equal "invalid" $invalidState.state "an old malformed lock should be classified explicitly"
    Assert-True (-not $invalidState.active) "an old malformed lock may be atomically replaced"
    Remove-Item -LiteralPath $invalidLock -Recurse -Force

    $oldPartition = Join-Path $CacheRoot "build\rustc-old\test-project\dev-host\aaaaaaaaaaaaaaaa"
    New-Item -ItemType Directory -Force -Path $oldPartition | Out-Null
    '{"last_used_utc":"2000-01-01T00:00:00Z"}' | Set-Content -LiteralPath (Join-Path $oldPartition ".last-used.json") -Encoding UTF8
    Set-Content -LiteralPath (Join-Path $oldPartition "artifact.bin") -Value "old"
    $gc = Invoke-RustCacheGc -CacheRoot $CacheRoot -RepoRoot $ProjectRoot -ForceAged
    $oldAction = $gc.actions | Where-Object { $_.path -eq $oldPartition } | Select-Object -First 1
    Assert-Equal "would-delete" $oldAction.action "dry-run GC should select an old toolchain partition"
    Assert-True (Test-Path -LiteralPath $oldPartition) "dry-run GC must not delete files"
    $retiredDomainPartition = Join-Path $CacheRoot "build\rustc-test\test-project\one-off-task\cccccccccccccccc"
    New-Item -ItemType Directory -Force -Path $retiredDomainPartition | Out-Null
    '{"last_used_utc":"2099-01-01T00:00:00Z"}' | Set-Content -LiteralPath (Join-Path $retiredDomainPartition ".last-used.json") -Encoding UTF8
    $domainGc = Invoke-RustCacheGc -CacheRoot $CacheRoot -RepoRoot $ProjectRoot
    $retiredDomainAction = $domainGc.actions | Where-Object { $_.path -eq $retiredDomainPartition } | Select-Object -First 1
    Assert-Equal "would-delete" $retiredDomainAction.action "GC should retire a current-epoch domain outside the project allowlist"
    Assert-Equal "retired-domain" $retiredDomainAction.reason "retired domains should have an explicit reason"

    $currentEpoch = Get-RustCacheToolchainEpoch
    $missingTaskRoot = Join-Path $env:ELON_AI_TASK_WORKTREE_BASE "12345-deadbeef"
    $orphanWorkspace = Join-Path $missingTaskRoot "server"
    $oldMarkerTime = [DateTime]::UtcNow.AddHours(-48).ToString("o")
    $orphanPartition = Join-Path $CacheRoot "build\$currentEpoch\test-project\dev-host\1111111111111111"
    New-Item -ItemType Directory -Force -Path $orphanPartition | Out-Null
    @{ workspace_root = $orphanWorkspace; cache_scope = "workspace"; cache_partition = "1111111111111111"; last_used_utc = $oldMarkerTime } |
        ConvertTo-Json | Set-Content -LiteralPath (Join-Path $orphanPartition ".last-used.json") -Encoding UTF8
    Set-Content -LiteralPath (Join-Path $orphanPartition "artifact.bin") -Value "orphan"

    $recentOrphanPartition = Join-Path $CacheRoot "build\$currentEpoch\test-project\dev-host\2222222222222222"
    New-Item -ItemType Directory -Force -Path $recentOrphanPartition | Out-Null
    @{ workspace_root = $orphanWorkspace; cache_scope = "workspace"; cache_partition = "2222222222222222"; last_used_utc = [DateTime]::UtcNow.ToString("o") } |
        ConvertTo-Json | Set-Content -LiteralPath (Join-Path $recentOrphanPartition ".last-used.json") -Encoding UTF8

    $sharedTaskPartition = Join-Path $CacheRoot "build\$currentEpoch\test-project\dev-host\shared-finish"
    New-Item -ItemType Directory -Force -Path $sharedTaskPartition | Out-Null
    @{ workspace_root = $orphanWorkspace; cache_scope = "shared"; cache_partition = "shared-finish"; last_used_utc = $oldMarkerTime } |
        ConvertTo-Json | Set-Content -LiteralPath (Join-Path $sharedTaskPartition ".last-used.json") -Encoding UTF8

    $unmanagedMissingPartition = Join-Path $CacheRoot "build\$currentEpoch\test-project\dev-host\3333333333333333"
    New-Item -ItemType Directory -Force -Path $unmanagedMissingPartition | Out-Null
    @{ workspace_root = (Join-Path $TempRoot "ordinary-missing\server"); cache_scope = "workspace"; cache_partition = "3333333333333333"; last_used_utc = $oldMarkerTime } |
        ConvertTo-Json | Set-Content -LiteralPath (Join-Path $unmanagedMissingPartition ".last-used.json") -Encoding UTF8

    $existingWorkspace = Join-Path $TempRoot "existing-workspace\server"
    New-Item -ItemType Directory -Force -Path $existingWorkspace | Out-Null
    $existingWorkspacePartition = Join-Path $CacheRoot "build\$currentEpoch\test-project\dev-host\8888888888888888"
    New-Item -ItemType Directory -Force -Path $existingWorkspacePartition | Out-Null
    @{ workspace_root = $existingWorkspace; cache_scope = "workspace"; cache_partition = "8888888888888888"; last_used_utc = $oldMarkerTime } |
        ConvertTo-Json | Set-Content -LiteralPath (Join-Path $existingWorkspacePartition ".last-used.json") -Encoding UTF8

    $invalidMarkerPartition = Join-Path $CacheRoot "build\$currentEpoch\test-project\dev-host\4444444444444444"
    New-Item -ItemType Directory -Force -Path $invalidMarkerPartition | Out-Null
    '{invalid' | Set-Content -LiteralPath (Join-Path $invalidMarkerPartition ".last-used.json") -Encoding UTF8

    $lockedOrphanPartition = Join-Path $CacheRoot "build\$currentEpoch\test-project\dev-host\5555555555555555"
    New-Item -ItemType Directory -Force -Path (Join-Path $lockedOrphanPartition ".rust-cache.lockdir") | Out-Null
    @{ workspace_root = $orphanWorkspace; cache_scope = "workspace"; cache_partition = "5555555555555555"; last_used_utc = $oldMarkerTime } |
        ConvertTo-Json | Set-Content -LiteralPath (Join-Path $lockedOrphanPartition ".last-used.json") -Encoding UTF8

    $staleMissingPartition = Join-Path $CacheRoot "build\$currentEpoch\test-project\dev-host\7777777777777777"
    $staleMissingLock = Join-Path $staleMissingPartition ".rust-cache.lockdir"
    New-Item -ItemType Directory -Force -Path $staleMissingLock | Out-Null
    @{ pid = 2147483000; started_utc = "2000-01-01T00:00:00Z" } |
        ConvertTo-Json | Set-Content -LiteralPath (Join-Path $staleMissingLock "owner.json") -Encoding UTF8
    @{ workspace_root = (Join-Path $TempRoot "recoverable-missing\server"); cache_scope = "workspace"; cache_partition = "7777777777777777"; last_used_utc = $oldMarkerTime } |
        ConvertTo-Json | Set-Content -LiteralPath (Join-Path $staleMissingPartition ".last-used.json") -Encoding UTF8

    $nonLowDiskPolicyPath = Get-RustCachePolicyPath -CacheRoot $CacheRoot
    $nonLowDiskPolicy = Get-Content -Raw -LiteralPath $nonLowDiskPolicyPath | ConvertFrom-Json
    $nonLowDiskPolicy.warning_free_percent = 1
    $nonLowDiskPolicy.recovery_free_percent = 2
    $nonLowDiskPolicy.critical_free_percent = 0
    $nonLowDiskPolicy | ConvertTo-Json -Depth 8 | Set-Content -LiteralPath $nonLowDiskPolicyPath -Encoding UTF8
    $orphanDryRun = Invoke-RustCacheGc -CacheRoot $CacheRoot -RepoRoot $ProjectRoot
    Assert-True (-not $orphanDryRun.low_disk) "orphan regression must prove selection independently of the disk watermark"
    $orphanAction = $orphanDryRun.actions | Where-Object { $_.path -eq $orphanPartition } | Select-Object -First 1
    Assert-Equal "would-delete" $orphanAction.action "ordinary GC should select an orphaned task partition without ForceAged"
    Assert-Equal "orphaned-task-worktree" $orphanAction.reason "orphaned task partitions should have a stable reason"
    Assert-True (Test-Path -LiteralPath $orphanPartition) "orphan dry-run must preserve files"
    Assert-True (-not ($orphanDryRun.actions | Where-Object { $_.path -in @($recentOrphanPartition, $sharedTaskPartition, $unmanagedMissingPartition, $invalidMarkerPartition) -and $_.orphaned_task_worktree })) "recent, shared, unrelated, and invalid partitions must not enter orphan cleanup"
    $lockedAction = $orphanDryRun.actions | Where-Object { $_.path -eq $lockedOrphanPartition } | Select-Object -First 1
    Assert-Equal "lock-present" $lockedAction.reason "locked orphaned task partitions must be preserved"
    $orphanApply = Invoke-RustCacheGc -CacheRoot $CacheRoot -RepoRoot $ProjectRoot -Apply
    Assert-True (-not (Test-Path -LiteralPath $orphanPartition)) "orphan apply should delete the eligible task partition"
    Assert-True (Test-Path -LiteralPath $lockedOrphanPartition) "orphan apply must preserve a locked partition"
    $preservedOrphanFixtures = @($recentOrphanPartition, $sharedTaskPartition, $unmanagedMissingPartition, $invalidMarkerPartition)
    Assert-True (@($preservedOrphanFixtures | Where-Object { Test-Path -LiteralPath $_ }).Count -eq 4) "orphan apply must preserve recent, shared, unrelated, and invalid partitions"

    $missingRecovery = Invoke-RustCacheGc -CacheRoot $CacheRoot -RepoRoot $ProjectRoot -RecoverMissingWorkspaces -WorkspaceOnly
    $missingAction = $missingRecovery.actions | Where-Object { $_.path -eq $unmanagedMissingPartition } | Select-Object -First 1
    Assert-Equal "would-delete" $missingAction.action "explicit recovery should select an aged valid workspace whose path is missing"
    Assert-Equal "missing-workspace-recovery" $missingAction.reason "missing workspace recovery should have a stable reason"
    $staleMissingAction = $missingRecovery.actions | Where-Object { $_.path -eq $staleMissingPartition } | Select-Object -First 1
    Assert-Equal "stale" $staleMissingAction.lock_state "a stale lock should be audited without blocking recovery"
    Assert-Equal "would-delete" $staleMissingAction.action "a stale lock should be replaced only by the managed deletion lock"
    $sharedFiltered = $missingRecovery.actions | Where-Object { $_.path -eq $sharedTaskPartition } | Select-Object -First 1
    Assert-Equal "workspace-scope-filter" $sharedFiltered.reason "workspace-only recovery must report shared partition preservation"
    Assert-True (-not ($missingRecovery.actions | Where-Object { $_.path -eq $recentOrphanPartition -and $_.action -eq "would-delete" })) "recent missing workspaces must remain protected"
    Assert-True (-not ($missingRecovery.actions | Where-Object { $_.path -eq $invalidMarkerPartition -and $_.action -eq "would-delete" })) "invalid markers must remain protected"

    $lowDiskPolicy = Get-Content -Raw -LiteralPath $nonLowDiskPolicyPath | ConvertFrom-Json
    $lowDiskPolicy.warning_free_percent = 99
    $lowDiskPolicy.recovery_free_percent = 100
    $lowDiskPolicy | ConvertTo-Json -Depth 8 | Set-Content -LiteralPath $nonLowDiskPolicyPath -Encoding UTF8
    $lowDiskRecovery = Invoke-RustCacheGc -CacheRoot $CacheRoot -RepoRoot $ProjectRoot -RecoverMissingWorkspaces -WorkspaceOnly
    Assert-True $lowDiskRecovery.low_disk "recovery regression must exercise the low-disk selection path"
    $existingWorkspaceAction = $lowDiskRecovery.actions | Where-Object { $_.path -eq $existingWorkspacePartition } | Select-Object -First 1
    Assert-Equal "preserve" $existingWorkspaceAction.action "missing-workspace recovery must preserve an existing workspace under low disk"
    Assert-Equal "missing-workspace-filter" $existingWorkspaceAction.reason "exclusive recovery should report why an existing workspace was filtered"
    Assert-True (-not $existingWorkspaceAction.selected) "existing workspaces must remain unselected under low-disk recovery"
    Assert-True (@($lowDiskRecovery.actions | Where-Object { $_.path -eq $unmanagedMissingPartition -and $_.action -eq "would-delete" }).Count -eq 1) "low-disk recovery must still select an eligible missing workspace"
    $nonLowDiskPolicy | ConvertTo-Json -Depth 8 | Set-Content -LiteralPath $nonLowDiskPolicyPath -Encoding UTF8

    $workspaceOnlyAged = Invoke-RustCacheGc -CacheRoot $CacheRoot -RepoRoot $ProjectRoot -ForceAged -WorkspaceOnly
    $sharedAgedAction = $workspaceOnlyAged.actions | Where-Object { $_.path -eq $sharedTaskPartition } | Select-Object -First 1
    Assert-Equal "workspace-scope-filter" $sharedAgedAction.reason "workspace-only must override force-aged selection for shared partitions"
    Assert-True (-not $sharedAgedAction.selected) "shared partitions must remain unselected under workspace-only force-aged GC"

    $missingRecoveryApply = Invoke-RustCacheGc -CacheRoot $CacheRoot -RepoRoot $ProjectRoot -RecoverMissingWorkspaces -WorkspaceOnly -Apply
    Assert-True (-not (Test-Path -LiteralPath $unmanagedMissingPartition)) "missing workspace apply should delete the reviewed workspace partition"
    Assert-True (-not (Test-Path -LiteralPath $staleMissingPartition)) "managed deletion should atomically replace and remove a stale lock"
    Assert-True (Test-Path -LiteralPath $sharedTaskPartition) "missing workspace apply must preserve shared partitions"
    Assert-True (Test-Path -LiteralPath $recentOrphanPartition) "missing workspace apply must preserve recent partitions"
    Assert-True (Test-Path -LiteralPath $existingWorkspacePartition) "missing workspace apply must preserve partitions whose workspace still exists"

    $ownedTaskRoot = Join-Path $TempRoot "explicit-finish-task"
    $ownedWorkspace = Join-Path $ownedTaskRoot "server"
    New-Item -ItemType Directory -Force -Path $ownedWorkspace | Out-Null
    $ownedPartition = Join-Path $CacheRoot "build\$currentEpoch\test-project\dev-host\6666666666666666"
    New-Item -ItemType Directory -Force -Path $ownedPartition | Out-Null
    @{ workspace_root = $ownedWorkspace; cache_scope = "workspace"; cache_partition = "6666666666666666"; last_used_utc = [DateTime]::UtcNow.ToString("o") } |
        ConvertTo-Json | Set-Content -LiteralPath (Join-Path $ownedPartition ".last-used.json") -Encoding UTF8
    $ownedSharedPartition = Join-Path $CacheRoot "build\$currentEpoch\test-project\dev-host\shared-owned"
    New-Item -ItemType Directory -Force -Path $ownedSharedPartition | Out-Null
    @{ workspace_root = $ownedWorkspace; cache_scope = "shared"; cache_partition = "shared-owned"; last_used_utc = [DateTime]::UtcNow.ToString("o") } |
        ConvertTo-Json | Set-Content -LiteralPath (Join-Path $ownedSharedPartition ".last-used.json") -Encoding UTF8
    $taskCleanup = Clear-RustCacheTaskPartitions -CacheRoot $CacheRoot -TaskWorktree $ownedTaskRoot -Apply
    Assert-Equal 1 $taskCleanup.removed_count "task cleanup should remove only its workspace-scoped partition"
    Assert-True (-not (Test-Path -LiteralPath $ownedPartition)) "task cleanup should remove the owned workspace partition"
    Assert-True (Test-Path -LiteralPath $ownedSharedPartition) "task cleanup must preserve a shared partition for the same workspace"
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
    $atomicPartition = Join-Path $CacheRoot "build\rustc-test\test-project\one-off-atomic\dddddddddddddddd"
    New-Item -ItemType Directory -Force -Path $atomicPartition | Out-Null
    Set-Content -LiteralPath (Join-Path $atomicPartition "artifact.bin") -Value "atomic"
    & $inventoryModule {
        param($Root, $PartitionPath, $WorkspaceRoot)
        Remove-RustCachePartitionSafely -CacheRoot $Root -Path $PartitionPath -WorkspaceRoot $WorkspaceRoot
    } $CacheRoot $atomicPartition $ProjectRoot
    Assert-True (-not (Test-Path -LiteralPath $atomicPartition)) "GC should atomically detach a locked partition before deletion"
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

[source.crates-io]
replace-with = "ustc"

[source.ustc]
registry = "sparse+https://mirrors.ustc.edu.cn/crates.io-index/"

[net]
retry = 3
'@ | Set-Content -LiteralPath $CargoConfig -Encoding UTF8
    $proposal = Set-RustCacheParentCargoConfig -CargoConfigPath $CargoConfig -IncludeConfigPath (Join-Path $CacheRoot "config\cargo-cache.toml")
    Assert-True ($proposal.content -match '^include = ') "Cargo activation should add an include"
    Assert-True ($proposal.content -notmatch 'target-dir\s*=') "Cargo activation should remove universal target-dir"
    Assert-True ($proposal.content -notmatch 'rustflags\s*=') "Cargo activation should remove machine-wide rustflags"
    Assert-True ($proposal.content -match 'jobs = 12') "Cargo activation should preserve unrelated build settings"
    Assert-True ($proposal.content -match '\[net\]') "Cargo activation should preserve other sections"
    Assert-True ($proposal.content -match 'replace-with\s*=') "ordinary activation preview should not silently reset an explicit source policy"
    $resetProposal = Set-RustCacheParentCargoConfig -CargoConfigPath $CargoConfig -IncludeConfigPath (Join-Path $CacheRoot "config\cargo-cache.toml") -ResetSourceReplacement
    Assert-True ($resetProposal.content -notmatch 'replace-with\s*=') "explicit source reset should remove permanent crates.io replacement"
    Assert-Equal 'ustc' $resetProposal.removed_source_replacements[0] "source reset should report the removed replacement"
    Assert-True ($resetProposal.content -match '\[source\.ustc\]') "source reset should preserve inactive source definitions and unrelated user data"

    $install = Install-RustCachePlatform -SourceScriptsRoot $PSScriptRoot -CacheRoot (Join-Path $TempRoot "installed") -RepoRoot $ProjectRoot
    Assert-True (Test-Path -LiteralPath $install.entry_path) "installer should copy the entry script"
    $installedCommand = Get-Command -Name $install.entry_path -ErrorAction Stop
    Assert-True ($installedCommand.Parameters.ContainsKey("SharedBuildPartition")) "installed machine entry should expose named shared partitions"
    Assert-True ($installedCommand.Parameters.ContainsKey("SharedAliasesOnly")) "installed machine entry should expose exact shared-alias GC"
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
    $localServerScript = Get-Content -Raw -LiteralPath (Join-Path $PSScriptRoot "start-local-elon-server.ps1")
    Assert-True ($localServerScript -match 'cargo-dev\.ps1') "local server builds should use the managed Rust cache entry"
    Assert-True ($localServerScript -match 'SharedBuildPartition local-server') "local server builds should reuse one named build partition"
    Assert-True ($localServerScript -notmatch [regex]::Escape('D:\rust\shared\target')) "local server builds must not recreate the retired universal target"
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

    $versionedTarget = Join-Path $TempRoot 'target-v261-linux-musl'
    New-Item -ItemType Directory -Force -Path $versionedTarget | Out-Null
    Set-Content -LiteralPath (Join-Path $versionedTarget '.rustc_info.json') -Value '{}'
    Set-Content -LiteralPath (Join-Path $versionedTarget 'CACHEDIR.TAG') -Value 'Signature: 8a477f597d28d172789f06886806bc55'
    Add-RustCacheLegacyRecord -CacheRoot $CacheRoot -Path $versionedTarget -Label 'versioned-target' -Retired | Out-Null
    $versionedPlan = Invoke-RustCacheLegacyPurge -CacheRoot $CacheRoot -RepoRoot $ProjectRoot -LegacyPath $versionedTarget
    Assert-Equal 'would-delete' $versionedPlan.action 'marked versioned targets should support dry-run purge'
    Assert-True (Test-Path -LiteralPath $versionedTarget) 'versioned target dry-run must preserve files'
    $unmarkedVersionedTarget = Join-Path $TempRoot 'target-v262-linux-musl'
    New-Item -ItemType Directory -Force -Path $unmarkedVersionedTarget | Out-Null
    Add-RustCacheLegacyRecord -CacheRoot $CacheRoot -Path $unmarkedVersionedTarget -Label 'unmarked-versioned-target' -Retired | Out-Null
    $unmarkedRejected = $false
    try { Invoke-RustCacheLegacyPurge -CacheRoot $CacheRoot -RepoRoot $ProjectRoot -LegacyPath $unmarkedVersionedTarget } catch { $unmarkedRejected = $_.Exception.Message -match 'missing Cargo cache markers' }
    Assert-True $unmarkedRejected 'versioned targets without both Cargo markers must fail closed'

    $activeGcPartition = Join-Path $CacheRoot "build\rustc-old\test-project\dev-host\eeeeeeeeeeeeeeee"
    New-Item -ItemType Directory -Force -Path $activeGcPartition | Out-Null
    '{"last_used_utc":"2000-01-01T00:00:00Z"}' | Set-Content -LiteralPath (Join-Path $activeGcPartition ".last-used.json") -Encoding UTF8
    Set-Content -LiteralPath (Join-Path $activeGcPartition "artifact.bin") -Value "managed-old"
    $activeGcQuarantine = Join-Path $CacheRoot "quarantine\ff\ffffffffffffff"
    New-Item -ItemType Directory -Force -Path $activeGcQuarantine | Out-Null
    '{"last_used_utc":"2000-01-01T00:00:00Z"}' | Set-Content -LiteralPath (Join-Path $activeGcQuarantine ".last-used.json") -Encoding UTF8
    Set-Content -LiteralPath (Join-Path $activeGcQuarantine "artifact.bin") -Value "unmanaged-old"
    $fakeCargoPath = Join-Path $TempRoot "cargo.exe"
    Copy-Item -LiteralPath (Join-Path $PSHOME "powershell.exe") -Destination $fakeCargoPath
    $fakeCargo = Start-Process -FilePath $fakeCargoPath -ArgumentList @('-NoProfile','-Command','Start-Sleep 60') -WindowStyle Hidden -PassThru
    try {
        $activeDeadline = [DateTime]::UtcNow.AddSeconds(5)
        while (-not (Get-Process -Name cargo -ErrorAction SilentlyContinue) -and [DateTime]::UtcNow -lt $activeDeadline) { Start-Sleep -Milliseconds 20 }
        Assert-True ($null -ne (Get-Process -Name cargo -ErrorAction SilentlyContinue)) "active-build GC regression needs a visible Cargo process"
        if (Get-Command sccache -ErrorAction SilentlyContinue) {
            $deferredSccache = Restart-RustCacheSccacheServer -CacheRoot $CacheRoot -MaxCacheSize "20G"
            Assert-Equal "deferred" $deferredSccache.status "platform installation should defer SCCache reload while Cargo is active"
            Assert-True $deferredSccache.restart_pending "deferred SCCache activation should remain durably pending"
        }
        $activeGc = Invoke-RustCacheGc -CacheRoot $CacheRoot -RepoRoot $ProjectRoot -ForceAged -Apply
        Assert-True (-not (Test-Path -LiteralPath $activeGcPartition)) "GC should reclaim an unlocked managed partition while unrelated Cargo is active"
        Assert-True (Test-Path -LiteralPath $activeGcQuarantine) "GC must preserve quarantine while unmanaged Cargo may be writing"
        $quarantineAction = $activeGc.actions | Where-Object { $_.path -eq $activeGcQuarantine } | Select-Object -First 1
        Assert-Equal "unmanaged-build-process-active" $quarantineAction.reason "active Cargo should explain quarantine preservation"
        Assert-True (@($activeGc.active_build_processes).Count -gt 0) "GC report should audit active build processes"
    } finally {
        if ($fakeCargo -and -not $fakeCargo.HasExited) { Stop-Process -Id $fakeCargo.Id -Force -ErrorAction SilentlyContinue }
        if ($fakeCargo) { $fakeCargo.Dispose() }
    }

    Write-Host "PASS: Rust cache platform tests ($script:Assertions assertions)." -ForegroundColor Green
} finally {
    Remove-Item Env:RUST_CACHE_TEST_ENV -ErrorAction SilentlyContinue
    if ($null -eq $previousTaskWorktreeBase) {
        Remove-Item Env:ELON_AI_TASK_WORKTREE_BASE -ErrorAction SilentlyContinue
    } else {
        $env:ELON_AI_TASK_WORKTREE_BASE = $previousTaskWorktreeBase
    }
    Remove-Item -LiteralPath $TempRoot -Recurse -Force -ErrorAction SilentlyContinue
}
