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
$ciPath = Join-Path $repoRoot 'scripts\project-memory-ci.ps1'
$proxy = Get-Content -Raw -LiteralPath $proxyPath
$observer = Get-Content -Raw -LiteralPath $observerPath
$ci = Get-Content -Raw -LiteralPath $ciPath
Assert-True $proxy.Contains('`${candidate}/api/health') 'Plugin proxy must discover the node through /api/health.'
Assert-True (-not $proxy.Contains('`${candidate}/health')) 'Plugin proxy still contains the obsolete /health probe.'
Assert-True $observer.Contains('`${candidate}/api/health') 'Observer must discover the node through /api/health.'
Assert-True (-not $observer.Contains('`${candidate}/health')) 'Observer still contains the obsolete /health probe.'
Assert-True $ci.Contains('$candidate/api/health') 'Memory CI must discover the node through /api/health.'
Assert-True (-not $ci.Contains('$candidate/health')) 'Memory CI still contains the obsolete /health probe.'

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

Write-Output 'PROJECT_MEMORY_AGENT_CONTRACT=passed'
Write-Output 'PROJECT_MEMORY_AGENT_MCP_SERVER_COUNT=2'
Write-Output 'PROJECT_MEMORY_AGENT_HOOK_EVENTS=PostToolUse,SessionEnd,Stop'
