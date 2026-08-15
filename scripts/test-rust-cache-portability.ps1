[CmdletBinding()]
param()

$ErrorActionPreference = "Stop"
$modulesRoot = Join-Path $PSScriptRoot "rust-cache"
Import-Module "$modulesRoot\RustCache.Paths.psm1" -Force -DisableNameChecking
Import-Module "$modulesRoot\RustCache.Policy.psm1" -Force -DisableNameChecking
Import-Module "$modulesRoot\RustCache.Portability.psm1" -Force -DisableNameChecking
# Nested module imports are scoped to the owner. Re-import public test surfaces last.
Import-Module "$modulesRoot\RustCache.Policy.psm1" -Force -DisableNameChecking
Import-Module "$modulesRoot\RustCache.Paths.psm1" -Force -DisableNameChecking

$script:Assertions = 0
function Assert-True {
    param([bool]$Condition, [string]$Message)
    $script:Assertions++
    if (-not $Condition) { throw "Assertion failed: $Message" }
}

function Assert-Equal {
    param($Expected, $Actual, [string]$Message)
    $script:Assertions++
    if ($Expected -ne $Actual) { throw "Assertion failed: $Message. Expected='$Expected' Actual='$Actual'" }
}

$tempRoot = Join-Path ([System.IO.Path]::GetTempPath()) ("rust-cache-portability-" + [Guid]::NewGuid().ToString("N"))
$projectRoot = Join-Path $tempRoot "project"
$cacheRoot = Join-Path $tempRoot "cache"
$codexSkillsRoot = Join-Path $tempRoot "codex-skills"
New-Item -ItemType Directory -Force -Path $projectRoot | Out-Null

