$ErrorActionPreference = 'Stop'
Set-StrictMode -Version 2.0

$RepoRoot = [System.IO.Path]::GetFullPath((Join-Path $PSScriptRoot '..'))
. (Join-Path $PSScriptRoot 'node-storage-paths.ps1')

$script:Assertions = 0
function Assert-True {
    param([bool]$Condition, [string]$Message)
    $script:Assertions++
    if (-not $Condition) { throw "ASSERT FAILED: $Message" }
}

function Assert-Equal {
    param($Expected, $Actual, [string]$Message)
    $script:Assertions++
    if ($Expected -ne $Actual) {
        throw "ASSERT FAILED: $Message expected='$Expected' actual='$Actual'"
    }
}

function Write-Utf8Json {
    param([string]$Path, $Value)
    $parent = Split-Path -Parent $Path
    New-Item -ItemType Directory -Force -Path $parent | Out-Null
    $json = $Value | ConvertTo-Json -Depth 8
    [System.IO.File]::WriteAllText($Path, $json, (New-Object System.Text.UTF8Encoding($false)))
}

function Set-TreeOld {
    param([string]$Path)
    $old = [DateTime]::UtcNow.AddDays(-40)
    Get-ChildItem -LiteralPath $Path -Recurse -Force | ForEach-Object {
        $_.LastWriteTimeUtc = $old
    }
    (Get-Item -LiteralPath $Path -Force).LastWriteTimeUtc = $old
}

$tempRoot = Join-Path ([System.IO.Path]::GetTempPath()) (
    'elon-node-storage-test-' + [Guid]::NewGuid().ToString('N')
)
New-Item -ItemType Directory -Force -Path $tempRoot | Out-Null
$saved = @{
    APPDATA = $env:APPDATA
    LOCALAPPDATA = $env:LOCALAPPDATA
    ELON_NODE_DATA_ROOT = $env:ELON_NODE_DATA_ROOT
    ELON_RUST_CACHE_ROOT = $env:ELON_RUST_CACHE_ROOT
}

