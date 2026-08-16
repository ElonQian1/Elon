[CmdletBinding()]
param()

$ErrorActionPreference = "Stop"
$modulesRoot = Join-Path $PSScriptRoot "rust-cache"
Import-Module "$modulesRoot\RustCache.Paths.psm1" -Force -DisableNameChecking
Import-Module "$modulesRoot\RustCache.Policy.psm1" -Force -DisableNameChecking
Import-Module "$modulesRoot\RustCache.Launcher.psm1" -Force -DisableNameChecking
Import-Module "$modulesRoot\RustCache.Portability.psm1" -Force -DisableNameChecking
Import-Module "$modulesRoot\RustCache.ProjectAdoption.psm1" -Force -DisableNameChecking
Import-Module "$modulesRoot\RustCache.Fleet.psm1" -Force -DisableNameChecking
Import-Module "$modulesRoot\RustCache.FleetQueue.psm1" -Force -DisableNameChecking
Import-Module "$modulesRoot\RustCache.Help.psm1" -Force -DisableNameChecking
# Nested module imports are scoped to the owner. Re-import public test surfaces last.
Import-Module "$modulesRoot\RustCache.FleetQueue.psm1" -Force -DisableNameChecking
Import-Module "$modulesRoot\RustCache.Fleet.psm1" -Force -DisableNameChecking
Import-Module "$modulesRoot\RustCache.Portability.psm1" -Force -DisableNameChecking
Import-Module "$modulesRoot\RustCache.ProjectAdoption.psm1" -Force -DisableNameChecking
Import-Module "$modulesRoot\RustCache.Portability.psm1" -Force -DisableNameChecking
Import-Module "$modulesRoot\RustCache.Policy.psm1" -Force -DisableNameChecking
Import-Module "$modulesRoot\RustCache.Paths.psm1" -Force -DisableNameChecking
Import-Module "$modulesRoot\RustCache.Launcher.psm1" -Force -DisableNameChecking

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

function Set-TestTreeLineEnding {
    param([Parameter(Mandatory)][string]$Root, [Parameter(Mandatory)][ValidateSet("lf", "crlf")][string]$LineEnding)

    $utf8 = [System.Text.UTF8Encoding]::new($false)
    Get-ChildItem -LiteralPath $Root -Recurse -File | Where-Object { $_.Extension -in @(".ps1", ".psm1", ".rs", ".md", ".yaml") } | ForEach-Object {
        $text = [System.IO.File]::ReadAllText($_.FullName)
        $text = $text.Replace("`r`n", "`n").Replace("`r", "`n")
        if ($LineEnding -eq "crlf") { $text = $text.Replace("`n", "`r`n") }
        [System.IO.File]::WriteAllText($_.FullName, $text, $utf8)
    }
}

