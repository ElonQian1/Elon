[CmdletBinding()]
param(
    [string]$ProjectRoot,
    [string]$CodexHome,
    [string]$NodeAdminUrl,
    [switch]$RequireInstalled,
    [switch]$RequireRuntime
)

$ErrorActionPreference = "Stop"

function Normalize-PathValue {
    param([string]$Value)
    if ([string]::IsNullOrWhiteSpace($Value)) { return "" }
    return [System.IO.Path]::GetFullPath($Value).TrimEnd([char[]]"\/").ToLowerInvariant()
}

function Get-PluginFileMap {
    param([string]$Root)
    $result = @{}
    if (-not (Test-Path -LiteralPath $Root -PathType Container)) { return $result }
    foreach ($file in Get-ChildItem -LiteralPath $Root -Recurse -File | Sort-Object FullName) {
        $relative = $file.FullName.Substring($Root.Length).TrimStart([char[]]"\/").Replace('\', '/')
        $result[$relative] = (Get-FileHash -Algorithm SHA256 -LiteralPath $file.FullName).Hash.ToLowerInvariant()
    }
    return $result
}

function Test-InstalledPluginCopy {
    param(
        [hashtable]$SourceFiles,
        [string]$InstalledRoot
    )
    $mismatches = New-Object System.Collections.Generic.List[string]
    foreach ($relative in $SourceFiles.Keys) {
        $installedFile = Join-Path $InstalledRoot ($relative.Replace('/', '\'))
        if (-not (Test-Path -LiteralPath $installedFile -PathType Leaf)) {
            $mismatches.Add("missing:$relative") | Out-Null
            continue
        }
        $hash = (Get-FileHash -Algorithm SHA256 -LiteralPath $installedFile).Hash.ToLowerInvariant()
        if ($hash -ne $SourceFiles[$relative]) {
            $mismatches.Add("changed:$relative") | Out-Null
        }
    }
    return [pscustomobject]@{
        exact = ($mismatches.Count -eq 0)
        mismatch_count = $mismatches.Count
        mismatches = @($mismatches | Select-Object -First 16)
    }
}

function Read-CodexConfigState {
    param(
        [string]$ConfigPath,
        [string]$ExpectedProject,
        [string]$PluginName,
        [string]$MarketplaceName
    )
    $result = [ordered]@{
        config_present = $false
        project_configured = $false
        project_trust_level = "unknown"
        plugin_configured = $false
        plugin_enabled = $false
        plugin_config_key = ""
    }
    if (-not (Test-Path -LiteralPath $ConfigPath -PathType Leaf)) {
        return [pscustomobject]$result
    }
    $result.config_present = $true
    $sectionKind = ""
    $sectionValue = ""
    foreach ($line in Get-Content -LiteralPath $ConfigPath) {
        if ($line -match '^\s*\[projects\.(?:''([^'']+)''|"([^"]+)")\]\s*$') {
            $sectionKind = "project"
            $sectionValue = if ($Matches[1]) { $Matches[1] } else { $Matches[2] }
            continue
        }
        if ($line -match '^\s*\[plugins\.(?:''([^'']+)''|"([^"]+)")\]\s*$') {
            $sectionKind = "plugin"
            $sectionValue = if ($Matches[1]) { $Matches[1] } else { $Matches[2] }
            continue
        }
        if ($line -match '^\s*\[') {
            $sectionKind = ""
            $sectionValue = ""
            continue
        }
        if ($sectionKind -eq "project" -and
            (Normalize-PathValue $sectionValue) -eq $ExpectedProject -and
            $line -match '^\s*trust_level\s*=\s*["'']([^"'']+)["'']\s*$') {
            $result.project_configured = $true
            $result.project_trust_level = $Matches[1].Trim().ToLowerInvariant()
        }
        if ($sectionKind -eq "plugin" -and
            ($sectionValue -eq "$PluginName@$MarketplaceName" -or $sectionValue -like "$PluginName@*") -and
            $line -match '^\s*enabled\s*=\s*(true|false)\s*$') {
            $result.plugin_configured = $true
            $result.plugin_enabled = $Matches[1] -eq 'true'
            $result.plugin_config_key = $sectionValue
        }
    }
    return [pscustomobject]$result
}

function Find-NodeAdminApi {
    param([string]$RequestedUrl)
    $candidates = New-Object System.Collections.Generic.List[string]
    if (-not [string]::IsNullOrWhiteSpace($RequestedUrl)) {
        $candidates.Add($RequestedUrl.TrimEnd('/')) | Out-Null
    } else {
        7799..7819 | ForEach-Object { $candidates.Add("http://127.0.0.1:$_") | Out-Null }
    }
    foreach ($candidate in $candidates) {
        if ($candidate -notmatch '^http://127\.0\.0\.1:\d{1,5}$') { continue }
        try {
            $response = Invoke-WebRequest -UseBasicParsing -Uri "$candidate/api/health" -TimeoutSec 1
            if ($response.StatusCode -ge 200 -and $response.StatusCode -lt 300) {
                return $candidate
            }
        } catch {
            # Continue bounded loopback discovery without surfacing response bodies.
        }
    }
    return ""
}

if ([string]::IsNullOrWhiteSpace($ProjectRoot)) {
    $ProjectRoot = Split-Path -Parent $PSScriptRoot
}
$ProjectRoot = [System.IO.Path]::GetFullPath($ProjectRoot)
if ([string]::IsNullOrWhiteSpace($CodexHome)) {
    if (-not [string]::IsNullOrWhiteSpace($env:CODEX_HOME)) {
        $CodexHome = $env:CODEX_HOME
    } else {
        $CodexHome = Join-Path $env:USERPROFILE '.codex'
    }
}
$CodexHome = [System.IO.Path]::GetFullPath($CodexHome)

$marketplacePath = Join-Path $ProjectRoot '.agents\plugins\marketplace.json'
$pluginRoot = Join-Path $ProjectRoot 'plugins\yilong-project-memory'
$manifestPath = Join-Path $pluginRoot '.codex-plugin\plugin.json'
$mcpPath = Join-Path $pluginRoot '.mcp.json'
$hooksPath = Join-Path $pluginRoot 'hooks\hooks.json'
$staticErrors = New-Object System.Collections.Generic.List[string]

$marketplace = $null
$manifest = $null
$mcp = $null
$hooks = $null
foreach ($entry in @(
    @{ Name = 'marketplace'; Path = $marketplacePath },
    @{ Name = 'plugin_manifest'; Path = $manifestPath },
    @{ Name = 'mcp_manifest'; Path = $mcpPath },
    @{ Name = 'hooks_manifest'; Path = $hooksPath }
)) {
    if (-not (Test-Path -LiteralPath $entry.Path -PathType Leaf)) {
        $staticErrors.Add("missing:$($entry.Name)") | Out-Null
    }
}
try { if (Test-Path -LiteralPath $marketplacePath) { $marketplace = Get-Content -Raw -LiteralPath $marketplacePath | ConvertFrom-Json } } catch { $staticErrors.Add('invalid_json:marketplace') | Out-Null }
try { if (Test-Path -LiteralPath $manifestPath) { $manifest = Get-Content -Raw -LiteralPath $manifestPath | ConvertFrom-Json } } catch { $staticErrors.Add('invalid_json:plugin_manifest') | Out-Null }
try { if (Test-Path -LiteralPath $mcpPath) { $mcp = Get-Content -Raw -LiteralPath $mcpPath | ConvertFrom-Json } } catch { $staticErrors.Add('invalid_json:mcp_manifest') | Out-Null }
try { if (Test-Path -LiteralPath $hooksPath) { $hooks = Get-Content -Raw -LiteralPath $hooksPath | ConvertFrom-Json } } catch { $staticErrors.Add('invalid_json:hooks_manifest') | Out-Null }

$marketplaceName = if ($marketplace -and $marketplace.name) { [string]$marketplace.name } else { "" }
$pluginName = if ($manifest -and $manifest.name) { [string]$manifest.name } else { "" }
$sourceEntry = @($marketplace.plugins | Where-Object { $_.name -eq $pluginName }) | Select-Object -First 1
if (-not $sourceEntry) { $staticErrors.Add('marketplace_plugin_entry_missing') | Out-Null }
if ($pluginName -ne 'yilong-project-memory') { $staticErrors.Add('plugin_name_mismatch') | Out-Null }
if ([string]::IsNullOrWhiteSpace($marketplaceName)) { $staticErrors.Add('marketplace_name_missing') | Out-Null }
if ($manifest -and ([string]$manifest.version -notmatch '^(0|[1-9]\d*)\.(0|[1-9]\d*)\.(0|[1-9]\d*)(?:-[0-9A-Za-z-]+(?:\.[0-9A-Za-z-]+)*)?(?:\+[0-9A-Za-z-]+(?:\.[0-9A-Za-z-]+)*)?$')) { $staticErrors.Add('plugin_version_not_semver') | Out-Null }
if ($manifest -and $manifest.mcpServers -ne './.mcp.json') { $staticErrors.Add('plugin_mcp_manifest_path_mismatch') | Out-Null }
if ($sourceEntry) {
    if ($sourceEntry.source.source -ne 'local') { $staticErrors.Add('marketplace_source_not_local') | Out-Null }
    if ([string]$sourceEntry.source.path -ne './plugins/yilong-project-memory') { $staticErrors.Add('marketplace_source_path_mismatch') | Out-Null }
    if ($sourceEntry.policy.installation -ne 'AVAILABLE') { $staticErrors.Add('marketplace_installation_policy_mismatch') | Out-Null }
    if (@('ON_INSTALL', 'ON_USE') -notcontains $sourceEntry.policy.authentication) { $staticErrors.Add('marketplace_authentication_policy_invalid') | Out-Null }
    if ([string]::IsNullOrWhiteSpace([string]$sourceEntry.category)) { $staticErrors.Add('marketplace_category_missing') | Out-Null }
    $resolvedSource = [System.IO.Path]::GetFullPath((Join-Path $ProjectRoot ([string]$sourceEntry.source.path)))
    if ((Normalize-PathValue $resolvedSource) -ne (Normalize-PathValue $pluginRoot)) { $staticErrors.Add('marketplace_source_resolves_outside_plugin') | Out-Null }
}

$mcpServers = if ($mcp) { @($mcp.mcpServers.PSObject.Properties) } else { @() }
if ($mcpServers.Count -ne 2) { $staticErrors.Add('mcp_server_count_mismatch') | Out-Null }
if ($mcpServers.Name -notcontains 'yilong-project-context') { $staticErrors.Add('context_mcp_missing') | Out-Null }
if ($mcpServers.Name -notcontains 'yilong-project-memory-receipt') { $staticErrors.Add('receipt_mcp_missing') | Out-Null }
$hookEvents = if ($hooks) { @($hooks.hooks.PSObject.Properties.Name | Sort-Object) } else { @() }
if (($hookEvents -join ',') -ne 'PostToolUse,SessionEnd,Stop') { $staticErrors.Add('hook_event_contract_mismatch') | Out-Null }

$nodeCommand = Get-Command node -ErrorAction SilentlyContinue
$nodeVersion = ""
$nodeMajor = 0
if ($nodeCommand) {
    try {
        $nodeVersion = (& node --version 2>$null | Select-Object -First 1).Trim()
        if ($nodeVersion -match '^v?(\d+)\.') { $nodeMajor = [int]$Matches[1] }
    } catch {
        $nodeVersion = ""
    }
}
if ($nodeMajor -lt 18) { $staticErrors.Add('node_18_or_newer_required') | Out-Null }

$configState = Read-CodexConfigState `
    -ConfigPath (Join-Path $CodexHome 'config.toml') `
    -ExpectedProject (Normalize-PathValue $ProjectRoot) `
    -PluginName $pluginName `
    -MarketplaceName $marketplaceName

$sourceFiles = Get-PluginFileMap -Root $pluginRoot
$cacheRoot = Join-Path $CodexHome "plugins\cache\$marketplaceName\$pluginName"
$cacheCandidates = @()
if (Test-Path -LiteralPath $cacheRoot -PathType Container) {
    $cacheCandidates = @(Get-ChildItem -LiteralPath $cacheRoot -Directory | Sort-Object LastWriteTimeUtc -Descending)
}
$installedChecks = New-Object System.Collections.Generic.List[object]
foreach ($candidate in $cacheCandidates) {
    $comparison = Test-InstalledPluginCopy -SourceFiles $sourceFiles -InstalledRoot $candidate.FullName
    $installedChecks.Add([pscustomobject]@{
        cache_version = $candidate.Name
        source_exact = [bool]$comparison.exact
        mismatch_count = $comparison.mismatch_count
        mismatches = $comparison.mismatches
    }) | Out-Null
}
$exactInstalled = @($installedChecks | Where-Object { $_.source_exact }).Count -gt 0
$projectTrusted = $configState.project_trust_level -eq 'trusted'
$installedReady = $staticErrors.Count -eq 0 -and $projectTrusted -and $exactInstalled -and $configState.plugin_enabled
$resolvedAdminUrl = Find-NodeAdminApi -RequestedUrl $NodeAdminUrl
$runtimeReady = $installedReady -and -not [string]::IsNullOrWhiteSpace($resolvedAdminUrl)

$nextActions = New-Object System.Collections.Generic.List[string]
if (-not $projectTrusted) { $nextActions.Add('Trust this project in Codex so project-scoped plugin configuration can load.') | Out-Null }
if (-not $exactInstalled) { $nextActions.Add("Restart Codex, select '$marketplaceName' in Plugins, and install '$pluginName'.") | Out-Null }
if ($exactInstalled -and -not $configState.plugin_enabled) { $nextActions.Add("Enable '$pluginName' in Codex Plugins, then start a new task.") | Out-Null }
if ([string]::IsNullOrWhiteSpace($resolvedAdminUrl)) { $nextActions.Add('Start the Yilong Windows node and expose its loopback /api/health endpoint.') | Out-Null }
if ($hookEvents.Count -gt 0) { $nextActions.Add('Open /hooks in a new Codex task and review the current Hook definitions; this probe never reads or fabricates Hook trust.') | Out-Null }

$result = [ordered]@{
    schema = 'elon.project_memory_codex_readiness.v1'
    project_root = $ProjectRoot
    marketplace = [ordered]@{
        path = $marketplacePath
        name = $marketplaceName
        plugin_name = $pluginName
        static_ready = ($staticErrors.Count -eq 0)
        errors = @($staticErrors | ForEach-Object { [string]$_ })
    }
    prerequisites = [ordered]@{
        node_version = $nodeVersion
        node_18_or_newer = ($nodeMajor -ge 18)
    }
    codex = [ordered]@{
        config_present = [bool]$configState.config_present
        project_configured = [bool]$configState.project_configured
        project_trust_level = $configState.project_trust_level
        plugin_configured = [bool]$configState.plugin_configured
        plugin_enabled = [bool]$configState.plugin_enabled
        plugin_config_key = $configState.plugin_config_key
        installed_cache_candidate_count = $installedChecks.Count
        installed_source_exact = $exactInstalled
        installed_candidates = @($installedChecks | ForEach-Object { $_ })
    }
    runtime = [ordered]@{
        node_admin_available = (-not [string]::IsNullOrWhiteSpace($resolvedAdminUrl))
        node_admin_url = $resolvedAdminUrl
    }
    claims = [ordered]@{
        static_ready = ($staticErrors.Count -eq 0)
        installed_ready = $installedReady
        runtime_ready = $runtimeReady
        hook_trust_verified = $false
        hook_trust_status = if ($hookEvents.Count -gt 0) { 'requires_interactive_codex_review' } else { 'not_configured' }
        end_to_end_verified = $false
    }
    next_actions = @($nextActions | ForEach-Object { [string]$_ })
}

$result | ConvertTo-Json -Depth 10
$failed = $staticErrors.Count -gt 0 -or ($RequireInstalled -and -not $installedReady) -or ($RequireRuntime -and -not $runtimeReady)
if ($failed) { exit 1 }
