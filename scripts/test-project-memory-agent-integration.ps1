$ErrorActionPreference = "Stop"

function Assert-True {
    param([bool]$Condition, [string]$Message)
    if (-not $Condition) { throw $Message }
}

$repoRoot = [System.IO.Path]::GetFullPath((Join-Path $PSScriptRoot '..'))
$nodeServer = Get-Content -Raw -LiteralPath (Join-Path $repoRoot 'server\src\node_agent_admin_server.rs')
Assert-True $nodeServer.Contains('"/api/health"') 'Node admin server must expose /api/health.'

$proxyPath = Join-Path $repoRoot 'plugins\yilong-project-memory\scripts\project-memory-mcp-proxy.mjs'
$observerPath = Join-Path $repoRoot 'scripts\project-memory-app-server-observer.mjs'
$readinessPath = Join-Path $repoRoot 'scripts\test-project-memory-codex-readiness.ps1'
$benchmarkPlanPath = Join-Path $repoRoot 'scripts\new-project-memory-benchmark-plan.ps1'
$ciPath = Join-Path $repoRoot 'scripts\project-memory-ci.ps1'
$dispatchPath = Join-Path $repoRoot 'scripts\dispatch-document-organization.ps1'
$proxy = Get-Content -Raw -LiteralPath $proxyPath
$observer = Get-Content -Raw -LiteralPath $observerPath
$ci = Get-Content -Raw -LiteralPath $ciPath
$dispatch = Get-Content -Raw -LiteralPath $dispatchPath
Assert-True $proxy.Contains('`${candidate}/api/health') 'Plugin proxy must discover the node through /api/health.'
Assert-True (-not $proxy.Contains('`${candidate}/health')) 'Plugin proxy still contains the obsolete /health probe.'
Assert-True $observer.Contains('`${candidate}/api/health') 'Observer must discover the node through /api/health.'
Assert-True (-not $observer.Contains('`${candidate}/health')) 'Observer still contains the obsolete /health probe.'
Assert-True $ci.Contains('$candidate/api/health') 'Memory CI must discover the node through /api/health.'
Assert-True (-not $ci.Contains('$candidate/health')) 'Memory CI still contains the obsolete /health probe.'
Assert-True $dispatch.Contains('$candidate/api/health') 'Document automation must discover the node through /api/health.'
Assert-True (-not $dispatch.Contains('$candidate/health')) 'Document automation still contains the obsolete /health probe.'

$marketplace = Get-Content -Raw -LiteralPath (Join-Path $repoRoot '.agents\plugins\marketplace.json') |
    ConvertFrom-Json
Assert-True ($marketplace.name -eq 'yilong-project') 'Repo marketplace name drifted.'
Assert-True ($marketplace.interface.displayName -eq 'Yilong Project') 'Repo marketplace display name drifted.'
$marketplaceEntries = @($marketplace.plugins)
Assert-True ($marketplaceEntries.Count -eq 1) 'Repo marketplace must expose exactly one project-memory plugin.'
$marketplaceEntry = $marketplaceEntries[0]
Assert-True ($marketplaceEntry.name -eq 'yilong-project-memory') 'Repo marketplace plugin name drifted.'
Assert-True ($marketplaceEntry.source.source -eq 'local') 'Repo marketplace plugin must remain local.'
Assert-True ($marketplaceEntry.source.path -eq './plugins/yilong-project-memory') 'Repo marketplace source path drifted.'
Assert-True ($marketplaceEntry.policy.installation -eq 'AVAILABLE') 'Repo marketplace installation policy drifted.'
Assert-True (@('ON_INSTALL', 'ON_USE') -contains $marketplaceEntry.policy.authentication) 'Repo marketplace authentication policy is invalid.'

foreach ($script in @(
    $proxyPath,
    $observerPath,
    (Join-Path $repoRoot 'plugins\yilong-project-memory\scripts\project-memory-hook.mjs')
)) {
    & node --check $script
    Assert-True ($LASTEXITCODE -eq 0) "Node syntax check failed: $script"
}

$mcp = Get-Content -Raw -LiteralPath (Join-Path $repoRoot 'plugins\yilong-project-memory\.mcp.json') |
    ConvertFrom-Json
