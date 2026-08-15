Import-Module "$PSScriptRoot\RustCache.Paths.psm1" -Force -DisableNameChecking
Import-Module "$PSScriptRoot\RustCache.Policy.psm1" -Force -DisableNameChecking
Import-Module "$PSScriptRoot\RustCache.Launcher.psm1" -Force -DisableNameChecking

function Get-RustCacheTextHash {
    param([Parameter(Mandatory)][string]$Text)

    $sha = [System.Security.Cryptography.SHA256]::Create()
    try {
        $bytes = [System.Text.Encoding]::UTF8.GetBytes($Text)
        return ([BitConverter]::ToString($sha.ComputeHash($bytes))).Replace("-", "").ToLowerInvariant()
    } finally {
        $sha.Dispose()
    }
}

function Get-RustCachePlatformFingerprint {
    param(
        [Parameter(Mandatory)][string]$SourceScriptsRoot,
        [switch]$Installed
    )

    $root = [System.IO.Path]::GetFullPath($SourceScriptsRoot)
    $entry = Join-Path $root "rust-cache.ps1"
    $modulesRoot = Join-Path $root "rust-cache"
    if (-not (Test-Path -LiteralPath $entry -PathType Leaf) -or -not (Test-Path -LiteralPath $modulesRoot -PathType Container)) {
        throw "Rust cache platform source is incomplete: $root"
    }

    $files = @((Get-Item -LiteralPath $entry))
    $files += @(Get-ChildItem -LiteralPath $modulesRoot -File -Recurse | Where-Object {
        $_.Extension -eq ".psm1" -or (-not $Installed -and $_.Extension -eq ".rs")
    })
    $records = @($files | ForEach-Object {
        $relative = $_.FullName.Substring($root.Length).TrimStart('\', '/').Replace('\', '/')
        $fileHash = (Get-FileHash -LiteralPath $_.FullName -Algorithm SHA256).Hash.ToLowerInvariant()
        "{0}:{1}:{2}" -f $relative, $_.Length, $fileHash
    } | Sort-Object)
    [pscustomobject]@{
        hash = Get-RustCacheTextHash -Text ($records -join "`n")
        file_count = $records.Count
        files = $records
    }
}

function Get-RustCachePlatformInstallManifestPath {
    param([Parameter(Mandatory)][string]$CacheRoot)
    Join-Path ([System.IO.Path]::GetFullPath($CacheRoot)) "platform\platform-install.json"
}

function Read-RustCachePlatformInstallManifest {
    param([Parameter(Mandatory)][string]$CacheRoot)

    $path = Get-RustCachePlatformInstallManifestPath -CacheRoot $CacheRoot
    if (-not (Test-Path -LiteralPath $path -PathType Leaf)) {
        return $null
    }
    try {
        $manifest = Get-Content -Raw -LiteralPath $path -Encoding UTF8 | ConvertFrom-Json
    } catch {
        throw "Invalid Rust cache platform install manifest at $path. $($_.Exception.Message)"
    }
    if ($manifest.schema -ne "elon.rust_cache.platform_install.v1") {
        throw "Unsupported Rust cache platform install manifest schema '$($manifest.schema)' at $path."
    }
    return $manifest
}

function Write-RustCachePlatformInstallManifest {
    param(
        [Parameter(Mandatory)][string]$CacheRoot,
        [Parameter(Mandatory)]$SourceFingerprint,
        [Parameter(Mandatory)]$InstalledFingerprint
    )

    $path = Get-RustCachePlatformInstallManifestPath -CacheRoot $CacheRoot
    $payload = [ordered]@{
        schema = "elon.rust_cache.platform_install.v1"
        tool_version = 1
        installed_at_utc = [DateTime]::UtcNow.ToString("o")
        source_hash = [string]$SourceFingerprint.hash
        source_file_count = [int]$SourceFingerprint.file_count
        installed_hash = [string]$InstalledFingerprint.hash
        installed_file_count = [int]$InstalledFingerprint.file_count
        entry_relative_path = "platform/rust-cache.ps1"
    }
    $temporary = "$path.$PID.tmp"
    $payload | ConvertTo-Json -Depth 6 | Set-Content -LiteralPath $temporary -Encoding UTF8
    Move-Item -LiteralPath $temporary -Destination $path -Force
    return $path
}

function Assert-RustCachePortableSlug {
    param([Parameter(Mandatory)][string]$Value, [Parameter(Mandatory)][string]$Field)

    $slug = ConvertTo-RustCacheSlug $Value
    if ($slug -eq "unknown" -or $slug -cne $Value) {
        throw "$Field must be a lowercase stable slug containing only letters, digits, '.', '_' or '-': $Value"
    }
    return $slug
}

function New-RustCacheProjectManifest {
    param(
        [Parameter(Mandatory)][string]$ProjectRoot,
        [Parameter(Mandatory)][string]$ProjectId,
        [string]$DefaultDomain = "dev-windows-msvc",
        [string[]]$AllowedDomains = @(),
        [string]$UnknownDomainFallback = "agent-validation",
        [string[]]$SharedPartitionDomains = @(),
        [switch]$Apply
    )

    $root = [System.IO.Path]::GetFullPath($ProjectRoot)
    if (-not (Test-Path -LiteralPath $root -PathType Container)) {
        throw "Project root does not exist: $root"
    }
    $projectSlug = Assert-RustCachePortableSlug -Value $ProjectId -Field "ProjectId"
    $defaultSlug = Assert-RustCachePortableSlug -Value $DefaultDomain -Field "DefaultDomain"
    $fallbackSlug = Assert-RustCachePortableSlug -Value $UnknownDomainFallback -Field "UnknownDomainFallback"
    $allowed = @($AllowedDomains + @($defaultSlug, $fallbackSlug) | ForEach-Object {
        Assert-RustCachePortableSlug -Value ([string]$_) -Field "AllowedDomain"
    } | Sort-Object -Unique)

    $mappingValues = @{}
    foreach ($item in @($SharedPartitionDomains)) {
        $parts = @(([string]$item) -split '=', 2)
        if ($parts.Count -ne 2 -or [string]::IsNullOrWhiteSpace($parts[0]) -or [string]::IsNullOrWhiteSpace($parts[1])) {
            throw "SharedPartitionDomain must use partition=domain syntax: $item"
        }
        $partition = Assert-RustCachePortableSlug -Value $parts[0] -Field "Shared partition"
        $mappedDomain = Assert-RustCachePortableSlug -Value $parts[1] -Field "Shared partition domain"
        if ($mappedDomain -notin $allowed) {
            throw "Shared partition domain must be listed in AllowedDomains: $mappedDomain"
        }
        if ($mappingValues.ContainsKey($partition)) {
            throw "Duplicate shared partition mapping: $partition"
        }
        $mappingValues[$partition] = $mappedDomain
    }
    $mapping = [ordered]@{}
    foreach ($partition in @($mappingValues.Keys | Sort-Object)) {
        $mapping[$partition] = $mappingValues[$partition]
    }

    $payload = [ordered]@{
        schema_version = 1
        project_id = $projectSlug
        default_domain = $defaultSlug
        allowed_domains = $allowed
        unknown_domain_fallback = $fallbackSlug
        shared_partition_domains = $mapping
    }
    $path = Join-Path $root "rust-cache.project.json"
    $content = ($payload | ConvertTo-Json -Depth 8) + [Environment]::NewLine
    if (Test-Path -LiteralPath $path -PathType Leaf) {
        $existing = Get-RustCacheProjectManifest -ProjectRoot $root
        $existingMapping = [ordered]@{}
        foreach ($key in @($existing.shared_partition_domains.Keys | Sort-Object)) {
            $existingMapping[$key] = [string]$existing.shared_partition_domains[$key]
        }
        $existingNormalized = [ordered]@{
            schema_version = 1
            project_id = [string]$existing.project_id
            default_domain = [string]$existing.default_domain
            allowed_domains = @($existing.allowed_domains | Sort-Object -Unique)
            unknown_domain_fallback = [string]$existing.unknown_domain_fallback
            shared_partition_domains = $existingMapping
        } | ConvertTo-Json -Depth 8 -Compress
        $requestedNormalized = $payload | ConvertTo-Json -Depth 8 -Compress
        if ($existingNormalized -ne $requestedNormalized) {
            throw "Project cache manifest already exists with different settings; review it instead of overwriting: $path"
        }
        return [pscustomobject]@{ action = "unchanged"; applied = $false; path = $path; content = $content; manifest = [pscustomobject]$payload }
    }

    if ($Apply) {
        $temporary = "$path.$PID.tmp"
        Set-Content -LiteralPath $temporary -Value $content -Encoding UTF8 -NoNewline
        Move-Item -LiteralPath $temporary -Destination $path -Force
    }
    [pscustomobject]@{
        action = if ($Apply) { "created" } else { "would-create" }
        applied = [bool]$Apply
        path = $path
        content = $content
        manifest = [pscustomobject]$payload
    }
}

function Get-RustCacheCodexSkillsRoot {
    param([string]$ExplicitRoot)

    if (-not [string]::IsNullOrWhiteSpace($ExplicitRoot)) {
        return [System.IO.Path]::GetFullPath($ExplicitRoot)
    }
    if (-not [string]::IsNullOrWhiteSpace($env:CODEX_HOME)) {
        return Join-Path $env:CODEX_HOME "skills"
    }
    $homeRoot = if (-not [string]::IsNullOrWhiteSpace($env:USERPROFILE)) { $env:USERPROFILE } else { $env:HOME }
    if ([string]::IsNullOrWhiteSpace($homeRoot)) {
        throw "Cannot resolve the Codex skills directory. Set CODEX_HOME or pass CodexSkillsRoot."
    }
    return Join-Path $homeRoot ".codex\skills"
}

function Get-RustCacheCodexSkillFingerprint {
    param([Parameter(Mandatory)][string]$SkillRoot)

    $root = [System.IO.Path]::GetFullPath($SkillRoot)
    $skillPath = Join-Path $root "SKILL.md"
    $agentPath = Join-Path $root "agents\openai.yaml"
    if (-not (Test-Path -LiteralPath $skillPath -PathType Leaf) -or -not (Test-Path -LiteralPath $agentPath -PathType Leaf)) {
        throw "Cache management skill is incomplete: $root"
    }
    return Get-RustCacheTextHash -Text ((Get-FileHash $skillPath -Algorithm SHA256).Hash + ":" + (Get-FileHash $agentPath -Algorithm SHA256).Hash)
}

function Install-RustCacheCodexSkill {
    param(
        [Parameter(Mandatory)][string]$SourceSkillRoot,
        [string]$CodexSkillsRoot
    )

    $source = [System.IO.Path]::GetFullPath($SourceSkillRoot)
    $sourceSkill = Join-Path $source "SKILL.md"
    $sourceAgent = Join-Path $source "agents\openai.yaml"
    if (-not (Test-Path -LiteralPath $sourceSkill -PathType Leaf) -or -not (Test-Path -LiteralPath $sourceAgent -PathType Leaf)) {
        throw "Cache management skill source is incomplete: $source"
    }
    $destination = Join-Path (Get-RustCacheCodexSkillsRoot -ExplicitRoot $CodexSkillsRoot) "manage-shared-build-cache"
    New-Item -ItemType Directory -Force -Path (Join-Path $destination "agents") | Out-Null
    Copy-Item -LiteralPath $sourceSkill -Destination (Join-Path $destination "SKILL.md") -Force
    Copy-Item -LiteralPath $sourceAgent -Destination (Join-Path $destination "agents\openai.yaml") -Force
    $fingerprint = Get-RustCacheCodexSkillFingerprint -SkillRoot $source
    $marker = Join-Path $destination ".elon-install.json"
    [ordered]@{
        schema = "elon.rust_cache.codex_skill_install.v1"
        installed_at_utc = [DateTime]::UtcNow.ToString("o")
        source_hash = $fingerprint
    } | ConvertTo-Json | Set-Content -LiteralPath $marker -Encoding UTF8
    [pscustomobject]@{ path = $destination; marker_path = $marker; source_hash = $fingerprint }
}

function New-RustCacheDoctorCheck {
    param(
        [Parameter(Mandatory)][string]$Id,
        [Parameter(Mandatory)][ValidateSet("pass", "warn", "fail")][string]$Status,
        [Parameter(Mandatory)][string]$Message,
        [AllowNull()][string]$Remediation
    )
    [pscustomobject]@{ id = $Id; status = $Status; message = $Message; remediation = $Remediation }
}

function Get-RustCacheDoctor {
    param(
        [Parameter(Mandatory)][string]$ProjectRoot,
        [Parameter(Mandatory)][string]$SourceScriptsRoot,
        [string]$CacheRoot,
        [string]$CargoConfigPath,
        [string]$SourceSkillRoot,
        [string]$CodexSkillsRoot,
        [string]$UserLauncherPath
    )

    $project = [System.IO.Path]::GetFullPath($ProjectRoot)
    $source = [System.IO.Path]::GetFullPath($SourceScriptsRoot)
    $root = Resolve-RustCacheRoot -ExplicitRoot $CacheRoot -RepoRoot $project -NoCreate
    if ([string]::IsNullOrWhiteSpace($CargoConfigPath)) {
        $CargoConfigPath = Get-RustCacheDefaultCargoConfigPath
    }
    $cargoConfig = [System.IO.Path]::GetFullPath($CargoConfigPath)
    $checks = New-Object System.Collections.Generic.List[object]

    if (Test-Path -LiteralPath $project -PathType Container) {
        try {
            $projectManifest = Get-RustCacheProjectManifest -ProjectRoot $project
            if ($projectManifest.registered) {
                $checks.Add((New-RustCacheDoctorCheck "project-manifest" "pass" "Registered project '$($projectManifest.project_id)'." $null))
            } else {
                $checks.Add((New-RustCacheDoctorCheck "project-manifest" "fail" "Project has no rust-cache.project.json and is quarantined." "Run init-project, review the preview, then repeat with -Apply."))
            }
        } catch {
            $checks.Add((New-RustCacheDoctorCheck "project-manifest" "fail" $_.Exception.Message "Repair rust-cache.project.json before using shared partitions."))
        }
    } else {
        $checks.Add((New-RustCacheDoctorCheck "project-root" "fail" "Project root does not exist: $project" "Pass an existing absolute ProjectRoot."))
    }

    foreach ($command in @("git", "cargo", "rustc")) {
        $available = Get-Command $command -ErrorAction SilentlyContinue
        $checks.Add((New-RustCacheDoctorCheck "command-$command" $(if ($available) { "pass" } else { "fail" }) $(if ($available) { "$command is available at $($available.Source)." } else { "$command is not available." }) $(if ($available) { $null } else { "Install $command or add it to PATH." })))
    }
    $sccache = Get-Command sccache -ErrorAction SilentlyContinue
    $checks.Add((New-RustCacheDoctorCheck "command-sccache" $(if ($sccache) { "pass" } else { "warn" }) $(if ($sccache) { "sccache is available at $($sccache.Source)." } else { "sccache is unavailable; builds work but cross-project object reuse is disabled." }) $(if ($sccache) { $null } else { "Install sccache before activating the platform for best reuse." })))

    $platformRoot = Join-Path $root "platform"
    $sourceComparison = if ($env:OS -eq "Windows_NT") { [System.StringComparison]::OrdinalIgnoreCase } else { [System.StringComparison]::Ordinal }
    $sourceIsInstalledEntry = $source.TrimEnd('\', '/').Equals($platformRoot.TrimEnd('\', '/'), $sourceComparison)
    $sourceFingerprint = Get-RustCachePlatformFingerprint -SourceScriptsRoot $source -Installed:$sourceIsInstalledEntry
    $installManifest = $null
    $installManifestError = $null
    try { $installManifest = Read-RustCachePlatformInstallManifest -CacheRoot $root } catch { $installManifestError = $_.Exception.Message }
    if ($installManifestError) {
        $checks.Add((New-RustCacheDoctorCheck "platform-install" "fail" $installManifestError "Re-run install from a trusted current checkout."))
    } elseif ($null -eq $installManifest) {
        $checks.Add((New-RustCacheDoctorCheck "platform-install" "fail" "Managed platform is not installed at $root." "Run install from the authoritative repository checkout."))
    } else {
        $checks.Add((New-RustCacheDoctorCheck "platform-install" "pass" "Managed platform install manifest is present." $null))
        if ($sourceIsInstalledEntry) {
            $checks.Add((New-RustCacheDoctorCheck "platform-version" "warn" "Doctor is running from the installed entry; source freshness was not compared." "Run doctor from a current trusted platform checkout to check for upgrades."))
        } elseif ([string]$installManifest.source_hash -eq [string]$sourceFingerprint.hash) {
            $checks.Add((New-RustCacheDoctorCheck "platform-version" "pass" "Installed platform matches this checkout." $null))
        } else {
            $checks.Add((New-RustCacheDoctorCheck "platform-version" "fail" "Installed platform differs from this checkout." "Re-run install from this checkout before starting new builds."))
        }
        try {
            $installedFingerprint = Get-RustCachePlatformFingerprint -SourceScriptsRoot (Join-Path $root "platform") -Installed
            if ([string]$installManifest.installed_hash -eq [string]$installedFingerprint.hash) {
                $checks.Add((New-RustCacheDoctorCheck "platform-integrity" "pass" "Installed platform files match their recorded fingerprint." $null))
            } else {
                $checks.Add((New-RustCacheDoctorCheck "platform-integrity" "fail" "Installed platform files changed after installation." "Re-run install from a trusted checkout."))
            }
        } catch {
            $checks.Add((New-RustCacheDoctorCheck "platform-integrity" "fail" $_.Exception.Message "Re-run install from a trusted checkout."))
        }
    }

    $includePath = Join-Path $root "config\cargo-cache.toml"
    if (Test-Path -LiteralPath $includePath -PathType Leaf) {
        $checks.Add((New-RustCacheDoctorCheck "cargo-include" "pass" "Managed Cargo include exists." $null))
    } else {
        $checks.Add((New-RustCacheDoctorCheck "cargo-include" "fail" "Managed Cargo include is missing: $includePath" "Run install before building."))
    }
    if (Test-Path -LiteralPath $cargoConfig -PathType Leaf) {
        $cargoContent = Get-Content -Raw -LiteralPath $cargoConfig -Encoding UTF8
        $includeToml = $includePath.Replace('\', '/')
        $pathComparison = if ($env:OS -eq "Windows_NT") { [System.StringComparison]::OrdinalIgnoreCase } else { [System.StringComparison]::Ordinal }
        $active = $cargoContent.IndexOf($includeToml, $pathComparison) -ge 0
        $checks.Add((New-RustCacheDoctorCheck "cargo-activation" $(if ($active) { "pass" } else { "warn" }) $(if ($active) { "User Cargo config activates the managed include." } else { "User Cargo config does not activate the managed include." }) $(if ($active) { $null } else { "Run install with -Apply or use only project-managed Cargo entry scripts." })))
    } else {
        $checks.Add((New-RustCacheDoctorCheck "cargo-activation" "warn" "User Cargo config does not exist: $cargoConfig" "Run install with -Apply to create and activate it."))
    }

    $launcher = Test-RustCacheUserLauncher -CacheRoot $root -UserLauncherPath $UserLauncherPath
    if ($launcher.healthy) {
        $checks.Add((New-RustCacheDoctorCheck "user-launcher" "pass" "Portable user launcher points to this installed cache platform." $null))
    } else {
        $checks.Add((New-RustCacheDoctorCheck "user-launcher" "fail" "Portable user launcher is $($launcher.status): $($launcher.path)" "Re-run install from a trusted current checkout."))
    }

    $drive = New-Object System.IO.DriveInfo ([System.IO.Path]::GetPathRoot($root))
    $freePercent = if ($drive.TotalSize -gt 0) { [math]::Round(100 * $drive.AvailableFreeSpace / $drive.TotalSize, 2) } else { 0 }
    $policyPath = Get-RustCachePolicyPath -CacheRoot $root
    $policy = if (Test-Path -LiteralPath $policyPath -PathType Leaf) { Get-RustCachePolicy -CacheRoot $root } else { Get-DefaultRustCachePolicy }
    $diskStatus = if ($freePercent -lt [double]$policy.critical_free_percent) { "fail" } elseif ($freePercent -lt [double]$policy.warning_free_percent) { "warn" } else { "pass" }
    $checks.Add((New-RustCacheDoctorCheck "disk-space" $diskStatus "Cache volume has $freePercent% free space." $(if ($diskStatus -eq "pass") { $null } else { "Run gc without -Apply, review the report, then apply only selected managed cleanup." })))

    $writers = @(Get-Process -Name cargo, rustc -ErrorAction SilentlyContinue | Select-Object ProcessName, Id, StartTime)
    $checks.Add((New-RustCacheDoctorCheck "active-writers" $(if ($writers.Count -eq 0) { "pass" } else { "warn" }) $(if ($writers.Count -eq 0) { "No active Cargo/rustc writers were found." } else { "$($writers.Count) Cargo/rustc writer process(es) are active." }) $(if ($writers.Count -eq 0) { $null } else { "Do not start duplicate builds or activate global Cargo configuration until they finish." })))

    $skillPath = $null
    if (-not [string]::IsNullOrWhiteSpace($SourceSkillRoot) -and (Test-Path -LiteralPath $SourceSkillRoot -PathType Container)) {
        $installedSkillRoot = Join-Path (Get-RustCacheCodexSkillsRoot -ExplicitRoot $CodexSkillsRoot) "manage-shared-build-cache"
        $skillPath = Join-Path $installedSkillRoot "SKILL.md"
        $skillMarkerPath = Join-Path $installedSkillRoot ".elon-install.json"
        if (-not (Test-Path -LiteralPath $skillPath -PathType Leaf)) {
            $checks.Add((New-RustCacheDoctorCheck "codex-skill" "warn" "Codex cache management skill is not installed for this user." "Re-run install with -InstallCodexSkill."))
        } else {
            $expectedSkillHash = Get-RustCacheCodexSkillFingerprint -SkillRoot $SourceSkillRoot
            $installedSkillHash = $null
            if (Test-Path -LiteralPath $skillMarkerPath -PathType Leaf) {
                try {
                    $skillMarker = Get-Content -Raw -LiteralPath $skillMarkerPath -Encoding UTF8 | ConvertFrom-Json
                    if ($skillMarker.schema -eq "elon.rust_cache.codex_skill_install.v1") {
                        $installedSkillHash = [string]$skillMarker.source_hash
                    }
                } catch { $installedSkillHash = $null }
            }
            if ($expectedSkillHash -eq $installedSkillHash) {
                $checks.Add((New-RustCacheDoctorCheck "codex-skill" "pass" "Codex cache management skill matches this checkout." $null))
            } else {
                $checks.Add((New-RustCacheDoctorCheck "codex-skill" "warn" "Codex cache management skill is unverified or stale." "Re-run install with -InstallCodexSkill."))
            }
        }
    }

    $failCount = @($checks | Where-Object { $_.status -eq "fail" }).Count
    $warnCount = @($checks | Where-Object { $_.status -eq "warn" }).Count
    [pscustomobject]@{
        schema = "elon.rust_cache.doctor.v1"
        status = if ($failCount -gt 0) { "action-required" } elseif ($warnCount -gt 0) { "warning" } else { "healthy" }
        healthy = $failCount -eq 0
        project_root = $project
        cache_root = $root
        cargo_config_path = $cargoConfig
        source_hash = $sourceFingerprint.hash
        source_mode = if ($sourceIsInstalledEntry) { "installed" } else { "repository" }
        user_launcher_path = $launcher.path
        checks = @($checks | ForEach-Object { $_ })
        active_writers = $writers
        destructive_actions_taken = $false
    }
}

Export-ModuleMember -Function Get-RustCachePlatformFingerprint, Get-RustCachePlatformInstallManifestPath, Read-RustCachePlatformInstallManifest, Write-RustCachePlatformInstallManifest, New-RustCacheProjectManifest, Get-RustCacheCodexSkillsRoot, Get-RustCacheCodexSkillFingerprint, Install-RustCacheCodexSkill, Get-RustCacheDoctor