$tempRoot = Join-Path ([System.IO.Path]::GetTempPath()) ("rust-cache-portability-" + [Guid]::NewGuid().ToString("N"))
$projectRoot = Join-Path $tempRoot "project"
$cacheRoot = Join-Path $tempRoot "cache"
$codexSkillsRoot = Join-Path $tempRoot "codex-skills"
$userLauncherPath = Join-Path $tempRoot "user-bin\rust-cache.ps1"
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
    $lineEndingSourceRoot = Join-Path $tempRoot "line-ending-source"
    New-Item -ItemType Directory -Force -Path $lineEndingSourceRoot | Out-Null
    Copy-Item -LiteralPath (Join-Path $PSScriptRoot "rust-cache.ps1") -Destination (Join-Path $lineEndingSourceRoot "rust-cache.ps1")
    Copy-Item -LiteralPath $modulesRoot -Destination (Join-Path $lineEndingSourceRoot "rust-cache") -Recurse
    Set-TestTreeLineEnding -Root $lineEndingSourceRoot -LineEnding lf
    $lfSourceFingerprint = Get-RustCachePlatformFingerprint -SourceScriptsRoot $lineEndingSourceRoot
    $lfInstalledFingerprint = Get-RustCachePlatformFingerprint -SourceScriptsRoot $lineEndingSourceRoot -Installed
    Set-TestTreeLineEnding -Root $lineEndingSourceRoot -LineEnding crlf
    $crlfSourceFingerprint = Get-RustCachePlatformFingerprint -SourceScriptsRoot $lineEndingSourceRoot
    $crlfInstalledFingerprint = Get-RustCachePlatformFingerprint -SourceScriptsRoot $lineEndingSourceRoot -Installed
    Assert-Equal $sourceFingerprint.hash $lfSourceFingerprint.hash "source fingerprint should ignore checkout line endings"
    Assert-Equal $lfSourceFingerprint.hash $crlfSourceFingerprint.hash "source fingerprint should be stable across LF and CRLF"
    Assert-True ($lfInstalledFingerprint.hash -ne $crlfInstalledFingerprint.hash) "installed integrity fingerprint must retain exact bytes"
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
    $userLauncher = Install-RustCacheUserLauncher -CacheRoot $cacheRoot -UserLauncherPath $userLauncherPath
    Assert-Equal $userLauncherPath $userLauncher.path "portable launcher path"
    Assert-True (Test-Path -LiteralPath $userLauncher.path) "portable launcher should be installed"
    Assert-Equal "healthy" (Test-RustCacheUserLauncher -CacheRoot $cacheRoot -UserLauncherPath $userLauncherPath).status "portable launcher integrity"
    $launcherContent = Get-Content -Raw -LiteralPath $userLauncher.path -Encoding UTF8
    Assert-True ($launcherContent -notmatch 'Start-Process|powershell\.exe|pwsh\.exe') "portable launcher must not open a nested visible shell"
    $launcherHelp = & $userLauncher.path help | Select-Object -Last 1
    Assert-Equal "elon.rust_cache.command_help.v1" $launcherHelp.schema "portable launcher help schema"
    Assert-True (@($launcherHelp.commands.name) -contains "fleet-report") "portable launcher help should advertise fleet reports"
    Assert-True (@($launcherHelp.commands.name) -contains "fleet-stage") "portable launcher help should advertise fleet staging"
    Assert-True (@($launcherHelp.commands.name) -contains "gc-plan") "portable launcher help should advertise immutable GC plans"
    Assert-True (@($launcherHelp.commands.name) -contains "gc-apply-approved") "portable launcher help should advertise digest-bound approved GC"
    Assert-True (@($launcherHelp.commands.name) -contains "adopt-project") "portable launcher help should advertise child-project adoption"
    $launcherResult = & $userLauncher.path init-project -ProjectRoot $projectRoot -ProjectId "portable-test" -AllowedDomain @("dev-windows-msvc", "agent-validation") -SharedPartitionDomain @("validation-light=agent-validation") -Apply
    Assert-Equal "unchanged" $launcherResult.action "portable launcher should invoke the installed platform"

    $adoptionRoot = Join-Path $tempRoot "adopted-child"
    New-Item -ItemType Directory -Force -Path $adoptionRoot | Out-Null
    $adoptionPreview = New-RustCacheProjectAdoption -ProjectRoot $adoptionRoot -ProjectId "portable-child" -AllowedDomains @("dev-windows-msvc", "agent-validation")
    Assert-Equal "preview" $adoptionPreview.mode "child-project adoption should preview by default"
    Assert-True (-not (Test-Path -LiteralPath (Join-Path $adoptionRoot "rust-cache.project.json"))) "adoption preview must not create a manifest"
    Assert-True (-not (Test-Path -LiteralPath (Join-Path $adoptionRoot "scripts\rust-cache.ps1"))) "adoption preview must not create a wrapper"
    Assert-True (@($adoptionPreview.files.action | Where-Object { $_ -eq "would-create" }).Count -eq 2) "adoption preview should plan both portable files"

    $adoption = New-RustCacheProjectAdoption -ProjectRoot $adoptionRoot -ProjectId "portable-child" -AllowedDomains @("dev-windows-msvc", "agent-validation") -Apply
    Assert-Equal "apply" $adoption.mode "child-project adoption apply mode"
    $projectWrapper = Join-Path $adoptionRoot "scripts\rust-cache.ps1"
    Assert-True (Test-Path -LiteralPath $projectWrapper -PathType Leaf) "child-project adoption should create the thin wrapper"
    Assert-Equal "portable-child" (Get-RustCacheProjectManifest -ProjectRoot $adoptionRoot).project_id "child-project adoption manifest"
    $projectWrapperContent = Get-Content -Raw -LiteralPath $projectWrapper -Encoding UTF8
    Assert-True ($projectWrapperContent -notmatch [regex]::Escape($tempRoot)) "project wrapper must not contain the generating PC path"
    Assert-True ($projectWrapperContent -notmatch 'Start-Process|powershell\.exe|pwsh\.exe') "project wrapper must not open a nested visible shell"

    $projectAdoptionAgain = New-RustCacheProjectAdoption -ProjectRoot $adoptionRoot -ProjectId "portable-child" -AllowedDomains @("agent-validation", "dev-windows-msvc") -Apply
    Assert-True (@($projectAdoptionAgain.files.action | Where-Object { $_ -eq "unchanged" }).Count -eq 2) "child-project adoption must be idempotent"

    $fakeLocalAppData = Join-Path $tempRoot "local-app-data"
    $projectLauncherPath = Join-Path $fakeLocalAppData "Elon\bin\rust-cache.ps1"
    New-Item -ItemType Directory -Force -Path (Split-Path $projectLauncherPath -Parent) | Out-Null
    Copy-Item -LiteralPath $userLauncher.path -Destination $projectLauncherPath
    $oldLocalAppData = $env:LOCALAPPDATA
    try {
        $env:LOCALAPPDATA = $fakeLocalAppData
        $forwarded = & $projectWrapper init-project -ProjectId "portable-child" -AllowedDomain @("dev-windows-msvc", "agent-validation") -Apply | Select-Object -Last 1
        Assert-Equal "unchanged" $forwarded.action "project wrapper should bind and forward its own project root"
        $overrideRejected = $false
        try { & $projectWrapper status -ProjectRoot $projectRoot | Out-Null } catch { $overrideRejected = $_.Exception.Message -match "owns -ProjectRoot" }
        Assert-True $overrideRejected "project wrapper must reject a caller-supplied project root"
    } finally {
        $env:LOCALAPPDATA = $oldLocalAppData
    }

    Set-Content -LiteralPath $projectWrapper -Value "# user-owned wrapper" -Encoding UTF8
    $wrapperConflictRejected = $false
    try {
        New-RustCacheProjectAdoption -ProjectRoot $adoptionRoot -ProjectId "portable-child" -AllowedDomains @("dev-windows-msvc", "agent-validation") -Apply | Out-Null
    } catch { $wrapperConflictRejected = $_.Exception.Message -match "different content" }
    Assert-True $wrapperConflictRejected "child-project adoption must not overwrite an existing different wrapper"
    Assert-True ((Get-Content -Raw -LiteralPath $projectWrapper) -match "user-owned wrapper") "conflicting project wrapper must be preserved"

    $includePath = Join-Path $cacheRoot "config\cargo-cache.toml"
    New-Item -ItemType Directory -Force -Path (Split-Path $includePath -Parent) | Out-Null
    Set-Content -LiteralPath $includePath -Value "[build]`nbuild-dir = 'placeholder'" -Encoding UTF8
    $cargoConfig = Join-Path $tempRoot "cargo\config.toml"
    New-Item -ItemType Directory -Force -Path (Split-Path $cargoConfig -Parent) | Out-Null
    Set-Content -LiteralPath $cargoConfig -Value ("include = ['" + $includePath.Replace('\', '/') + "']") -Encoding UTF8
    $policyPath = Get-RustCachePolicyPath -CacheRoot $cacheRoot
    Assert-True (-not (Test-Path -LiteralPath $policyPath)) "doctor fixture should begin without a policy"
    $skillRoot = Join-Path (Split-Path $PSScriptRoot -Parent) ".agents\skills\manage-shared-build-cache"
    $skillVariantRoot = Join-Path $tempRoot "skill-line-ending-source"
    Copy-Item -LiteralPath $skillRoot -Destination $skillVariantRoot -Recurse
    Set-TestTreeLineEnding -Root $skillVariantRoot -LineEnding lf
    $lfSkillFingerprint = Get-RustCacheCodexSkillFingerprint -SkillRoot $skillVariantRoot
    Set-TestTreeLineEnding -Root $skillVariantRoot -LineEnding crlf
    $crlfSkillFingerprint = Get-RustCacheCodexSkillFingerprint -SkillRoot $skillVariantRoot
    Assert-Equal (Get-RustCacheCodexSkillFingerprint -SkillRoot $skillRoot) $lfSkillFingerprint "skill source fingerprint should ignore checkout line endings"
    Assert-Equal $lfSkillFingerprint $crlfSkillFingerprint "skill source fingerprint should be stable across LF and CRLF"
    $skillInstall = Install-RustCacheCodexSkill -SourceSkillRoot $skillRoot -CodexSkillsRoot $codexSkillsRoot
    Assert-True (Test-Path -LiteralPath (Join-Path $skillInstall.path "SKILL.md")) "skill installer should copy SKILL.md"
    Assert-True (Test-Path -LiteralPath $skillInstall.marker_path) "skill installer should record provenance"
    $doctor = Get-RustCacheDoctor -ProjectRoot $projectRoot -SourceScriptsRoot $PSScriptRoot -CacheRoot $cacheRoot -CargoConfigPath $cargoConfig -SourceSkillRoot $skillRoot -CodexSkillsRoot $codexSkillsRoot -UserLauncherPath $userLauncherPath
    Assert-Equal "pass" ($doctor.checks | Where-Object id -eq "project-manifest").status "doctor project manifest check"
    Assert-Equal "pass" ($doctor.checks | Where-Object id -eq "platform-install").status "doctor platform install check"
    Assert-True (-not $doctor.destructive_actions_taken) "doctor must be read-only"
    Assert-True (-not (Test-Path -LiteralPath $policyPath)) "doctor must not initialize policy state"
    Assert-Equal "pass" ($doctor.checks | Where-Object id -eq "platform-version").status "doctor platform version check"
    Assert-Equal "pass" ($doctor.checks | Where-Object id -eq "codex-skill").status "doctor skill version check"
    Assert-Equal "pass" ($doctor.checks | Where-Object id -eq "user-launcher").status "doctor portable launcher check"
    $lineEndingDoctor = Get-RustCacheDoctor -ProjectRoot $projectRoot -SourceScriptsRoot $lineEndingSourceRoot -CacheRoot $cacheRoot -CargoConfigPath $cargoConfig -SourceSkillRoot $skillVariantRoot -CodexSkillsRoot $codexSkillsRoot -UserLauncherPath $userLauncherPath
    Assert-Equal "pass" ($lineEndingDoctor.checks | Where-Object id -eq "platform-version").status "doctor should accept an equivalent CRLF checkout"
    Assert-Equal "pass" ($lineEndingDoctor.checks | Where-Object id -eq "codex-skill").status "doctor should accept an equivalent CRLF skill source"
    $fleetPath = Join-Path $tempRoot "exports\fleet.json"
    $fleetReport = New-RustCacheFleetReport -ProjectRoot $projectRoot -SourceScriptsRoot $PSScriptRoot -CacheRoot $cacheRoot -CargoConfigPath $cargoConfig -SourceSkillRoot $skillRoot -CodexSkillsRoot $codexSkillsRoot -UserLauncherPath $userLauncherPath -NodeId "portable-node-1" -IncludeSizes
    Assert-Equal "elon.rust_cache.fleet_report.v1" $fleetReport.schema "fleet report schema"
    Assert-Equal "portable-node-1" $fleetReport.node.node_id "fleet report explicit node identity"
    Assert-Equal "portable-test" $fleetReport.project.project_id "fleet report project identity"
    Assert-True (-not $fleetReport.destructive_actions_taken) "fleet report must not run destructive actions"
    $fleetExport = Export-RustCacheFleetReport -Report $fleetReport -CacheRoot $cacheRoot -OutputPath $fleetPath
    Assert-True (Test-Path -LiteralPath $fleetExport.report_path -PathType Leaf) "fleet report export path"
    Assert-True (-not [string]::IsNullOrWhiteSpace([string]$fleetExport.content_sha256)) "fleet report export hash"
    $fleetJson = Get-Content -Raw -LiteralPath $fleetPath -Encoding UTF8
    Assert-True ($fleetJson -notmatch [regex]::Escape($projectRoot)) "fleet report must omit the project absolute path"
    Assert-True ($fleetJson -notmatch [regex]::Escape($cacheRoot)) "fleet report must omit the cache absolute path"
    Assert-True ($fleetJson -notmatch 'project_root|cache_root|user_launcher_path') "fleet report must omit path-bearing fields"
    $fleetEnvelope = New-RustCacheFleetEnvelope -Report $fleetReport
    Assert-Equal "elon.rust_cache.fleet_envelope.v1" $fleetEnvelope.schema "fleet envelope schema"
    Assert-Equal "portable-node-1" $fleetEnvelope.node_id "fleet envelope node identity"
    Assert-True (-not $fleetEnvelope.security.destructive_actions_authorized) "fleet envelope must not authorize destructive actions"
    $fleetEnvelopeValidation = Test-RustCacheFleetEnvelope -Envelope $fleetEnvelope
    Assert-True $fleetEnvelopeValidation.valid "fresh fleet envelope validation"
    Assert-Equal "portable-test" $fleetEnvelopeValidation.report.project.project_id "fleet envelope should retain the sanitized report"
    $fleetEnvelopePath = Join-Path $tempRoot "exports\fleet-envelope.json"
    $fleetStage = Export-RustCacheFleetEnvelope -Envelope $fleetEnvelope -CacheRoot $cacheRoot -OutputPath $fleetEnvelopePath
    Assert-Equal "elon.rust_cache.fleet_stage.v1" $fleetStage.schema "fleet stage schema"
    Assert-True (Test-Path -LiteralPath $fleetStage.envelope_path -PathType Leaf) "fleet envelope export path"
    $fleetEnvelopeJson = Get-Content -Raw -LiteralPath $fleetEnvelopePath -Encoding UTF8
    Assert-True ($fleetEnvelopeJson -notmatch [regex]::Escape($projectRoot)) "fleet envelope must omit the project absolute path"
    Assert-True ($fleetEnvelopeJson -notmatch [regex]::Escape($cacheRoot)) "fleet envelope must omit the cache absolute path"
    Assert-True ([string]::IsNullOrWhiteSpace($env:COMPUTERNAME) -or $fleetEnvelopeJson -notmatch [regex]::Escape($env:COMPUTERNAME)) "fleet envelope must omit the host name"
    $tamperedEnvelope = $fleetEnvelope | ConvertTo-Json -Depth 10 | ConvertFrom-Json
    $tamperedEnvelope.report.json = $tamperedEnvelope.report.json.Replace("portable-test", "tampered-project")
    Assert-True (-not (Test-RustCacheFleetEnvelope -Envelope $tamperedEnvelope).valid) "fleet envelope should detect report tampering"
    $missingEnvelopeNodeRejected = $false
    $reportWithoutNode = $fleetReport | ConvertTo-Json -Depth 10 | ConvertFrom-Json
    $reportWithoutNode.node.node_id = $null
    try { New-RustCacheFleetEnvelope -Report $reportWithoutNode | Out-Null } catch { $missingEnvelopeNodeRejected = $_.Exception.Message -match "NodeId" }
    Assert-True $missingEnvelopeNodeRejected "fleet staging should require an explicit platform node identity"
    $invalidNodeRejected = $false
    try { New-RustCacheFleetReport -ProjectRoot $projectRoot -SourceScriptsRoot $PSScriptRoot -CacheRoot $cacheRoot -NodeId "invalid node id" | Out-Null } catch { $invalidNodeRejected = $_.Exception.Message -match "NodeId" }
    Assert-True $invalidNodeRejected "fleet report should reject unstable node identifiers"
    $relativeOutputRejected = $false
    try { Export-RustCacheFleetReport -Report $fleetReport -CacheRoot $cacheRoot -OutputPath "fleet.json" | Out-Null } catch { $relativeOutputRejected = $_.Exception.Message -match "absolute" }
    Assert-True $relativeOutputRejected "fleet report export should reject relative output paths"
    $launcherFleetPath = Join-Path $tempRoot "exports\launcher-fleet.json"
    $launcherFleet = & $userLauncherPath fleet-report -ProjectRoot $projectRoot -CacheRoot $cacheRoot -CargoConfigPath $cargoConfig -UserLauncherPath $userLauncherPath -NodeId "portable-node-2" -OutputPath $launcherFleetPath | Select-Object -Last 1
    Assert-Equal "elon.rust_cache.fleet_export.v1" $launcherFleet.schema "installed launcher fleet export schema"
    Assert-True (Test-Path -LiteralPath $launcherFleetPath -PathType Leaf) "installed launcher should write fleet report"
    $launcherFleetEnvelopePath = Join-Path $tempRoot "exports\launcher-fleet-envelope.json"
    $launcherFleetStage = & $userLauncherPath fleet-stage -ProjectRoot $projectRoot -CacheRoot $cacheRoot -CargoConfigPath $cargoConfig -UserLauncherPath $userLauncherPath -NodeId "portable-node-3" -OutputPath $launcherFleetEnvelopePath | Select-Object -Last 1
    Assert-Equal "elon.rust_cache.fleet_stage.v1" $launcherFleetStage.schema "installed launcher fleet stage schema"
    Assert-True (Test-Path -LiteralPath $launcherFleetEnvelopePath -PathType Leaf) "installed launcher should queue a fleet envelope"
    $defaultFleetStage = & $userLauncherPath fleet-stage -ProjectRoot $projectRoot -CacheRoot $cacheRoot -CargoConfigPath $cargoConfig -UserLauncherPath $userLauncherPath -NodeId "portable-node-4" | Select-Object -Last 1
    Assert-True ($defaultFleetStage.envelope_path.StartsWith((Join-Path $cacheRoot "reports\fleet\outbox"), [System.StringComparison]::OrdinalIgnoreCase)) "default fleet stage should stay inside the managed outbox"
    $installedDoctor = Get-RustCacheDoctor -ProjectRoot $projectRoot -SourceScriptsRoot $platformRoot -CacheRoot $cacheRoot -CargoConfigPath $cargoConfig -UserLauncherPath $userLauncherPath
    Assert-Equal "installed" $installedDoctor.source_mode "installed entry doctor mode"
    Assert-Equal "warn" ($installedDoctor.checks | Where-Object id -eq "platform-version").status "installed entry should defer source freshness"
    Assert-Equal "pass" ($installedDoctor.checks | Where-Object id -eq "platform-integrity").status "installed entry integrity check"

    Add-Content -LiteralPath $userLauncherPath -Value "# stale"
    $staleLauncherDoctor = Get-RustCacheDoctor -ProjectRoot $projectRoot -SourceScriptsRoot $PSScriptRoot -CacheRoot $cacheRoot -CargoConfigPath $cargoConfig -UserLauncherPath $userLauncherPath
    Assert-Equal "fail" ($staleLauncherDoctor.checks | Where-Object id -eq "user-launcher").status "doctor stale launcher check"
    Import-Module "$modulesRoot\RustCache.Launcher.psm1" -Force -DisableNameChecking
    Install-RustCacheUserLauncher -CacheRoot $cacheRoot -UserLauncherPath $userLauncherPath | Out-Null

    Add-Content -LiteralPath (Join-Path $skillInstall.path "SKILL.md") -Value "# tampered"
    $tamperedSkillDoctor = Get-RustCacheDoctor -ProjectRoot $projectRoot -SourceScriptsRoot $PSScriptRoot -CacheRoot $cacheRoot -CargoConfigPath $cargoConfig -SourceSkillRoot $skillRoot -CodexSkillsRoot $codexSkillsRoot -UserLauncherPath $userLauncherPath
    Assert-Equal "warn" ($tamperedSkillDoctor.checks | Where-Object id -eq "codex-skill").status "doctor should detect installed skill tampering"
    Install-RustCacheCodexSkill -SourceSkillRoot $skillRoot -CodexSkillsRoot $codexSkillsRoot | Out-Null

    Add-Content -LiteralPath (Join-Path $targetModules "RustCache.Portability.psm1") -Value "# tampered"
    $tampered = Get-RustCacheDoctor -ProjectRoot $projectRoot -SourceScriptsRoot $PSScriptRoot -CacheRoot $cacheRoot -CargoConfigPath $cargoConfig -UserLauncherPath $userLauncherPath
    Assert-True (-not $tampered.healthy) "doctor should reject changed installed files"
    Assert-Equal "fail" ($tampered.checks | Where-Object id -eq "platform-integrity").status "doctor integrity check"

    $launcherStatus = & $userLauncherPath status -ProjectRoot $projectRoot -CacheRoot $cacheRoot
    Assert-Equal 0 $launcherStatus.partition_count "fresh installs should report an empty managed inventory"

    Write-Host "PASS: Rust cache portability tests ($script:Assertions assertions)." -ForegroundColor Green
} finally {
    Remove-Item -LiteralPath $tempRoot -Recurse -Force -ErrorAction SilentlyContinue
}