try {
    $env:APPDATA = Join-Path $tempRoot 'appdata'
    $env:LOCALAPPDATA = Join-Path $tempRoot 'local'
    $env:ELON_NODE_DATA_ROOT = $null
    $env:ELON_RUST_CACHE_ROOT = $null

    $nodeRoot = Join-Path $tempRoot 'managed-node-data'
    New-Item -ItemType Directory -Force -Path $nodeRoot | Out-Null
    Write-Utf8Json -Path (Join-Path $nodeRoot '.elon-node-data-root.json') -Value @{
        schema_version = 1
        install_id = 'test-install'
    }
    Write-Utf8Json -Path (Join-Path $env:APPDATA 'elon-node-agent\node.json') -Value @{
        install_id = 'test-install'
        node_data_root = $nodeRoot
    }

    Assert-Equal $nodeRoot (Get-ElonPersistedNodeDataRoot) `
        'matching persisted config and ownership marker should resolve the node data root'
    Assert-Equal (Join-Path $nodeRoot 'cache\release-targets') `
        (Get-ElonManagedReleaseTargetRoot) `
        'managed release targets should live below the node data cache root'

    Write-Utf8Json -Path (Join-Path $nodeRoot '.elon-node-data-root.json') -Value @{
        schema_version = 1
        install_id = 'other-install'
    }
    Assert-True ($null -eq (Get-ElonPersistedNodeDataRoot)) `
        'an install-id mismatch must reject the persisted node data root'
    Write-Utf8Json -Path (Join-Path $nodeRoot '.elon-node-data-root.json') -Value @{
        schema_version = 1
        install_id = 'test-install'
    }

    $buildRoot = Join-Path $env:LOCALAPPDATA 'Elon\build-target'
    $legacyDev = Join-Path $buildRoot 'elon-dev-cargo'
    $orphaned = Join-Path $buildRoot 'elon-build-deadbeef'
    $activeNode = Join-Path $buildRoot 'elon-node-agent'
    foreach ($path in @($legacyDev, $orphaned, $activeNode)) {
        New-Item -ItemType Directory -Force -Path $path | Out-Null
        [System.IO.File]::WriteAllText((Join-Path $path '.rustc_info.json'), 'test')
    }
    Set-TreeOld -Path $legacyDev
    Set-TreeOld -Path $orphaned

    $oldRustV2 = Join-Path $env:LOCALAPPDATA 'Elon\rust-cache-v2'
    New-Item -ItemType Directory -Force -Path $oldRustV2 | Out-Null
    [System.IO.File]::WriteAllText((Join-Path $oldRustV2 'legacy.bin'), 'test')
    Set-TreeOld -Path $oldRustV2

    $inspectScript = Join-Path $PSScriptRoot 'inspect-node-disk-usage.ps1'
    $preview = (& $inspectScript -MinAgeDays 7 6>&1 | Out-String)
    Assert-True ($preview -match 'retired_dev_target') `
        'default preview should include the retired development target'
    Assert-True ($preview -match 'orphaned_build_target') `
        'default preview should include marked and expired elon-build targets'
    Assert-True ($preview -match 'retired_rust_cache_v2') `
        'default preview should include a superseded local rust-cache-v2 root'
    Assert-True ($preview -notmatch 'active_rebuildable_target') `
        'default preview must not mix active release caches into safe legacy candidates'
    Assert-True ($preview -match 'PREVIEW_ONLY=true') `
        'inspection must remain dry-run by default'

    Set-TreeOld -Path $activeNode
    $activePreview = (& $inspectScript -MinAgeDays 7 -IncludeActiveBuildCaches 6>&1 | Out-String)
    Assert-True ($activePreview -match 'active_rebuildable_target') `
        'active release caches should require an explicit opt-in'

    $oldSha = 'a' * 40
    $newSha = 'b' * 40
    $outboxEvent = Join-Path $env:LOCALAPPDATA "Elon\release-outbox-v1\events\$oldSha"
    $outboxSource = Join-Path $env:LOCALAPPDATA "Elon\release-outbox-v1\sources\$oldSha"
    New-Item -ItemType Directory -Force -Path $outboxEvent, $outboxSource | Out-Null
    Write-Utf8Json -Path (Join-Path $outboxEvent 'event.json') -Value @{
        sync_state = 'synced'
        created_at_ms = 1
        updated_at_ms = 2
    }
    [System.IO.File]::WriteAllText((Join-Path $outboxSource '.git'), 'gitdir: test')
    Set-TreeOld -Path $outboxEvent
    Set-TreeOld -Path $outboxSource

    $releaseRoot = Join-Path $env:LOCALAPPDATA 'Elon\local-node-releases-v1\releases'
    foreach ($release in @(
        @{ Sha = $oldSha; Verified = 1; Activation = 'superseded'; Terminal = 'complete'; Old = $true },
        @{ Sha = $newSha; Verified = 2; Activation = 'activated'; Terminal = 'complete'; Old = $false }
    )) {
        $dir = Join-Path $releaseRoot $release.Sha
        New-Item -ItemType Directory -Force -Path $dir | Out-Null
        Write-Utf8Json -Path (Join-Path $dir 'state.json') -Value @{
            verified_at_ms = $release.Verified
            activation_state = $release.Activation
            local_terminal_state = $release.Terminal
        }
        if ($release.Old) { Set-TreeOld -Path $dir }
    }
    $historyPreview = (
        & $inspectScript -MinAgeDays 7 -IncludeReleaseHistory -ReleaseKeepNewest 1 6>&1 |
            Out-String
    )
    Assert-True ($historyPreview -match 'terminal_outbox_event') `
        'terminal outbox events older than retention should be reported'
    Assert-True ($historyPreview -match 'terminal_outbox_source') `
        'terminal outbox source worktrees should use a dedicated cleanup candidate'
    Assert-True ($historyPreview -match 'terminal_local_release') `
        'superseded local releases outside the newest protection set should be reported'

    foreach ($scriptName in @(
        'node-storage-paths.ps1',
        'inspect-node-disk-usage.ps1',
        'publish-node-agent.ps1',
        'publish-server.ps1'
    )) {
        $tokens = $null
        $parseErrors = $null
        [System.Management.Automation.Language.Parser]::ParseFile(
            (Join-Path $PSScriptRoot $scriptName),
            [ref]$tokens,
            [ref]$parseErrors
        ) | Out-Null
        Assert-Equal 0 $parseErrors.Count "PowerShell parser should accept $scriptName"
    }

    $nodePublishText = Get-Content -Raw -LiteralPath (Join-Path $PSScriptRoot 'publish-node-agent.ps1')
    $serverPublishText = Get-Content -Raw -LiteralPath (Join-Path $PSScriptRoot 'publish-server.ps1')
    Assert-True ($nodePublishText -match 'Resolve-ElonNodeAgentTargetDir') `
        'node-agent publishing should prefer the managed node data root'
    Assert-True ($serverPublishText -match 'Resolve-ElonServerMuslTargetDir') `
        'server publishing should prefer the managed node data root'

    Write-Host "PASS: node storage maintenance tests ($script:Assertions assertions)"
} finally {
    $env:APPDATA = $saved.APPDATA
    $env:LOCALAPPDATA = $saved.LOCALAPPDATA
    $env:ELON_NODE_DATA_ROOT = $saved.ELON_NODE_DATA_ROOT
    $env:ELON_RUST_CACHE_ROOT = $saved.ELON_RUST_CACHE_ROOT
    if (Test-Path -LiteralPath $tempRoot) {
        Remove-Item -LiteralPath $tempRoot -Recurse -Force
    }
}