try {
    $missingCandidate = Join-Path $tempRoot "not-created-by-doctor"
    $resolved = Resolve-RustCacheRoot -ExplicitRoot $missingCandidate -RepoRoot $projectRoot -NoCreate
    Assert-Equal ([System.IO.Path]::GetFullPath($missingCandidate)) $resolved "read-only root resolution"
    Assert-True (-not (Test-Path -LiteralPath $missingCandidate)) "read-only root resolution must not create directories"

    $preview = New-RustCacheProjectManifest -ProjectRoot $projectRoot -ProjectId "portable-test" -AllowedDomains @("dev-windows-msvc", "agent-validation") -SharedPartitionDomains @("validation-light=agent-validation")
    Assert-Equal "would-create" $preview.action "project adoption should preview by default"
    Assert-True (-not (Test-Path -LiteralPath $preview.path)) "project adoption preview must not write"
    $created = New-RustCacheProjectManifest -ProjectRoot $projectRoot -ProjectId "portable-test" -AllowedDomains @("dev-windows-msvc", "agent-validation") -SharedPartitionDomains @("validation-light=agent-validation") -Apply
    Assert-Equal "created" $created.action "project adoption apply"
    $manifest = Get-RustCacheProjectManifest -ProjectRoot $projectRoot
    Assert-Equal "portable-test" $manifest.project_id "portable project identity"
    Assert-Equal "agent-validation" $manifest.shared_partition_domains["validation-light"] "portable shared partition mapping"
    $unchanged = New-RustCacheProjectManifest -ProjectRoot $projectRoot -ProjectId "portable-test" -AllowedDomains @("agent-validation", "dev-windows-msvc") -SharedPartitionDomains @("validation-light=agent-validation") -Apply
    Assert-Equal "unchanged" $unchanged.action "project adoption must be idempotent"
    $overwriteRejected = $false
    try {
        New-RustCacheProjectManifest -ProjectRoot $projectRoot -ProjectId "different-project" -Apply | Out-Null
    } catch { $overwriteRejected = $_.Exception.Message -match "different settings" }
    Assert-True $overwriteRejected "project adoption must fail closed instead of overwriting"

    $sourceFingerprint = Get-RustCachePlatformFingerprint -SourceScriptsRoot $PSScriptRoot
    Assert-True ($sourceFingerprint.file_count -gt 5) "source fingerprint should cover the platform files"
    $platformRoot = Join-Path $cacheRoot "platform"
    $targetModules = Join-Path $platformRoot "rust-cache"
    New-Item -ItemType Directory -Force -Path $targetModules | Out-Null
    Copy-Item -LiteralPath (Join-Path $PSScriptRoot "rust-cache.ps1") -Destination (Join-Path $platformRoot "rust-cache.ps1")
    Get-ChildItem -LiteralPath $modulesRoot -File -Filter "*.psm1" | ForEach-Object {
        Copy-Item -LiteralPath $_.FullName -Destination (Join-Path $targetModules $_.Name)
    }
    $installedFingerprint = Get-RustCachePlatformFingerprint -SourceScriptsRoot $platformRoot -Installed
    $installManifestPath = Write-RustCachePlatformInstallManifest -CacheRoot $cacheRoot -SourceFingerprint $sourceFingerprint -InstalledFingerprint $installedFingerprint
    Assert-True (Test-Path -LiteralPath $installManifestPath) "platform installation should record a manifest"
    Assert-Equal $sourceFingerprint.hash (Read-RustCachePlatformInstallManifest -CacheRoot $cacheRoot).source_hash "platform source fingerprint"

    $includePath = Join-Path $cacheRoot "config\cargo-cache.toml"
    New-Item -ItemType Directory -Force -Path (Split-Path $includePath -Parent) | Out-Null
    Set-Content -LiteralPath $includePath -Value "[build]`nbuild-dir = 'placeholder'" -Encoding UTF8
    $cargoConfig = Join-Path $tempRoot "cargo\config.toml"
    New-Item -ItemType Directory -Force -Path (Split-Path $cargoConfig -Parent) | Out-Null
    Set-Content -LiteralPath $cargoConfig -Value ("include = ['" + $includePath.Replace('\', '/') + "']") -Encoding UTF8
    $policyPath = Get-RustCachePolicyPath -CacheRoot $cacheRoot
    Assert-True (-not (Test-Path -LiteralPath $policyPath)) "doctor fixture should begin without a policy"
    $skillRoot = Join-Path (Split-Path $PSScriptRoot -Parent) ".agents\skills\manage-shared-build-cache"
    $skillInstall = Install-RustCacheCodexSkill -SourceSkillRoot $skillRoot -CodexSkillsRoot $codexSkillsRoot
    Assert-True (Test-Path -LiteralPath (Join-Path $skillInstall.path "SKILL.md")) "skill installer should copy SKILL.md"
    Assert-True (Test-Path -LiteralPath $skillInstall.marker_path) "skill installer should record provenance"
    $doctor = Get-RustCacheDoctor -ProjectRoot $projectRoot -SourceScriptsRoot $PSScriptRoot -CacheRoot $cacheRoot -CargoConfigPath $cargoConfig -SourceSkillRoot $skillRoot -CodexSkillsRoot $codexSkillsRoot
    Assert-Equal "pass" ($doctor.checks | Where-Object id -eq "project-manifest").status "doctor project manifest check"
    Assert-Equal "pass" ($doctor.checks | Where-Object id -eq "platform-install").status "doctor platform install check"
    Assert-True (-not $doctor.destructive_actions_taken) "doctor must be read-only"
    Assert-True (-not (Test-Path -LiteralPath $policyPath)) "doctor must not initialize policy state"
    Assert-Equal "pass" ($doctor.checks | Where-Object id -eq "platform-version").status "doctor platform version check"
    Assert-Equal "pass" ($doctor.checks | Where-Object id -eq "codex-skill").status "doctor skill version check"
    $installedDoctor = Get-RustCacheDoctor -ProjectRoot $projectRoot -SourceScriptsRoot $platformRoot -CacheRoot $cacheRoot -CargoConfigPath $cargoConfig
    Assert-Equal "installed" $installedDoctor.source_mode "installed entry doctor mode"
    Assert-Equal "warn" ($installedDoctor.checks | Where-Object id -eq "platform-version").status "installed entry should defer source freshness"
    Assert-Equal "pass" ($installedDoctor.checks | Where-Object id -eq "platform-integrity").status "installed entry integrity check"

    Add-Content -LiteralPath (Join-Path $targetModules "RustCache.Portability.psm1") -Value "# tampered"
    $tampered = Get-RustCacheDoctor -ProjectRoot $projectRoot -SourceScriptsRoot $PSScriptRoot -CacheRoot $cacheRoot -CargoConfigPath $cargoConfig
    Assert-True (-not $tampered.healthy) "doctor should reject changed installed files"
    Assert-Equal "fail" ($tampered.checks | Where-Object id -eq "platform-integrity").status "doctor integrity check"

    Write-Host "PASS: Rust cache portability tests ($script:Assertions assertions)." -ForegroundColor Green
} finally {
    Remove-Item -LiteralPath $tempRoot -Recurse -Force -ErrorAction SilentlyContinue
}