$servers = @($mcp.mcpServers.PSObject.Properties)
Assert-True ($servers.Count -eq 2) 'Project-memory plugin must expose exactly two MCP servers.'
Assert-True ($servers.Name -contains 'yilong-project-context') 'Context MCP server is missing.'
Assert-True ($servers.Name -contains 'yilong-project-memory-receipt') 'Receipt MCP server is missing.'
foreach ($server in $servers) {
    Assert-True ($server.Value.command -eq 'node') "MCP server $($server.Name) must use the bundled Node proxy."
    Assert-True (@($server.Value.args).Count -eq 2) "MCP server $($server.Name) must pass script and profile only."
}
Assert-True ($mcp.mcpServers.'yilong-project-context'.args[1] -eq 'context') 'Context server profile drifted.'
Assert-True ($mcp.mcpServers.'yilong-project-memory-receipt'.args[1] -eq 'receipt') 'Receipt server profile drifted.'

$hooks = Get-Content -Raw -LiteralPath (Join-Path $repoRoot 'plugins\yilong-project-memory\hooks\hooks.json') |
    ConvertFrom-Json
$events = @($hooks.hooks.PSObject.Properties.Name | Sort-Object)
Assert-True (($events -join ',') -eq 'PostToolUse,SessionEnd,Stop') 'Plugin Hook events must remain bounded to PostToolUse, Stop, and SessionEnd.'

