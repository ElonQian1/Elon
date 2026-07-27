Import-Module "$PSScriptRoot\RustCache.Paths.psm1" -Force -DisableNameChecking

function Get-DefaultRustCachePolicy {
    [pscustomobject]@{
        schema_version = 1
        warning_free_percent = 15
        recovery_free_percent = 20
        critical_free_percent = 8
        partition_ttl_days = 14
        old_epoch_ttl_days = 7
        sccache_max_size = "20G"
        auto_gc_on_run = $true
        legacy_caches = @()
    }
}

function Get-RustCachePolicyPath {
    param([Parameter(Mandatory)][string]$CacheRoot)
    Join-Path $CacheRoot "config\policy.json"
}

function Initialize-RustCachePolicy {
    param([Parameter(Mandatory)][string]$CacheRoot)

    $policyPath = Get-RustCachePolicyPath -CacheRoot $CacheRoot
    if (Test-Path -LiteralPath $policyPath) {
        return $policyPath
    }
    New-Item -ItemType Directory -Force -Path (Split-Path $policyPath -Parent) | Out-Null
    $payload = Get-DefaultRustCachePolicy | ConvertTo-Json -Depth 8
    Set-Content -LiteralPath $policyPath -Value $payload -Encoding UTF8
    return $policyPath
}

function Get-RustCachePolicy {
    param([Parameter(Mandatory)][string]$CacheRoot)

    $policyPath = Initialize-RustCachePolicy -CacheRoot $CacheRoot
    try {
        $policy = Get-Content -Raw -LiteralPath $policyPath -Encoding UTF8 | ConvertFrom-Json
    } catch {
        throw "Invalid Rust cache policy JSON at $policyPath. $($_.Exception.Message)"
    }
    if ($policy.schema_version -ne 1) {
        throw "Unsupported Rust cache policy schema_version '$($policy.schema_version)' at $policyPath."
    }
    foreach ($field in @("warning_free_percent", "recovery_free_percent", "critical_free_percent", "partition_ttl_days", "old_epoch_ttl_days")) {
        if ($null -eq $policy.$field) {
            throw "Rust cache policy is missing '$field': $policyPath"
        }
    }
    if ($policy.critical_free_percent -ge $policy.warning_free_percent) {
        throw "critical_free_percent must be lower than warning_free_percent."
    }
    if ($policy.recovery_free_percent -le $policy.warning_free_percent) {
        throw "recovery_free_percent must be greater than warning_free_percent."
    }
    return $policy
}

function Get-RustCacheProjectManifest {
    param([Parameter(Mandatory)][string]$ProjectRoot)

    if (-not (Test-RustCacheAbsolutePath $ProjectRoot)) {
        throw "Project root must be absolute: $ProjectRoot"
    }
    $manifestPath = Join-Path $ProjectRoot "rust-cache.project.json"
    if (-not (Test-Path -LiteralPath $manifestPath)) {
        return [pscustomobject]@{
            schema_version = 1
            project_id = "quarantine-$(Get-RustCacheWorkspaceHash -WorkspaceRoot $ProjectRoot)"
            default_domain = "unregistered"
            allowed_domains = @()
            unknown_domain_fallback = "unregistered"
            registered = $false
            manifest_path = $null
        }
    }
    try {
        $manifest = Get-Content -Raw -LiteralPath $manifestPath -Encoding UTF8 | ConvertFrom-Json
    } catch {
        throw "Invalid Rust cache project manifest at $manifestPath. $($_.Exception.Message)"
    }
    if ($manifest.schema_version -ne 1) {
        throw "Unsupported Rust cache project schema_version '$($manifest.schema_version)' at $manifestPath."
    }
    $projectId = ConvertTo-RustCacheSlug ([string]$manifest.project_id)
    if ([string]::IsNullOrWhiteSpace($manifest.project_id) -or $projectId -eq "unknown") {
        throw "rust-cache.project.json must define a non-empty project_id: $manifestPath"
    }
    $domain = ConvertTo-RustCacheSlug ([string]$manifest.default_domain)
    $allowedDomains = @()
    if ($null -ne $manifest.PSObject.Properties["allowed_domains"]) {
        $allowedDomains = @($manifest.allowed_domains |
            ForEach-Object { ConvertTo-RustCacheSlug ([string]$_) } |
            Where-Object { -not [string]::IsNullOrWhiteSpace($_) -and $_ -ne "unknown" } |
            Sort-Object -Unique)
    }
    $fallbackDomain = if ($null -ne $manifest.PSObject.Properties["unknown_domain_fallback"]) {
        ConvertTo-RustCacheSlug ([string]$manifest.unknown_domain_fallback)
    } else {
        $domain
    }
    if ($allowedDomains.Count -gt 0) {
        if ($domain -notin $allowedDomains) {
            throw "rust-cache.project.json default_domain must be listed in allowed_domains: $manifestPath"
        }
        if ($fallbackDomain -notin $allowedDomains) {
            throw "rust-cache.project.json unknown_domain_fallback must be listed in allowed_domains: $manifestPath"
        }
    }
    return [pscustomobject]@{
        schema_version = 1
        project_id = $projectId
        default_domain = $domain
        allowed_domains = $allowedDomains
        unknown_domain_fallback = $fallbackDomain
        registered = $true
        manifest_path = $manifestPath
    }
}

function Resolve-RustCacheDomain {
    param(
        [Parameter(Mandatory)][string]$ProjectRoot,
        [string]$Domain,
        [AllowNull()]$Manifest
    )

    $projectManifest = if ($null -eq $Manifest) {
        Get-RustCacheProjectManifest -ProjectRoot $ProjectRoot
    } else {
        $Manifest
    }
    $requested = if ([string]::IsNullOrWhiteSpace($Domain)) {
        [string]$projectManifest.default_domain
    } else {
        ConvertTo-RustCacheSlug $Domain
    }
    $allowed = @($projectManifest.allowed_domains)
    if (-not $projectManifest.registered -or $allowed.Count -eq 0 -or $requested -in $allowed) {
        return $requested
    }
    return [string]$projectManifest.unknown_domain_fallback
}

function Add-RustCacheLegacyRecord {
    param(
        [Parameter(Mandatory)][string]$CacheRoot,
        [Parameter(Mandatory)][string]$Path,
        [Parameter(Mandatory)][string]$Label,
        [switch]$Retired
    )

    if (-not (Test-RustCacheAbsolutePath $Path)) {
        throw "Legacy cache path must be absolute: $Path"
    }
    $policyPath = Initialize-RustCachePolicy -CacheRoot $CacheRoot
    $policy = Get-RustCachePolicy -CacheRoot $CacheRoot
    $records = @($policy.legacy_caches)
    $fullPath = [System.IO.Path]::GetFullPath($Path)
    $existing = $records | Where-Object { [string]$_.path -ieq $fullPath } | Select-Object -First 1
    if ($existing) {
        $existing.label = $Label
        $existing.retired = [bool]$Retired
    } else {
        $records += [pscustomobject]@{
            path = $fullPath
            label = $Label
            retired = [bool]$Retired
            managed = $false
            registered_utc = [DateTime]::UtcNow.ToString("o")
        }
    }
    $policy.legacy_caches = $records
    $policy | ConvertTo-Json -Depth 8 | Set-Content -LiteralPath $policyPath -Encoding UTF8
    return $policyPath
}

Export-ModuleMember -Function Get-DefaultRustCachePolicy, Get-RustCachePolicyPath, Initialize-RustCachePolicy, Get-RustCachePolicy, Get-RustCacheProjectManifest, Resolve-RustCacheDomain, Add-RustCacheLegacyRecord
