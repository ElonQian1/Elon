#requires -Version 7.0
[CmdletBinding()]
param(
    [Parameter(Mandatory = $true)][string]$ProjectRoot,
    [Parameter(Mandatory = $true)][string[]]$ConversationReference,
    [string]$ApkMcpUrl = ''
)
$ErrorActionPreference = 'Stop'
$codex = Get-Command codex -ErrorAction Stop
# Preserve this reader's existing device selection when refreshing its code/grant.
$oldRegistration = & $codex.Source mcp get yilong_web_conversations --json 2>$null
$existingEnvironment = @{}
if ($LASTEXITCODE -eq 0) {
    $previous = $oldRegistration | ConvertFrom-Json -AsHashtable
    foreach ($name in @('ELON_APK_MCP_URL', 'ELON_WEB_CONVERSATION_WIN_INSTANCE', 'ELON_NODE_ADMIN_URL', 'ELON_WEB_CONVERSATION_AUTOSTART')) {
        if ($previous.transport.env -is [System.Collections.IDictionary] -and $previous.transport.env.ContainsKey($name)) {
            $existingEnvironment[$name] = $previous.transport.env[$name]
        }
    }
}
if ($ApkMcpUrl) { $existingEnvironment['ELON_APK_MCP_URL'] = $ApkMcpUrl }
if ($existingEnvironment.ContainsKey('ELON_APK_MCP_URL')) {
    $endpoint = [Uri]$existingEnvironment['ELON_APK_MCP_URL']
    if ($endpoint.Scheme -ne 'http' -or $endpoint.Host -ne '127.0.0.1' -or $endpoint.UserInfo -or
        $endpoint.Query -or $endpoint.Fragment -or $endpoint.AbsolutePath -ne '/') { throw 'Invalid local APK MCP endpoint.' }
}
$storageRoot = Join-Path ([Environment]::GetFolderPath('LocalApplicationData')) 'Elon/web-conversation-reader'
$inputJson = @{ projectRoot = $ProjectRoot; references = $ConversationReference; storageRoot = $storageRoot } | ConvertTo-Json -Compress
$registration = $inputJson | & node (Join-Path $PSScriptRoot 'registration.mjs')
if ($LASTEXITCODE -ne 0) { throw 'Reader registration preparation failed.' }
$registration = $registration | ConvertFrom-Json
$cliArguments = @('mcp', 'add', 'yilong_web_conversations',
    '--env', "ELON_PROJECT_ROOT=$($registration.projectRoot)",
    '--env', "ELON_WEB_CONVERSATION_IDS=$($registration.ids)")
foreach ($name in $existingEnvironment.Keys) { $cliArguments += @('--env', "$name=$($existingEnvironment[$name])") }
$cliArguments += @('--', $registration.command, $registration.entrypoint)
& $codex.Source @cliArguments
if ($LASTEXITCODE -ne 0) { throw 'Codex MCP registration failed.' }
# The CLI owns TOML creation/merging. Set a timeout only in our exact server table.
$configRoot = if ($env:CODEX_HOME) { $env:CODEX_HOME } else { Join-Path $env:USERPROFILE '.codex' }
$configPath = Join-Path $configRoot 'config.toml'
$configText = [IO.File]::ReadAllText($configPath)
$tablePattern = '(?ms)^\[mcp_servers\.yilong_web_conversations\]\r?\n(?<body>.*?)(?=^\[|\z)'
$matches = [regex]::Matches($configText, $tablePattern)
if ($matches.Count -ne 1) { throw 'Cannot locate the registered reader table; other settings were preserved.' }
$match = $matches[0]
$body = [regex]::Replace($match.Groups['body'].Value, '(?m)^tool_timeout_sec\s*=.*\r?\n?', '')
$replacement = "[mcp_servers.yilong_web_conversations]`ntool_timeout_sec = 150`n$body"
$updated = $configText.Substring(0, $match.Index) + $replacement + $configText.Substring($match.Index + $match.Length)
# Fail if another client modified the config after it was read.
if ([IO.File]::ReadAllText($configPath) -cne $configText) { throw 'Codex config changed concurrently; retry registration.' }
[IO.File]::WriteAllText($configPath, $updated, [Text.UTF8Encoding]::new($false))
& $codex.Source mcp get yilong_web_conversations --json | Out-Null
if ($LASTEXITCODE -ne 0) { throw 'Codex could not read the registered MCP.' }
Write-Output 'CODEX_CONVERSATION_MCP=registered'
Write-Output 'RELOAD_TOOLS=Open a new Codex task or reload MCP tools to discover this server.'