$tempRoot = Join-Path ([System.IO.Path]::GetTempPath()) ("elon-project-memory-contract-" + [guid]::NewGuid().ToString('N'))
New-Item -ItemType Directory -Path $tempRoot | Out-Null
try {
    $emptyCodexHome = Join-Path $tempRoot 'empty-codex-home'
    New-Item -ItemType Directory -Path $emptyCodexHome | Out-Null
    $staticOutput = @(& powershell -NoProfile -ExecutionPolicy Bypass -File $readinessPath `
        -ProjectRoot $repoRoot -CodexHome $emptyCodexHome -NodeAdminUrl 'http://127.0.0.1:1')
    Assert-True ($LASTEXITCODE -eq 0) 'Static Codex readiness probe failed.'
    $staticReadiness = ($staticOutput -join "`n") | ConvertFrom-Json
    Assert-True $staticReadiness.claims.static_ready 'Static Codex readiness should pass for the repository source.'
    Assert-True (-not $staticReadiness.claims.installed_ready) 'Empty Codex home must not report the plugin as installed.'
    Assert-True (-not $staticReadiness.claims.hook_trust_verified) 'Readiness must not infer Hook trust.'

    $fakeCodexHome = Join-Path $tempRoot 'installed-codex-home'
    $installedPlugin = Join-Path $fakeCodexHome 'plugins\cache\yilong-project\yilong-project-memory\local'
    New-Item -ItemType Directory -Force -Path (Split-Path -Parent $installedPlugin) | Out-Null
    Copy-Item -LiteralPath (Join-Path $repoRoot 'plugins\yilong-project-memory') -Destination $installedPlugin -Recurse
    $configText = @"
[projects.'$($repoRoot.ToLowerInvariant())']
trust_level = "trusted"

[plugins."yilong-project-memory@yilong-project"]
enabled = true
"@
    [System.IO.File]::WriteAllText(
        (Join-Path $fakeCodexHome 'config.toml'),
        $configText,
        (New-Object System.Text.UTF8Encoding($false))
    )
    $installedOutput = @(& powershell -NoProfile -ExecutionPolicy Bypass -File $readinessPath `
        -ProjectRoot $repoRoot -CodexHome $fakeCodexHome -NodeAdminUrl 'http://127.0.0.1:1' -RequireInstalled)
    Assert-True ($LASTEXITCODE -eq 0) 'Installed Codex readiness fixture failed.'
    $installedReadiness = ($installedOutput -join "`n") | ConvertFrom-Json
    Assert-True $installedReadiness.claims.installed_ready 'Exact cache copy, trusted project, and enabled plugin should be install-ready.'
    Assert-True (-not $installedReadiness.claims.runtime_ready) 'Unavailable loopback node must keep runtime readiness false.'

    $benchmarkRepo = Join-Path $tempRoot 'benchmark-repo'
    New-Item -ItemType Directory -Path $benchmarkRepo | Out-Null
    & git -C $benchmarkRepo init --quiet
    Assert-True ($LASTEXITCODE -eq 0) 'Unable to initialize benchmark fixture repository.'
    [System.IO.File]::WriteAllText((Join-Path $benchmarkRepo 'tracked.txt'), "fixture`n")
    & git -C $benchmarkRepo add tracked.txt
    & git -C $benchmarkRepo -c user.name=Codex -c user.email=codex@example.invalid commit --quiet -m fixture
    Assert-True ($LASTEXITCODE -eq 0) 'Unable to commit benchmark fixture repository.'
    $taskSecret = 'private benchmark task text must never enter the manifest'
    $taskFile = Join-Path $tempRoot 'benchmark-task.txt'
    [System.IO.File]::WriteAllText($taskFile, $taskSecret, (New-Object System.Text.UTF8Encoding($false)))
    $benchmarkManifestPath = Join-Path $tempRoot 'benchmark-manifest.json'
    $planOutput = @(& powershell -NoProfile -ExecutionPolicy Bypass -File $benchmarkPlanPath `
        -CaseId 'project-orientation' -ModelId 'gpt-test' -TaskFile $taskFile `
        -CodexBuild 'test-build' -ProjectRoot $benchmarkRepo -OutputPath $benchmarkManifestPath)
    Assert-True ($LASTEXITCODE -eq 0) 'Benchmark plan generation failed.'
    $plan = ($planOutput -join "`n") | ConvertFrom-Json
    Assert-True ($plan.benchmark_key -match '^pmab-[0-9a-f]{32}$') 'Benchmark key must be derived from the manifest hash.'
    $manifestText = Get-Content -Raw -LiteralPath $benchmarkManifestPath
    Assert-True (-not $manifestText.Contains($taskSecret)) 'Benchmark manifest leaked task text.'
    $validationOutput = @(& node $observerPath --benchmark-manifest $benchmarkManifestPath `
        --task-file $taskFile --model-id gpt-test --codex-build test-build `
        --window baseline_without_project_memory --validate-manifest-only)
    Assert-True ($LASTEXITCODE -eq 0) 'Benchmark observer manifest validation failed.'
    $validation = ($validationOutput -join "`n") | ConvertFrom-Json
    Assert-True $validation.benchmark_protocol_verified 'Observer did not verify the benchmark protocol.'
    Assert-True ($validation.benchmark_key -eq $plan.benchmark_key) 'Observer and plan disagree on benchmark key.'

    [System.IO.File]::WriteAllText($taskFile, "$taskSecret changed", (New-Object System.Text.UTF8Encoding($false)))
    $previousErrorActionPreference = $ErrorActionPreference
    $ErrorActionPreference = 'Continue'
    & node $observerPath --benchmark-manifest $benchmarkManifestPath `
        --task-file $taskFile --model-id gpt-test --codex-build test-build `
        --window baseline_without_project_memory --validate-manifest-only 2>$null | Out-Null
    $mismatchExitCode = $LASTEXITCODE
    $ErrorActionPreference = $previousErrorActionPreference
    Assert-True ($mismatchExitCode -ne 0) 'Observer accepted a task file that did not match the benchmark manifest.'
    [System.IO.File]::WriteAllText($taskFile, $taskSecret, (New-Object System.Text.UTF8Encoding($false)))

    [System.IO.File]::AppendAllText((Join-Path $benchmarkRepo 'tracked.txt'), "dirty`n")
    $ErrorActionPreference = 'Continue'
    & node $observerPath --benchmark-manifest $benchmarkManifestPath `
        --task-file $taskFile --model-id gpt-test --codex-build test-build `
        --window with_project_memory --validate-manifest-only 2>$null | Out-Null
    $dirtyExitCode = $LASTEXITCODE
    $ErrorActionPreference = $previousErrorActionPreference
    Assert-True ($dirtyExitCode -ne 0) 'Observer accepted a dirty tracked benchmark worktree.'
} finally {
    if (Test-Path -LiteralPath $tempRoot) {
        Remove-Item -LiteralPath $tempRoot -Recurse -Force
    }
}

Write-Output 'PROJECT_MEMORY_AGENT_CONTRACT=passed'
Write-Output 'PROJECT_MEMORY_AGENT_MCP_SERVER_COUNT=2'
Write-Output 'PROJECT_MEMORY_AGENT_HOOK_EVENTS=PostToolUse,SessionEnd,Stop'
Write-Output 'PROJECT_MEMORY_CODEX_MARKETPLACE=passed'
Write-Output 'PROJECT_MEMORY_CODEX_READINESS=passed'
Write-Output 'PROJECT_MEMORY_BENCHMARK_PROTOCOL=passed'
