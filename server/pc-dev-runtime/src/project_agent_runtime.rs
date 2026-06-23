// server/pc-dev-runtime/src/project_agent_runtime.rs

use crate::{
    project_agent_runtime_context::agent_runtime_context_helpers,
    project_agent_runtime_lifecycle::agent_runtime_lifecycle_helpers,
    project_agent_runtime_patch::{
        agent_runtime_apply_patch_action_case, agent_runtime_apply_patch_helpers,
    },
    project_scaffold::ProjectScaffoldRequest,
};
use std::{
    fs, io,
    path::{Path, PathBuf},
};

pub(crate) fn ensure_project_agent_runtime_files(
    repo: &Path,
    req: &ProjectScaffoldRequest<'_>,
) -> io::Result<()> {
    ensure_file(
        repo.join("scripts").join("elon-agent.ps1"),
        agent_runtime_script,
    )?;
    ensure_file(repo.join("docs").join("agent-runtime.md"), || {
        agent_runtime_doc(req)
    })?;
    Ok(())
}

fn ensure_file(path: PathBuf, content: impl FnOnce() -> io::Result<String>) -> io::Result<()> {
    if path.exists() {
        return Ok(());
    }
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, content()?)
}

fn agent_runtime_doc(req: &ProjectScaffoldRequest<'_>) -> io::Result<String> {
    Ok(format!(
        r#"# Agent Runtime Modes

This project supports three AI development modes through `scripts\elon.ps1 agent`.
- Route A, `cli-wrapper`: this project CLI calls an installed AI CLI such as Codex, Claude, Gemini, or Copilot. That external CLI owns model calls, file edits, shell execution, and its own approval policy.
- Route B, `api-runtime`: this project CLI calls an OpenAI-compatible API directly and executes a small local tool loop itself. The local runtime owns file reads, file writes, command execution checks, confirmations, and workspace path limits.
- Route C, `server-runtime`: this project CLI asks the Elon server to call a configured platform AI agent. The server owns the model/API key, while this PC still owns local file reads, file writes, command execution checks, confirmations, and workspace path limits.

Project metadata: `project_id={}`, `template={}`, `owner_user_id={}`

## Status

```powershell
scripts\elon.ps1 agent
```

## Route A: wrap another CLI

```powershell
scripts\elon.ps1 agent -AgentMode cli-wrapper -Cli codex -Prompt "Inspect this repo and suggest the next safe change"
scripts\elon.ps1 agent -AgentMode cli-wrapper -Cli claude -Prompt "Run the project checks and summarize failures"
scripts\elon.ps1 agent -AgentMode cli-wrapper -Cli gemini -Prompt "Explain this project structure"
```

## Route B: direct API runtime
Configure an OpenAI-compatible endpoint with environment variables:

```powershell
$env:ELON_AGENT_API_BASE = "https://api.openai.com/v1"
$env:ELON_AGENT_API_KEY = "<secret>"
$env:ELON_AGENT_MODEL = "<model>"
```

```powershell
scripts\elon.ps1 agent -AgentMode api-runtime -Prompt "Read README.md and tell me what check command to run"
scripts\elon.ps1 agent -AgentMode api-runtime -Prompt "Create a docs note with the current project status" -DryRun
```

Route B is intentionally conservative. It only permits workspace-scoped `list_dir`, `search_files`, `file_info`, `read_file`, `read_file_range`, read-only `git_status` / `git_diff` / `git_log`, `write_file`, `apply_patch`, and a small allowlist of project commands. `search_files` is read-only and bounded, so use it before broad file reads when locating symbols, filenames, TODOs, errors, or related code. `file_info` is read-only and helps inspect unknown files or directories before reading them. `git_status`, `git_diff`, and `git_log` are read-only git inspection tools and do not require command approval. `run_command` should use structured `program` + `args` fields instead of one shell string. File writes, patch application, and command execution require confirmation unless `-Yes` is provided. `-DryRun` previews writes, patch checks, and commands without applying them. Each agent run has a `-MaxRunCommands` budget, a `-MaxContextChars` context budget, and truncates large command output before sending it back to the model. When the local conversation grows past the context budget, older assistant/tool-result messages are compacted into a metadata-only summary while the original instruction and recent turns are kept. This is not an OS sandbox: build/test commands can still execute project code, so only run it for projects you trust.

## Route C: Elon server runtime
Use this when the PC does not have Codex/Claude/Gemini/Copilot installed and the user does not have their own API key.

```powershell
$env:ELON_SERVER_URL = "http://43.139.149.158:8080"
$env:ELON_SERVER_TOKEN = "<login token>"
```

```powershell
scripts\elon.ps1 agent -AgentMode server-runtime -Prompt "Read README.md and summarize this project"
scripts\elon.ps1 agent -AgentMode server-runtime -Prompt "Create a docs note" -DryRun
```

Route C does not expose the server API key to this PC. The server only returns structured local actions; this PC runtime still applies the same workspace path, dry-run, command policy, and confirmation checks as Route B.
When the Windows client is installed and logged in, Route C can reuse the local node login token automatically. Manual `ELON_SERVER_TOKEN` is only needed for advanced or portable setups.
`ELON_SERVER_AGENT` is optional and is honored only when the Elon server operator explicitly allows that agent through `ELON_SERVER_AGENT_RUNTIME_ALLOWED_AGENTS`; otherwise Route C uses the server default agent.

## Task lifecycle logs
Route B and Route C write one JSONL lifecycle trace per run under `.elon\agent-runs\`. The trace records run start, context budget, each model turn, requested tool names and targets, result sizes, context compaction counts, and final completion or failure status. It intentionally avoids storing full file contents, tool output, prompts, or API keys.
"#,
        req.project_id, req.template, req.user_id
    ))
}

fn agent_runtime_script() -> io::Result<String> {
    Ok(r#"param(
    [ValidateSet('status', 'cli-wrapper', 'api-runtime', 'server-runtime')][string]$Mode = 'status',
    [string]$Prompt = '',
    [ValidateSet('codex', 'claude', 'gemini', 'copilot')][string]$Cli = 'codex',
    [string[]]$CliArgs = @(),
    [string]$ApiBase = '',
    [string]$ApiKey = '',
    [string]$Model = '',
    [string]$ServerUrl = '',
    [string]$ServerToken = '',
    [string]$ServerAgent = '',
    [string]$RunId = '',
    [int]$MaxTurns = 6,
    [int]$MaxRunCommands = 8,
    [int]$MaxContextChars = 60000,
    [switch]$DryRun,
    [switch]$Yes
)

$ErrorActionPreference = 'Stop'

$ProjectRoot = (Resolve-Path -LiteralPath (Join-Path $PSScriptRoot '..')).Path
Set-Location $ProjectRoot

if ($MaxRunCommands -lt 1) { $MaxRunCommands = 1 }
if ($MaxRunCommands -gt 20) { $MaxRunCommands = 20 }
if ($MaxContextChars -lt 10000) { $MaxContextChars = 10000 }
if ($MaxContextChars -gt 200000) { $MaxContextChars = 200000 }
$Script:AgentRunCommandCount = 0
$Script:AgentContextCompactionCount = 0
$Script:AgentRunId = ''
$Script:AgentRunLogPath = ''
$Script:AgentRunLifecycleClosed = $false
$AgentCommandOutputMaxChars = 12000

function Test-Tool {
    param([Parameter(Mandatory = $true)][string]$Name)
    return [bool](Get-Command $Name -ErrorAction SilentlyContinue)
}

function Get-FirstEnv {
    param([Parameter(Mandatory = $true)][string[]]$Names)
    foreach ($name in $Names) {
        $value = [Environment]::GetEnvironmentVariable($name, 'Process')
        if (-not $value) {
            $value = [Environment]::GetEnvironmentVariable($name, 'User')
        }
        if ($value -and $value.Trim()) {
            return $value.Trim()
        }
    }
    return ''
}

function Get-NodeAgentUserToken {
    $path = ''
    if ($env:APPDATA) {
        $path = Join-Path $env:APPDATA 'elon-node-agent\node.json'
    }
    if (-not $path -or -not (Test-Path -LiteralPath $path -PathType Leaf)) {
        return ''
    }
    try {
        $state = Get-Content -LiteralPath $path -Raw | ConvertFrom-Json
        $token = [string]$state.user_token
        if ($token -and $token.Trim()) { return $token.Trim() }
    } catch {
        return ''
    }
    return ''
}

function Show-AgentStatus {
    $cliNames = @('codex', 'claude', 'gemini', 'copilot')
    Write-Host 'Elon local agent runtime'
    Write-Host "Root: $ProjectRoot"
    Write-Host "Run logs: .elon\agent-runs"
    Write-Host ''
    Write-Host 'Route A: cli-wrapper'
    foreach ($name in $cliNames) {
        if (Test-Tool $name) {
            $cmd = Get-Command $name -ErrorAction SilentlyContinue
            Write-Host "[OK] $name -> $($cmd.Source)"
        } else {
            Write-Host "[WARN] $name not found"
        }
    }
    Write-Host ''
    Write-Host 'Route B: api-runtime'
    $base = if ($ApiBase.Trim()) { $ApiBase.Trim() } else { Get-FirstEnv @('ELON_AGENT_API_BASE', 'OPENAI_API_BASE', 'HUNYUAN_API_BASE') }
    $key = if ($ApiKey.Trim()) { $ApiKey.Trim() } else { Get-FirstEnv @('ELON_AGENT_API_KEY', 'OPENAI_API_KEY', 'HUNYUAN_API_KEY') }
    $modelName = if ($Model.Trim()) { $Model.Trim() } else { Get-FirstEnv @('ELON_AGENT_MODEL', 'OPENAI_MODEL', 'HUNYUAN_MODEL') }
    if ($base) { Write-Host "[OK] api_base -> $base" } else { Write-Host '[WARN] api_base missing; set ELON_AGENT_API_BASE or pass -ApiBase' }
    if ($key) { Write-Host '[OK] api_key -> configured' } else { Write-Host '[WARN] api_key missing; set ELON_AGENT_API_KEY or pass -ApiKey' }
    if ($modelName) { Write-Host "[OK] model -> $modelName" } else { Write-Host '[WARN] model missing; set ELON_AGENT_MODEL or pass -Model' }
    Write-Host ''
    Write-Host 'Route C: server-runtime'
    $server = if ($ServerUrl.Trim()) { $ServerUrl.Trim() } else { Get-FirstEnv @('ELON_SERVER_URL', 'ELON_AGENT_SERVER_URL') }
    if (-not $server) { $server = 'http://43.139.149.158:8080' }
    $token = if ($ServerToken.Trim()) { $ServerToken.Trim() } else { Get-FirstEnv @('ELON_SERVER_TOKEN', 'ELON_AGENT_SERVER_TOKEN', 'OWNER_TOKEN') }
    if (-not $token) { $token = Get-NodeAgentUserToken }
    $agent = if ($ServerAgent.Trim()) { $ServerAgent.Trim() } else { Get-FirstEnv @('ELON_SERVER_AGENT', 'ELON_AGENT_SERVER_AGENT') }
    if ($server) { Write-Host "[OK] server_url -> $server" } else { Write-Host '[WARN] server_url missing; set ELON_SERVER_URL or pass -ServerUrl' }
    if ($token) { Write-Host '[OK] server_token -> configured' } else { Write-Host '[WARN] server_token missing; install/login Windows client or set ELON_SERVER_TOKEN' }
    if ($agent) { Write-Host "[OK] server_agent -> $agent" } else { Write-Host '[INFO] server_agent not set; server default agent will be used' }
}

function Require-Prompt {
    if (-not $Prompt.Trim()) {
        throw 'Prompt is required.'
    }
}

function Invoke-CliWrapper {
    Require-Prompt
    if (-not (Test-Tool $Cli)) {
        throw "CLI not found: $Cli"
    }

    $externalArgs = @()
    if ($CliArgs.Count -gt 0) {
        $usedPrompt = $false
        foreach ($arg in $CliArgs) {
            if ($arg -eq '{prompt}') {
                $externalArgs += $Prompt
                $usedPrompt = $true
            } else {
                $externalArgs += $arg
            }
        }
        if (-not $usedPrompt) {
            $externalArgs += $Prompt
        }
    } else {
        switch ($Cli) {
            'codex' { $externalArgs = @('exec', $Prompt) }
            'copilot' { $externalArgs = @('-p', $Prompt) }
            'claude' { $externalArgs = @('-p', $Prompt) }
            'gemini' { $externalArgs = @('-p', $Prompt) }
        }
    }

    Write-Host "> $Cli $($externalArgs -join ' ')"
    & $Cli @externalArgs
    if ($LASTEXITCODE -ne 0) {
        throw "$Cli failed with exit code $LASTEXITCODE"
    }
}

function Resolve-ApiConfig {
    $resolvedBase = if ($ApiBase.Trim()) { $ApiBase.Trim() } else { Get-FirstEnv @('ELON_AGENT_API_BASE', 'OPENAI_API_BASE', 'HUNYUAN_API_BASE') }
    $resolvedKey = if ($ApiKey.Trim()) { $ApiKey.Trim() } else { Get-FirstEnv @('ELON_AGENT_API_KEY', 'OPENAI_API_KEY', 'HUNYUAN_API_KEY') }
    $resolvedModel = if ($Model.Trim()) { $Model.Trim() } else { Get-FirstEnv @('ELON_AGENT_MODEL', 'OPENAI_MODEL', 'HUNYUAN_MODEL') }
    if (-not $resolvedBase) { $resolvedBase = 'https://api.openai.com/v1' }
    if (-not $resolvedKey) { throw 'Missing API key. Set ELON_AGENT_API_KEY, OPENAI_API_KEY, HUNYUAN_API_KEY, or pass -ApiKey.' }
    if (-not $resolvedModel) { throw 'Missing model. Set ELON_AGENT_MODEL or pass -Model.' }
    return [pscustomobject]@{
        Base = $resolvedBase.TrimEnd('/')
        Key = $resolvedKey
        Model = $resolvedModel
    }
}

function Resolve-ServerConfig {
    $resolvedUrl = if ($ServerUrl.Trim()) { $ServerUrl.Trim() } else { Get-FirstEnv @('ELON_SERVER_URL', 'ELON_AGENT_SERVER_URL') }
    $resolvedToken = if ($ServerToken.Trim()) { $ServerToken.Trim() } else { Get-FirstEnv @('ELON_SERVER_TOKEN', 'ELON_AGENT_SERVER_TOKEN', 'OWNER_TOKEN') }
    $resolvedAgent = if ($ServerAgent.Trim()) { $ServerAgent.Trim() } else { Get-FirstEnv @('ELON_SERVER_AGENT', 'ELON_AGENT_SERVER_AGENT') }
    if (-not $resolvedUrl) { $resolvedUrl = 'http://43.139.149.158:8080' }
    if (-not $resolvedToken) { $resolvedToken = Get-NodeAgentUserToken }
    if (-not $resolvedToken) { throw 'Missing server token. Install/login Windows client, set ELON_SERVER_TOKEN, or pass -ServerToken.' }
    return [pscustomobject]@{
        Url = $resolvedUrl.TrimEnd('/')
        Token = $resolvedToken
        Agent = $resolvedAgent
    }
}

function New-SystemPrompt {
    return @'
You are the Route B local agent runtime for an Elon-managed project workspace.
Return strict JSON only, without markdown fences.

Schema:
{
  "message": "short human-readable progress or final answer",
  "done": false,
    "actions": [
    {"tool": "list_dir", "path": "."},
    {"tool": "search_files", "query": "TODO", "path": "src", "max_results": 40},
    {"tool": "file_info", "path": "src/main.rs"},
    {"tool": "read_file", "path": "README.md"},
    {"tool": "read_file_range", "path": "src/main.rs", "start_line": 1, "line_count": 80},
    {"tool": "git_status"},
    {"tool": "git_diff", "path": "src/main.rs", "cached": false, "stat": false},
    {"tool": "git_log", "path": "src/main.rs", "limit": 20},
    {"tool": "write_file", "path": "docs/note.md", "content": "full content"},
    {"tool": "apply_patch", "patch": "unified diff", "check_only": false},
    {"tool": "run_command", "program": "cargo", "args": ["test"], "reason": "verify project tests"}
  ]
}

Rules:
- Use paths relative to the project root.
- Prefer read-only actions first.
- Use search_files before broad file reads when you need to locate symbols, filenames, TODOs, errors, or related code.
- Use file_info before reading unknown files, binary-looking files, or directories.
- Use read_file_range for large files or when you only need a specific section.
- Use git_status, git_diff, and git_log for read-only git inspection; do not spend run_command approvals on status/diff/log.
- Do not request destructive commands, privilege changes, downloads that execute code, persistence, credential access, or writes outside the project.
- Prefer apply_patch with unified diff for local edits to existing project files.
- Use write_file only for intentional new project files or full-file rewrites.
- Use run_command only for low-risk project checks such as cargo check/test, npm test/run lint, or Gradle test/assembleDebug.
- Prefer structured run_command with program and args. The legacy command string field exists only for older clients.
- There is a limited run_command budget per agent run. Choose commands carefully and stop after enough evidence.
- Set done=true when no further tool action is needed.
'@
}

function Invoke-ChatCompletion {
    param(
        [Parameter(Mandatory = $true)]$Config,
        [Parameter(Mandatory = $true)]$Messages
    )
    $body = @{
        model = $Config.Model
        messages = $Messages
        temperature = 0.2
    } | ConvertTo-Json -Depth 20

    $headers = @{
        Authorization = "Bearer $($Config.Key)"
    }
    Invoke-RestMethod -Method Post -Uri "$($Config.Base)/chat/completions" -Headers $headers -ContentType 'application/json' -Body $body -TimeoutSec 120
}

function Invoke-ServerChatCompletion {
    param(
        [Parameter(Mandatory = $true)]$Config,
        [Parameter(Mandatory = $true)]$Messages
    )
    $payload = @{
        messages = $Messages
    }
    if ($Config.Agent) {
        $payload.agent = $Config.Agent
    }
    $body = $payload | ConvertTo-Json -Depth 20
    $headers = @{
        Authorization = "Bearer $($Config.Token)"
    }
    Invoke-RestMethod -Method Post -Uri "$($Config.Url)/api/agent/runtime/chat" -Headers $headers -ContentType 'application/json' -Body $body -TimeoutSec 120
}

function Get-AssistantContent {
    param([Parameter(Mandatory = $true)]$Response)
    $content = $Response.choices[0].message.content
    if (-not $content) {
        throw 'API response did not contain choices[0].message.content.'
    }
    return [string]$content
}

function ConvertFrom-AgentJson {
    param([Parameter(Mandatory = $true)][string]$Content)
    $text = $Content.Trim()
    try {
        return $text | ConvertFrom-Json
    } catch {
        $start = $text.IndexOf('{')
        $end = $text.LastIndexOf('}')
        if ($start -ge 0 -and $end -gt $start) {
            return $text.Substring($start, $end - $start + 1) | ConvertFrom-Json
        }
        throw "Agent response was not JSON: $($Content.Substring(0, [Math]::Min(300, $Content.Length)))"
    }
}

function Resolve-SafePath {
    param([Parameter(Mandatory = $true)][string]$RelativePath)
    $raw = $RelativePath.Trim()
    if (-not $raw) { throw 'Path cannot be empty.' }
    if ([System.IO.Path]::IsPathRooted($raw)) { throw "Absolute paths are not allowed: $raw" }
    $parts = @($raw -split '[\\/]' | Where-Object { $_ -and $_ -ne '.' })
    foreach ($part in $parts) {
        if ($part -eq '..') { throw "Parent path segments are not allowed: $raw" }
        if ($part.Equals('.git', [System.StringComparison]::OrdinalIgnoreCase)) {
            throw "Path cannot target .git: $raw"
        }
    }

    $rootFull = [System.IO.Path]::GetFullPath($ProjectRoot)
    $rootPrefix = $rootFull
    if (-not $rootPrefix.EndsWith([System.IO.Path]::DirectorySeparatorChar)) {
        $rootPrefix = $rootPrefix + [System.IO.Path]::DirectorySeparatorChar
    }

    $full = [System.IO.Path]::GetFullPath((Join-Path $ProjectRoot $raw))
    if ($full -ne $rootFull -and -not $full.StartsWith($rootPrefix, [System.StringComparison]::OrdinalIgnoreCase)) {
        throw "Path escapes project root: $raw"
    }
    $current = $rootFull
    foreach ($part in $parts) {
        $current = Join-Path $current $part
        if (Test-Path -LiteralPath $current) {
            $item = Get-Item -LiteralPath $current -Force
            if (($item.Attributes -band [System.IO.FileAttributes]::ReparsePoint) -ne 0) {
                throw "Path crosses a symlink or junction: $raw"
            }
        }
    }
    return $full
}

function Confirm-AgentAction {
    param(
        [Parameter(Mandatory = $true)][string]$Kind,
        [Parameter(Mandatory = $true)][string]$Target
    )
    if ($Yes) { return $true }
    Write-Host ''
    Write-Host "Agent requests $Kind`: $Target"
    $answer = Read-Host 'Type yes to allow'
    return @('yes', 'y') -contains $answer.Trim().ToLowerInvariant()
}

function Limit-AgentText {
    param(
        [AllowNull()][string]$Text,
        [Parameter(Mandatory = $true)][int]$MaxChars
    )
    if ($null -eq $Text) { return '' }
    if ($Text.Length -le $MaxChars) { return $Text }
    return $Text.Substring(0, $MaxChars) + "`n[truncated after $MaxChars chars]"
}

function Use-AgentRunCommandBudget {
    if ($Script:AgentRunCommandCount -ge $MaxRunCommands) {
        return $false
    }
    $Script:AgentRunCommandCount += 1
    return $true
}

# __ELON_LIFECYCLE_HELPERS__

# __ELON_CONTEXT_HELPERS__

# __ELON_APPLY_PATCH_HELPERS__

function Test-AgentCommandAllowed {
    param([Parameter(Mandatory = $true)][string]$Command)
    $trimmed = $Command.Trim()
    $lower = $trimmed.ToLowerInvariant()
    if (-not $lower) { return $false }
    $shellMarkers = @(';', '&&', '||', '|', "`n", "`r", '>', '<', '$', '`')
    foreach ($marker in $shellMarkers) {
        if ($lower.Contains($marker)) { return $false }
    }
    if ([regex]::IsMatch($trimmed, '(^|\s|")([a-zA-Z]:[\\/]|\\\\)')) { return $false }
    $blockedPatterns = @(
        'remove-item', ' del ', ' rmdir ', 'format ', 'shutdown', 'restart-computer',
        'set-executionpolicy', 'reg delete', 'sc delete', 'takeown', 'icacls',
        'invoke-webrequest', ' iwr ', 'curl ', 'invoke-expression',
        'start-process', 'powershell', 'pwsh', 'cmd ', 'cmd.exe'
    )
    foreach ($pattern in $blockedPatterns) {
        if ($lower.Contains($pattern)) { return $false }
    }

    $allowedPrefixes = @(
        'git status', 'git diff', 'git log', 'cargo check', 'cargo test',
        'npm test', 'npm run lint', 'npm run test', '.\gradlew.bat test',
        '.\gradlew.bat :app:assembledebug', 'gradle test'
    )
    foreach ($prefix in $allowedPrefixes) {
        if ($lower.StartsWith($prefix)) { return $true }
    }
    return $false
}

function Test-AgentArgSafe {
    param([Parameter(Mandatory = $true)][string]$Arg)
    $trimmed = $Arg.Trim()
    if (-not $trimmed) { return $false }
    $lower = $trimmed.ToLowerInvariant()
    $shellMarkers = @(';', '&&', '||', '|', "`n", "`r", '>', '<', '$', '`')
    foreach ($marker in $shellMarkers) {
        if ($lower.Contains($marker)) { return $false }
    }
    if ([regex]::IsMatch($trimmed, '(^|\s|")([a-zA-Z]:[\\/]|\\\\)')) { return $false }
    $normalized = $lower.Replace('\', '/')
    if ($normalized -match '(^|/)(\.\.|\.git)(/|$)') { return $false }
    return $true
}

function Test-AgentFirstArgIn {
    param(
        [Parameter(Mandatory = $true)][string[]]$Args,
        [Parameter(Mandatory = $true)][string[]]$Allowed
    )
    if ($Args.Count -lt 1) { return $false }
    return $Allowed -contains $Args[0]
}

function Test-AgentPackageArgsAllowed {
    param(
        [Parameter(Mandatory = $true)][string[]]$Args,
        [switch]$RunRequired
    )
    if ($Args.Count -lt 1) { return $false }
    if (-not $RunRequired -and $Args[0] -eq 'test') { return $true }
    $scripts = @('lint', 'test', 'build', 'check', 'format', 'typecheck')
    return $Args.Count -ge 2 -and $Args[0] -eq 'run' -and ($scripts -contains $Args[1])
}

function Test-AgentGitArgsAllowed {
    param([Parameter(Mandatory = $true)][string[]]$Args)
    if ($Args.Count -lt 1) { return $false }
    $first = $Args[0]
    $simple = @('status', 'diff', 'log', 'show', 'branch', 'remote', 'fetch', 'add', 'commit', 'push')
    if ($simple -contains $first) { return $true }
    return $first -eq 'pull' -and ($Args -contains '--ff-only')
}

function Test-AgentCommandAllowedParts {
    param(
        [Parameter(Mandatory = $true)][string]$Program,
        [string[]]$Args = @()
    )
    $programName = $Program.Trim().ToLowerInvariant()
    $allowedPrograms = @('git', 'cargo', 'rustfmt', 'npm', 'pnpm', 'yarn', 'bun', 'python', 'pytest', 'go', 'dotnet', 'gradle', '.\gradlew.bat', './gradlew', './gradlew.bat', 'gradlew.bat')
    if (-not ($allowedPrograms -contains $programName)) { return $false }
    foreach ($arg in $Args) {
        if (-not (Test-AgentArgSafe $arg)) { return $false }
    }

    switch ($programName) {
        'git' { return Test-AgentGitArgsAllowed $Args }
        'cargo' { return Test-AgentFirstArgIn $Args @('check', 'test', 'build', 'fmt', 'clippy', 'run') }
        'rustfmt' { return $Args.Count -gt 0 }
        'npm' { return Test-AgentPackageArgsAllowed $Args }
        'pnpm' { return Test-AgentPackageArgsAllowed $Args -RunRequired }
        'yarn' { return Test-AgentPackageArgsAllowed $Args -RunRequired }
        'bun' { return Test-AgentPackageArgsAllowed $Args -RunRequired }
        'python' { return ($Args.Count -ge 2) -and ($Args[0] -eq '-m') -and (@('pytest', 'unittest') -contains $Args[1]) }
        'pytest' { return $true }
        'go' { return Test-AgentFirstArgIn $Args @('test', 'vet', 'build') }
        'dotnet' { return Test-AgentFirstArgIn $Args @('test', 'build') }
        default { return Test-AgentFirstArgIn $Args @('test', 'build', 'testDebugUnitTest', ':app:assembleDebug') }
    }
}

function Invoke-AgentSearchFiles {
    param(
        [string]$Path = '.',
        [Parameter(Mandatory = $true)][string]$Query,
        [int]$MaxResults = 50
    )
    $queryText = $Query.Trim()
    if (-not $queryText) { return 'search_files error: query cannot be empty' }
    if ($queryText.Length -gt 200) { return 'search_files error: query is too long' }
    if ($MaxResults -lt 1) { $MaxResults = 1 }
    if ($MaxResults -gt 200) { $MaxResults = 200 }

    $root = Resolve-SafePath $Path
    if (-not (Test-Path -LiteralPath $root -PathType Container)) {
        return "search_files error: directory not found: $Path"
    }

    $projectRootFull = [System.IO.Path]::GetFullPath($ProjectRoot)
    $projectRootPrefix = $projectRootFull
    if (-not $projectRootPrefix.EndsWith([System.IO.Path]::DirectorySeparatorChar)) {
        $projectRootPrefix = $projectRootPrefix + [System.IO.Path]::DirectorySeparatorChar
    }
    $skipDirs = @('.git', '.hg', '.svn', 'node_modules', 'target', 'dist', 'build', '.gradle', '.idea', '.vscode', '.elon')
    $queue = New-Object 'System.Collections.Generic.Queue[string]'
    $results = New-Object 'System.Collections.Generic.List[string]'
    $queue.Enqueue($root)
    $filesScanned = 0
    $truncated = $false

    while ($queue.Count -gt 0) {
        $dir = $queue.Dequeue()
        $items = @(Get-ChildItem -LiteralPath $dir -Force -ErrorAction SilentlyContinue)
        foreach ($item in $items) {
            if (($item.Attributes -band [System.IO.FileAttributes]::ReparsePoint) -ne 0) { continue }
            if ($item.PSIsContainer) {
                if ($skipDirs -contains $item.Name) { continue }
                $queue.Enqueue($item.FullName)
                continue
            }
            $filesScanned += 1
            if ($filesScanned -gt 2000) {
                $truncated = $true
                break
            }

            $full = [System.IO.Path]::GetFullPath($item.FullName)
            $relative = $full
            if ($full.StartsWith($projectRootPrefix, [System.StringComparison]::OrdinalIgnoreCase)) {
                $relative = $full.Substring($projectRootPrefix.Length)
            }
            $relative = $relative.Replace('\', '/')

            if ($relative.IndexOf($queryText, [System.StringComparison]::OrdinalIgnoreCase) -ge 0) {
                $results.Add("$relative`: path match")
            }
            if ($results.Count -ge $MaxResults) {
                $truncated = $true
                break
            }

            if ($item.Length -gt 1048576) { continue }
            try {
                $lineNo = 0
                foreach ($line in Get-Content -LiteralPath $full -ErrorAction Stop) {
                    $lineNo += 1
                    $text = [string]$line
                    if ($text.IndexOf($queryText, [System.StringComparison]::OrdinalIgnoreCase) -lt 0) { continue }
                    $snippet = (Limit-AgentText $text.Trim() 240).Replace("`t", ' ')
                    $results.Add("$relative`:$lineNo`: $snippet")
                    if ($results.Count -ge $MaxResults) {
                        $truncated = $true
                        break
                    }
                }
            } catch {
                continue
            }
            if ($results.Count -ge $MaxResults) { break }
        }
        if ($truncated -or $results.Count -ge $MaxResults) { break }
    }

    if ($results.Count -eq 0) { return "no matches for query: $queryText" }
    if ($truncated) { $results.Add('[truncated]') }
    return ($results -join "`n")
}

function Invoke-AgentFileInfo {
    param([Parameter(Mandatory = $true)][string]$Path)
    $full = Resolve-SafePath $Path
    if (-not (Test-Path -LiteralPath $full)) {
        return "file_info error: path not found: $Path"
    }
    $item = Get-Item -LiteralPath $full -Force
    $kind = if ($item.PSIsContainer) { 'dir' } elseif ($item -is [System.IO.FileInfo]) { 'file' } else { 'other' }
    $rows = New-Object 'System.Collections.Generic.List[string]'
    $rows.Add("file_info ok: $($Path.Trim())")
    $rows.Add("kind=$kind")
    $rows.Add("modified_utc=$($item.LastWriteTimeUtc.ToString('o'))")

    if ($kind -eq 'file') {
        $rows.Add("bytes=$($item.Length)")
        if ($item.Length -gt 1048576) {
            $rows.Add('line_probe=skipped_large_file')
            $rows.Add('line_probe_max_bytes=1048576')
            $rows.Add('advice=use read_file_range only if you know the needed line span')
        } else {
            try {
                $text = Get-Content -LiteralPath $full -Raw -ErrorAction Stop
                if ($null -eq $text) { $text = '' }
                $lineCount = if ($text.Length -eq 0) { 0 } else { @($text -split "`r`n|`n|`r").Count }
                if ($lineCount -gt 0 -and $text -match "(`r`n|`n|`r)$") { $lineCount -= 1 }
                $rows.Add('line_probe=text')
                $rows.Add("line_count=$lineCount")
                $rows.Add('advice=use read_file_range before broad reads on large files')
            } catch {
                $rows.Add('line_probe=non_text_or_unreadable')
                $rows.Add('advice=do not use read_file on binary or unreadable files')
            }
        }
    } elseif ($kind -eq 'dir') {
        $items = @(Get-ChildItem -LiteralPath $full -Force -ErrorAction SilentlyContinue | Select-Object -First 201)
        $sampled = [Math]::Min($items.Count, 200)
        $rows.Add("entries_sampled=$sampled")
        $rows.Add("entries_truncated=$($items.Count -gt 200)")
        $rows.Add('advice=use list_dir to inspect names in this directory')
    }

    return ($rows -join "`n")
}

function Invoke-AgentGitStatus {
    $output = & git -c core.quotepath=false status --short --branch 2>&1 | Out-String
    $text = "git -c core.quotepath=false status --short --branch exit=$LASTEXITCODE`n$output"
    return Limit-AgentText $text $AgentCommandOutputMaxChars
}

function Invoke-AgentGitDiff {
    param(
        [string]$Path = '.',
        [bool]$Cached = $false,
        [bool]$Stat = $false
    )
    $args = @('-c', 'core.quotepath=false', 'diff', '--no-ext-diff')
    if ($Cached) { $args += '--cached' }
    if ($Stat) { $args += '--stat' }
    $pathText = if ($Path) { $Path.Trim() } else { '' }
    if ($pathText -and $pathText -ne '.') {
        $full = Resolve-SafePath $pathText
        $rootFull = [System.IO.Path]::GetFullPath($ProjectRoot)
        $rootPrefix = $rootFull
        if (-not $rootPrefix.EndsWith([System.IO.Path]::DirectorySeparatorChar)) {
            $rootPrefix = $rootPrefix + [System.IO.Path]::DirectorySeparatorChar
        }
        $fullPath = [System.IO.Path]::GetFullPath($full)
        if ($fullPath -eq $rootFull) {
            $relative = '.'
        } elseif ($fullPath.StartsWith($rootPrefix, [System.StringComparison]::OrdinalIgnoreCase)) {
            $relative = $fullPath.Substring($rootPrefix.Length)
        } else {
            $relative = $pathText
        }
        $relative = $relative.Replace('\', '/')
        $args += '--'
        $args += $relative
    }
    $output = & git @args 2>&1 | Out-String
    $text = "git $($args -join ' ') exit=$LASTEXITCODE`n$output"
    return Limit-AgentText $text $AgentCommandOutputMaxChars
}

function Invoke-AgentGitLog {
    param(
        [string]$Path = '.',
        [int]$Limit = 20
    )
    if ($Limit -lt 1) { $Limit = 1 }
    if ($Limit -gt 100) { $Limit = 100 }
    $args = @('-c', 'core.quotepath=false', 'log', '--oneline', '--decorate', "--max-count=$Limit")
    $pathText = if ($Path) { $Path.Trim() } else { '' }
    if ($pathText -and $pathText -ne '.') {
        $full = Resolve-SafePath $pathText
        $rootFull = [System.IO.Path]::GetFullPath($ProjectRoot)
        $rootPrefix = $rootFull
        if (-not $rootPrefix.EndsWith([System.IO.Path]::DirectorySeparatorChar)) {
            $rootPrefix = $rootPrefix + [System.IO.Path]::DirectorySeparatorChar
        }
        $fullPath = [System.IO.Path]::GetFullPath($full)
        if ($fullPath -eq $rootFull) {
            $relative = '.'
        } elseif ($fullPath.StartsWith($rootPrefix, [System.StringComparison]::OrdinalIgnoreCase)) {
            $relative = $fullPath.Substring($rootPrefix.Length)
        } else {
            $relative = $pathText
        }
        $relative = $relative.Replace('\', '/')
        $args += '--'
        $args += $relative
    }
    $output = & git @args 2>&1 | Out-String
    $text = "git $($args -join ' ') exit=$LASTEXITCODE`n$output"
    return Limit-AgentText $text $AgentCommandOutputMaxChars
}

function Invoke-AgentAction {
    param([Parameter(Mandatory = $true)]$Action)
    $tool = [string]$Action.tool
    switch ($tool) {
        'list_dir' {
            $path = if ($Action.path) { [string]$Action.path } else { '.' }
            $full = Resolve-SafePath $path
            if (-not (Test-Path -LiteralPath $full -PathType Container)) {
                return "list_dir error: directory not found: $path"
            }
            $items = Get-ChildItem -LiteralPath $full | Select-Object -First 200 Name, Mode, Length
            return ($items | Format-Table -AutoSize | Out-String)
        }
        'search_files' {
            $path = if ($Action.path) { [string]$Action.path } else { '.' }
            $query = [string]$Action.query
            $maxResults = if ($Action.max_results) { [int]$Action.max_results } else { 50 }
            return Invoke-AgentSearchFiles -Path $path -Query $query -MaxResults $maxResults
        }
        'file_info' {
            $path = [string]$Action.path
            return Invoke-AgentFileInfo -Path $path
        }
        'read_file' {
            $path = [string]$Action.path
            $full = Resolve-SafePath $path
            if (-not (Test-Path -LiteralPath $full -PathType Leaf)) {
                return "read_file error: file not found: $path"
            }
            $text = Get-Content -LiteralPath $full -Raw
            if ($text.Length -gt 20000) {
                return $text.Substring(0, 20000) + "`n[truncated]"
            }
            return $text
        }
        'read_file_range' {
            $path = [string]$Action.path
            $startLine = [int]$Action.start_line
            $lineCount = [int]$Action.line_count
            if ($startLine -lt 1) {
                return "read_file_range error: start_line must be >= 1"
            }
            if ($lineCount -lt 1) {
                return "read_file_range error: line_count must be >= 1"
            }
            if ($lineCount -gt 400) {
                $lineCount = 400
            }
            $full = Resolve-SafePath $path
            if (-not (Test-Path -LiteralPath $full -PathType Leaf)) {
                return "read_file_range error: file not found: $path"
            }
            $endLine = $startLine + $lineCount - 1
            $lines = @(Get-Content -LiteralPath $full -TotalCount $endLine)
            $selected = @($lines | Select-Object -Skip ($startLine - 1) -First $lineCount)
            if ($selected.Count -eq 0) {
                return "read_file_range empty: $path has no lines at or after $startLine"
            }
            $lineNo = $startLine
            $numbered = foreach ($line in $selected) {
                "{0}: {1}" -f $lineNo, $line
                $lineNo += 1
            }
            return ($numbered -join "`n")
        }
        'git_status' {
            return Invoke-AgentGitStatus
        }
        'git_diff' {
            $path = if ($Action.path) { [string]$Action.path } else { '.' }
            $cached = if ($Action.cached) { [bool]$Action.cached } else { $false }
            $stat = if ($Action.stat) { [bool]$Action.stat } else { $false }
            return Invoke-AgentGitDiff -Path $path -Cached $cached -Stat $stat
        }
        'git_log' {
            $path = if ($Action.path) { [string]$Action.path } else { '.' }
            $limit = if ($Action.limit) { [int]$Action.limit } else { 20 }
            return Invoke-AgentGitLog -Path $path -Limit $limit
        }
        'write_file' {
            $path = [string]$Action.path
            $content = [string]$Action.content
            $full = Resolve-SafePath $path
            if ($DryRun) {
                Write-Host "[dry-run] write_file $path"
                return "dry-run: would write $path ($($content.Length) chars)"
            }
            if (-not (Confirm-AgentAction 'write_file' $path)) {
                return "write_file denied by user: $path"
            }
            $parent = Split-Path -Parent $full
            if ($parent) { New-Item -ItemType Directory -Force -Path $parent | Out-Null }
            Set-Content -LiteralPath $full -Value $content -Encoding UTF8
            return "write_file ok: $path"
        }
# __ELON_APPLY_PATCH_ACTION__
        'run_command' {
            $program = [string]$Action.program
            $args = @()
            if ($Action.args) {
                $args = @($Action.args | ForEach-Object { [string]$_ })
            }
            if ($program.Trim()) {
                if (-not (Test-AgentCommandAllowedParts $program $args)) {
                    return "run_command denied by policy: $program $($args -join ' ')"
                }
                if ($DryRun) {
                    Write-Host "[dry-run] run_command $program $($args -join ' ')"
                    return "dry-run: would run $program $($args -join ' ')"
                }
                if (-not (Confirm-AgentAction 'run_command' "$program $($args -join ' ')")) {
                    return "run_command denied by user: $program $($args -join ' ')"
                }
                if (-not (Use-AgentRunCommandBudget)) {
                    return "run_command denied: command budget exhausted ($MaxRunCommands per agent run)"
                }
                $output = & $program @args 2>&1 | Out-String
            } else {
                $command = [string]$Action.command
                if (-not (Test-AgentCommandAllowed $command)) {
                    return "run_command denied by policy: $command"
                }
                if ($DryRun) {
                    Write-Host "[dry-run] run_command $command"
                    return "dry-run: would run $command"
                }
                if (-not (Confirm-AgentAction 'run_command' $command)) {
                    return "run_command denied by user: $command"
                }
                if (-not (Use-AgentRunCommandBudget)) {
                    return "run_command denied: command budget exhausted ($MaxRunCommands per agent run)"
                }
                $output = powershell -NoProfile -ExecutionPolicy Bypass -Command $command 2>&1 | Out-String
            }
            $output = Limit-AgentText $output $AgentCommandOutputMaxChars
            if ($LASTEXITCODE -ne 0) {
                return "run_command exit=$LASTEXITCODE`n$output"
            }
            return $output
        }
        default {
            return "unknown tool: $tool"
        }
    }
}

function Invoke-AgentRuntimeLoop {
    param(
        [Parameter(Mandatory = $true)][string]$Label,
        [Parameter(Mandatory = $true)][scriptblock]$ChatCompletion
    )
    Require-Prompt
    Initialize-AgentRunLifecycle -Label $Label
    $messages = @(
        @{ role = 'system'; content = (New-SystemPrompt) },
        @{ role = 'user'; content = $Prompt }
    )

    try {
        for ($turn = 1; $turn -le $MaxTurns; $turn++) {
            Write-AgentRunEvent -Type 'turn_started' -Data ([ordered]@{ turn = $turn })
            Write-Host "[$Label] turn $turn"
            $response = & $ChatCompletion -Messages $messages
            $content = Get-AssistantContent $response
            $messages += @{ role = 'assistant'; content = $content }
            $agent = ConvertFrom-AgentJson $content

            if ($agent.message) {
                Write-Host ([string]$agent.message)
            }

            $actions = @($agent.actions)
            Write-AgentRunEvent -Type 'assistant_response' -Data ([ordered]@{
                turn = $turn
                done = [bool]$agent.done
                message_chars = if ($agent.message) { ([string]$agent.message).Length } else { 0 }
                action_count = $actions.Count
            })

            if ($actions.Count -eq 0) {
                Complete-AgentRunLifecycle -Status completed -Data ([ordered]@{
                    turn = $turn
                    reason = 'no_actions'
                })
                return
            }

            $results = @()
            foreach ($action in $actions) {
                $tool = [string]$action.tool
                $target = Get-AgentActionTarget $action
                Write-Host "[tool] $tool"
                Write-AgentRunEvent -Type 'tool_started' -Data ([ordered]@{
                    turn = $turn
                    tool = $tool
                    target = $target
                })
                try {
                    $result = Invoke-AgentAction $action
                    Write-AgentRunEvent -Type 'tool_finished' -Data ([ordered]@{
                        turn = $turn
                        tool = $tool
                        target = $target
                        result_chars = if ($null -eq $result) { 0 } else { ([string]$result).Length }
                    })
                } catch {
                    Write-AgentRunEvent -Type 'tool_failed' -Data ([ordered]@{
                        turn = $turn
                        tool = $tool
                        target = $target
                        error = $_.Exception.Message
                    })
                    throw
                }
                $results += [pscustomobject]@{
                    tool = $tool
                    result = $result
                }
            }

            $messages += @{
                role = 'user'
                content = "Tool results JSON:`n" + ($results | ConvertTo-Json -Depth 8)
            }
            $compression = Compress-AgentRuntimeMessages -Messages $messages -Turn $turn
            $messages = @($compression.Messages)

            if ($agent.done -eq $true) {
                Complete-AgentRunLifecycle -Status completed -Data ([ordered]@{
                    turn = $turn
                    reason = 'done_true'
                })
                return
            }
        }

        throw "$Label stopped after MaxTurns=$MaxTurns without done=true"
    } catch {
        Complete-AgentRunLifecycle -Status failed -Data ([ordered]@{
            error = $_.Exception.Message
        })
        throw
    }
}

function Invoke-ApiRuntime {
    $config = Resolve-ApiConfig
    Invoke-AgentRuntimeLoop -Label 'api-runtime' -ChatCompletion {
        param([Parameter(Mandatory = $true)]$Messages)
        Invoke-ChatCompletion -Config $config -Messages $Messages
    }
}

function Invoke-ServerRuntime {
    $config = Resolve-ServerConfig
    Invoke-AgentRuntimeLoop -Label 'server-runtime' -ChatCompletion {
        param([Parameter(Mandatory = $true)]$Messages)
        Invoke-ServerChatCompletion -Config $config -Messages $Messages
    }
}

switch ($Mode) {
    'status' { Show-AgentStatus }
    'cli-wrapper' { Invoke-CliWrapper }
    'api-runtime' { Invoke-ApiRuntime }
    'server-runtime' { Invoke-ServerRuntime }
}
"#
    .to_string()
    .replace(
        "# __ELON_APPLY_PATCH_HELPERS__",
        agent_runtime_apply_patch_helpers(),
    )
    .replace(
        "# __ELON_LIFECYCLE_HELPERS__",
        agent_runtime_lifecycle_helpers(),
    )
    .replace(
        "# __ELON_CONTEXT_HELPERS__",
        agent_runtime_context_helpers(),
    )
    .replace(
        "# __ELON_APPLY_PATCH_ACTION__",
        agent_runtime_apply_patch_action_case(),
    ))
}

#[cfg(test)]
mod tests {
    use super::{agent_runtime_doc, agent_runtime_script, ensure_project_agent_runtime_files};
    use crate::project_scaffold::ProjectScaffoldRequest;
    use std::{
        fs,
        process::Command,
        time::{SystemTime, UNIX_EPOCH},
    };

    #[test]
    fn agent_runtime_files_are_created_without_overwrite() {
        let root = temp_dir("agent_runtime_files_are_created_without_overwrite");
        fs::create_dir_all(root.join("scripts")).unwrap();
        fs::write(root.join("scripts").join("elon-agent.ps1"), "custom").unwrap();

        ensure_project_agent_runtime_files(&root, &request()).unwrap();

        assert_eq!(
            fs::read_to_string(root.join("scripts").join("elon-agent.ps1")).unwrap(),
            "custom"
        );
        assert!(root.join("docs").join("agent-runtime.md").exists());
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn agent_runtime_script_exposes_both_routes() {
        let script = agent_runtime_script().unwrap();
        assert!(script
            .contains("ValidateSet('status', 'cli-wrapper', 'api-runtime', 'server-runtime')"));
        assert!(script.contains("ValidateSet('codex', 'claude', 'gemini', 'copilot')"));
        assert!(script.contains("Invoke-CliWrapper"));
        assert!(script.contains("Invoke-ApiRuntime"));
        assert!(script.contains("Invoke-ServerRuntime"));
        assert!(script.contains("[string]$RunId = ''"));
        assert!(script.contains("/api/agent/runtime/chat"));
        assert!(script.contains("Run logs: .elon\\agent-runs"));
        assert!(script.contains("Initialize-AgentRunLifecycle"));
        assert!(script.contains("Write-AgentRunEvent"));
        assert!(script.contains("Get-AgentActionTarget"));
        assert!(script.contains("Complete-AgentRunLifecycle"));
        assert!(script.contains("'run_started'"));
        assert!(script.contains("'turn_started'"));
        assert!(script.contains("'tool_started'"));
        assert!(script.contains("'tool_finished'"));
        assert!(script.contains("'run_finished'"));
        assert!(script.contains("'run_failed'"));
        assert!(script.contains("result_chars"));
        assert!(script.contains("Resolve-SafePath"));
        assert!(script.contains("Parent path segments are not allowed"));
        assert!(script.contains("ReparsePoint"));
        assert!(script.contains("'search_files'"));
        assert!(script.contains("Invoke-AgentSearchFiles"));
        assert!(script.contains("search_files error: query cannot be empty"));
        assert!(script.contains("'file_info'"));
        assert!(script.contains("Invoke-AgentFileInfo"));
        assert!(script.contains("line_probe=skipped_large_file"));
        assert!(script.contains("'read_file_range'"));
        assert!(script.contains("start_line must be >= 1"));
        assert!(script.contains("$lineCount = 400"));
        assert!(script.contains("'git_status'"));
        assert!(script.contains("'git_diff'"));
        assert!(script.contains("'git_log'"));
        assert!(script.contains("Invoke-AgentGitStatus"));
        assert!(script.contains("Invoke-AgentGitDiff"));
        assert!(script.contains("Invoke-AgentGitLog"));
        assert!(script.contains("core.quotepath=false"));
        assert!(script.contains("'apply_patch'"));
        assert!(script.contains("Invoke-AgentApplyPatch"));
        assert!(script.contains("git apply"));
        assert!(script.contains("binary patches are not supported"));
        assert!(!script.contains("__ELON_APPLY_PATCH_HELPERS__"));
        assert!(!script.contains("__ELON_APPLY_PATCH_ACTION__"));
        assert!(script.contains("Test-AgentCommandAllowedParts"));
        assert!(script.contains("[int]$MaxRunCommands = 8"));
        assert!(script.contains("[int]$MaxContextChars = 60000"));
        assert!(script.contains("$Script:AgentRunCommandCount"));
        assert!(script.contains("$Script:AgentContextCompactionCount"));
        assert!(script.contains("Use-AgentRunCommandBudget"));
        assert!(script.contains("command budget exhausted"));
        assert!(script.contains("Compress-AgentRuntimeMessages"));
        assert!(script.contains("'context_compacted'"));
        assert!(script.contains("max_context_chars"));
        assert!(script.contains("$AgentCommandOutputMaxChars = 12000"));
        assert!(script.contains("Limit-AgentText $output $AgentCommandOutputMaxChars"));
        assert!(script.contains("\"tool\": \"git_status\""));
        assert!(script.contains("\"tool\": \"git_diff\""));
        assert!(script.contains("\"tool\": \"git_log\""));
        assert!(script.contains("\"program\": \"cargo\""));
        assert!(script.contains("$shellMarkers"));
        assert!(script.contains("[regex]::IsMatch"));
        assert!(!script.contains("__ELON_LIFECYCLE_HELPERS__"));
        assert!(!script.contains("__ELON_CONTEXT_HELPERS__"));
        assert!(!script.contains('\u{662f}'));
    }

    #[test]
    fn agent_runtime_doc_names_route_a_and_b() {
        let doc = agent_runtime_doc(&request()).unwrap();
        assert!(doc.contains("Route A"));
        assert!(doc.contains("Route B"));
        assert!(doc.contains("Route C"));
        assert!(doc.contains("ELON_AGENT_API_KEY"));
        assert!(doc.contains("ELON_SERVER_TOKEN"));
        assert!(doc.contains("ELON_SERVER_AGENT_RUNTIME_ALLOWED_AGENTS"));
        assert!(doc.contains("search_files"));
        assert!(doc.contains("file_info"));
        assert!(doc.contains("read_file_range"));
        assert!(doc.contains("git_status"));
        assert!(doc.contains("git_diff"));
        assert!(doc.contains("git_log"));
        assert!(doc.contains("apply_patch"));
        assert!(doc.contains("program"));
        assert!(doc.contains("args"));
        assert!(doc.contains("-MaxRunCommands"));
        assert!(doc.contains("-MaxContextChars"));
        assert!(doc.contains("context budget"));
        assert!(doc.contains("Task lifecycle logs"));
        assert!(doc.contains(".elon\\agent-runs\\"));
        assert!(doc.contains("does not expose the server API key"));
    }

    #[cfg(windows)]
    #[test]
    fn generated_agent_runtime_script_parses_and_applies_patch() {
        let root = temp_dir("generated_agent_runtime_script_parses_and_applies_patch");
        let scripts = root.join("scripts");
        fs::create_dir_all(&scripts).unwrap();
        fs::write(root.join("note.txt"), "hello\n").unwrap();
        fs::create_dir_all(root.join("src")).unwrap();
        fs::create_dir_all(root.join("target")).unwrap();
        fs::write(
            root.join("src").join("route_b.rs"),
            "fn route_b_search() {}\nlet marker = \"needle\";\n",
        )
        .unwrap();
        fs::write(root.join("target").join("ignored.txt"), "needle\n").unwrap();
        let script_path = scripts.join("elon-agent.ps1");
        fs::write(&script_path, agent_runtime_script().unwrap()).unwrap();
        let validation_path = root.join("validate-agent-runtime.ps1");

        let init = Command::new("git")
            .arg("init")
            .current_dir(&root)
            .output()
            .expect("git init should run");
        assert!(
            init.status.success(),
            "git init failed: {}",
            String::from_utf8_lossy(&init.stderr)
        );

        let command = r#"
param(
    [Parameter(Mandatory = $true)][string]$ScriptPath,
    [Parameter(Mandatory = $true)][string]$Root
)
$tokens = $null
$errors = $null
[System.Management.Automation.Language.Parser]::ParseFile($ScriptPath, [ref]$tokens, [ref]$errors) | Out-Null
if ($errors.Count -gt 0) {
    $errors | ForEach-Object { Write-Error $_.Message }
    exit 1
}
. $ScriptPath -Mode status -Yes -MaxContextChars 10000 | Out-Null
Initialize-AgentRunLifecycle -Label 'api-runtime-test'
Write-AgentRunEvent -Type 'turn_started' -Data ([ordered]@{ turn = 1 })
Write-AgentRunEvent -Type 'tool_started' -Data ([ordered]@{ turn = 1; tool = 'read_file'; target = 'note.txt' })
Write-AgentRunEvent -Type 'tool_finished' -Data ([ordered]@{ turn = 1; tool = 'read_file'; target = 'note.txt'; result_chars = 6 })
$messages = @(
    @{ role = 'system'; content = 'system' },
    @{ role = 'user'; content = 'prompt' },
    @{ role = 'assistant'; content = ('a' * 7000) },
    @{ role = 'user'; content = ('b' * 7000) },
    @{ role = 'assistant'; content = ('c' * 7000) }
)
$compressed = Compress-AgentRuntimeMessages -Messages $messages -Turn 2
if (-not $compressed.Compacted) {
    Write-Error 'expected context compaction'
    exit 1
}
if (@($compressed.Messages).Count -ge $messages.Count) {
    Write-Error 'expected compaction to replace omitted messages'
    exit 1
}
$serializedCompressed = $compressed.Messages | ConvertTo-Json -Depth 8
if ($serializedCompressed -match ('a' * 40) -or $serializedCompressed -match ('b' * 40)) {
    Write-Error 'compaction should not keep omitted old message body'
    exit 1
}
Complete-AgentRunLifecycle -Status completed -Data ([ordered]@{ turn = 1; reason = 'test' })
$logDir = Join-Path $Root '.elon\agent-runs'
$logs = @(Get-ChildItem -LiteralPath $logDir -Filter '*.jsonl')
if ($logs.Count -ne 1) {
    Write-Error "expected one lifecycle log, got $($logs.Count)"
    exit 1
}
$events = @(Get-Content -LiteralPath $logs[0].FullName | ForEach-Object { $_ | ConvertFrom-Json })
$types = @($events | ForEach-Object { [string]$_.type })
foreach ($expected in @('run_started', 'turn_started', 'tool_started', 'tool_finished', 'context_compacted', 'run_finished')) {
    if (-not ($types -contains $expected)) {
        Write-Error "missing lifecycle event: $expected"
        exit 1
    }
}
$eventsJson = $events | ConvertTo-Json -Depth 20
if ($eventsJson -match 'hello') {
    Write-Error 'lifecycle log should not store file content'
    exit 1
}
if ($eventsJson -match ('a' * 40) -or $eventsJson -match ('b' * 40) -or $eventsJson -match ('c' * 40)) {
    Write-Error 'lifecycle log should not store compacted message content'
    exit 1
}
$search = Invoke-AgentSearchFiles -Path '.' -Query 'needle' -MaxResults 20
if ($search -notmatch 'src/route_b.rs:2') {
    Write-Error "search_files did not find content match: $search"
    exit 1
}
if ($search -match 'target/ignored.txt') {
    Write-Error "search_files should skip target output: $search"
    exit 1
}
$pathSearch = Invoke-AgentSearchFiles -Path 'src' -Query 'route_b' -MaxResults 20
if ($pathSearch -notmatch 'src/route_b.rs: path match') {
    Write-Error "search_files did not find path match: $pathSearch"
    exit 1
}
$fileInfo = Invoke-AgentAction ([pscustomobject]@{ tool = 'file_info'; path = 'src/route_b.rs' })
if ($fileInfo -notmatch 'kind=file' -or $fileInfo -notmatch 'line_probe=text' -or $fileInfo -notmatch 'line_count=2') {
    Write-Error "file_info did not report file shape: $fileInfo"
    exit 1
}
$patch = @'
diff --git a/note.txt b/note.txt
--- a/note.txt
+++ b/note.txt
@@ -1 +1 @@
-hello
+hello patched
'@
$check = Invoke-AgentApplyPatch -Patch $patch -CheckOnly
if ($check -notlike 'apply_patch check ok:*') {
    Write-Error "unexpected check result: $check"
    exit 1
}
$before = Get-Content -LiteralPath (Join-Path $Root 'note.txt') -Raw
$beforeNormalized = $before.Replace("`r`n", "`n").Replace("`r", "`n")
if ($beforeNormalized -ne "hello`n") {
    Write-Error "check-only changed file: $before"
    exit 1
}
$applied = Invoke-AgentApplyPatch -Patch $patch
if ($applied -notlike 'apply_patch ok:*') {
    Write-Error "unexpected apply result: $applied"
    exit 1
}
$after = Get-Content -LiteralPath (Join-Path $Root 'note.txt') -Raw
$afterNormalized = $after.Replace("`r`n", "`n").Replace("`r", "`n")
if ($afterNormalized -ne "hello patched`n") {
    Write-Error "patch did not apply: $after"
    exit 1
}
"#;
        fs::write(&validation_path, command).unwrap();
        let output = Command::new("powershell")
            .args(["-NoProfile", "-ExecutionPolicy", "Bypass", "-File"])
            .arg(&validation_path)
            .arg(&script_path)
            .arg(&root)
            .output()
            .expect("powershell should run generated runtime script");
        assert!(
            output.status.success(),
            "powershell validation failed\nstdout:\n{}\nstderr:\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );

        let _ = fs::remove_dir_all(root);
    }

    fn request() -> ProjectScaffoldRequest<'static> {
        ProjectScaffoldRequest {
            project_id: "project-1",
            user_id: "user-1",
            name: "Demo App",
            template: "android",
            repo_url: None,
            branch: None,
        }
    }

    fn temp_dir(label: &str) -> std::path::PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!("elon-pc-dev-runtime-{label}-{nanos}"))
    }
}
