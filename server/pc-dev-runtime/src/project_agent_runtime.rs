use crate::project_scaffold::ProjectScaffoldRequest;
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

Route B is intentionally conservative. It only permits workspace-scoped `list_dir`, `read_file`, `write_file`, and a small allowlist of project commands. File writes and command execution require confirmation unless `-Yes` is provided. `-DryRun` previews writes and commands without applying them. This is not an OS sandbox: build/test commands can still execute project code, so only run it for projects you trust.

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
    [int]$MaxTurns = 6,
    [switch]$DryRun,
    [switch]$Yes
)

$ErrorActionPreference = 'Stop'

$ProjectRoot = (Resolve-Path -LiteralPath (Join-Path $PSScriptRoot '..')).Path
Set-Location $ProjectRoot

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
    {"tool": "read_file", "path": "README.md"},
    {"tool": "write_file", "path": "docs/note.md", "content": "full content"},
    {"tool": "run_command", "command": "git status --short", "reason": "inspect git state"}
  ]
}

Rules:
- Use paths relative to the project root.
- Prefer read-only actions first.
- Do not request destructive commands, privilege changes, downloads that execute code, persistence, credential access, or writes outside the project.
- Use write_file only for intentional project files.
- Use run_command only for low-risk project checks such as git status/diff/log, cargo check/test, npm test/run lint, or Gradle test/assembleDebug.
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
    return @('yes', 'y', '是') -contains $answer.Trim().ToLowerInvariant()
}

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
        'run_command' {
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
            $output = powershell -NoProfile -ExecutionPolicy Bypass -Command $command 2>&1 | Out-String
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
    $messages = @(
        @{ role = 'system'; content = (New-SystemPrompt) },
        @{ role = 'user'; content = $Prompt }
    )

    for ($turn = 1; $turn -le $MaxTurns; $turn++) {
        Write-Host "[$Label] turn $turn"
        $response = & $ChatCompletion -Messages $messages
        $content = Get-AssistantContent $response
        $messages += @{ role = 'assistant'; content = $content }
        $agent = ConvertFrom-AgentJson $content

        if ($agent.message) {
            Write-Host ([string]$agent.message)
        }

        $actions = @($agent.actions)
        if ($actions.Count -eq 0) {
            return
        }

        $results = @()
        foreach ($action in $actions) {
            $tool = [string]$action.tool
            Write-Host "[tool] $tool"
            $result = Invoke-AgentAction $action
            $results += [pscustomobject]@{
                tool = $tool
                result = $result
            }
        }

        $messages += @{
            role = 'user'
            content = "Tool results JSON:`n" + ($results | ConvertTo-Json -Depth 8)
        }

        if ($agent.done -eq $true) {
            return
        }
    }

    throw "$Label stopped after MaxTurns=$MaxTurns without done=true"
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
    .to_string())
}

#[cfg(test)]
mod tests {
    use super::{agent_runtime_doc, agent_runtime_script, ensure_project_agent_runtime_files};
    use crate::project_scaffold::ProjectScaffoldRequest;
    use std::{
        fs,
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
        assert!(script.contains("/api/agent/runtime/chat"));
        assert!(script.contains("Resolve-SafePath"));
        assert!(script.contains("Parent path segments are not allowed"));
        assert!(script.contains("ReparsePoint"));
        assert!(script.contains("$shellMarkers"));
        assert!(script.contains("[regex]::IsMatch"));
    }

    #[test]
    fn agent_runtime_doc_names_route_a_and_b() {
        let doc = agent_runtime_doc(&request()).unwrap();
        assert!(doc.contains("Route A"));
        assert!(doc.contains("Route B"));
        assert!(doc.contains("Route C"));
        assert!(doc.contains("ELON_AGENT_API_KEY"));
        assert!(doc.contains("ELON_SERVER_TOKEN"));
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
