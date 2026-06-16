use crate::project_scaffold::ProjectScaffoldRequest;
use std::{
    fs, io,
    path::{Path, PathBuf},
};

pub(crate) fn ensure_project_command_files(
    repo: &Path,
    _req: &ProjectScaffoldRequest<'_>,
) -> io::Result<()> {
    ensure_file(repo.join("scripts").join("elon.ps1"), local_cli_script)?;
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

fn local_cli_script() -> io::Result<String> {
    Ok(r#"param(
    [ValidateSet('env', 'status', 'task', 'agent', 'check', 'build', 'test')][string]$Action = 'status',

    [string]$TaskName = '',

    [string]$Base = '',

    [ValidateSet('status', 'cli-wrapper', 'api-runtime')][string]$AgentMode = 'status',

    [ValidateSet('codex', 'claude', 'gemini', 'copilot')][string]$Cli = 'codex',

    [string]$Prompt = '',

    [string]$ApiBase = '',

    [string]$ApiKey = '',

    [string]$Model = '',

    [switch]$DryRun,

    [switch]$Yes
)

$ErrorActionPreference = 'Stop'

$ProjectRoot = (Resolve-Path -LiteralPath (Join-Path $PSScriptRoot '..')).Path
Set-Location $ProjectRoot

function Invoke-External {
    param(
        [Parameter(Mandatory = $true)][string]$File,
        [string[]]$CommandArgs = @()
    )
    $rendered = @($File) + $CommandArgs
    Write-Host "> $($rendered -join ' ')"
    & $File @CommandArgs
    if ($LASTEXITCODE -ne 0) {
        throw "$File failed with exit code $LASTEXITCODE"
    }
}

function Test-Tool {
    param([Parameter(Mandatory = $true)][string]$Name)
    return [bool](Get-Command $Name -ErrorAction SilentlyContinue)
}

function Test-NpmScript {
    param([Parameter(Mandatory = $true)][string]$ScriptName)
    if (-not (Test-Path -LiteralPath 'package.json')) {
        return $false
    }
    try {
        $pkg = Get-Content -LiteralPath 'package.json' -Raw | ConvertFrom-Json
        if (-not $pkg.scripts) {
            return $false
        }
        return $pkg.scripts.PSObject.Properties.Name -contains $ScriptName
    } catch {
        return $false
    }
}

function Invoke-EnvCheck {
    $script = Join-Path $PSScriptRoot 'elon-dev-check.ps1'
    if (-not (Test-Path -LiteralPath $script)) {
        throw "Missing environment check script: $script"
    }
    & $script
}

function Invoke-Status {
    if (-not (Test-Tool 'git')) {
        throw 'Git is required for project status.'
    }
    Invoke-External 'git' @('status', '--short', '--branch')
}

function Invoke-Task {
    if (-not $TaskName.Trim()) {
        throw 'Task name is required. Example: scripts\elon.ps1 task -TaskName login-page'
    }
    $script = Join-Path $PSScriptRoot 'elon-new-task.ps1'
    if (-not (Test-Path -LiteralPath $script)) {
        throw "Missing task worktree script: $script"
    }
    & $script -Name $TaskName -Base $Base
}

function Invoke-Agent {
    $script = Join-Path $PSScriptRoot 'elon-agent.ps1'
    if (-not (Test-Path -LiteralPath $script)) {
        throw "Missing agent runtime script: $script"
    }
    $agentArgs = @('-Mode', $AgentMode, '-Cli', $Cli)
    if ($Prompt.Trim()) { $agentArgs += @('-Prompt', $Prompt) }
    if ($ApiBase.Trim()) { $agentArgs += @('-ApiBase', $ApiBase) }
    if ($ApiKey.Trim()) { $agentArgs += @('-ApiKey', $ApiKey) }
    if ($Model.Trim()) { $agentArgs += @('-Model', $Model) }
    if ($DryRun) { $agentArgs += '-DryRun' }
    if ($Yes) { $agentArgs += '-Yes' }
    & $script @agentArgs
}

function Invoke-Check {
    if (Test-Path -LiteralPath 'gradlew.bat') {
        Invoke-External '.\gradlew.bat' @(':app:assembleDebug')
    } elseif (Test-Path -LiteralPath 'Cargo.toml') {
        Invoke-External 'cargo' @('check')
    } elseif (Test-Path -LiteralPath 'package.json') {
        if (Test-NpmScript 'lint') {
            Invoke-External 'npm' @('run', 'lint')
        } elseif (Test-NpmScript 'test') {
            Invoke-External 'npm' @('test')
        } else {
            Write-Host 'No npm lint/test script found.'
        }
    } else {
        Write-Host 'No supported check command found for this project.'
    }
}

function Invoke-Build {
    if (Test-Path -LiteralPath 'gradlew.bat') {
        Invoke-External '.\gradlew.bat' @(':app:assembleDebug')
    } elseif (Test-Path -LiteralPath 'Cargo.toml') {
        Invoke-External 'cargo' @('build')
    } elseif (Test-NpmScript 'build') {
        Invoke-External 'npm' @('run', 'build')
    } else {
        Write-Host 'No supported build command found for this project.'
    }
}

function Invoke-Test {
    if (Test-Path -LiteralPath 'gradlew.bat') {
        Invoke-External '.\gradlew.bat' @('testDebugUnitTest')
    } elseif (Test-Path -LiteralPath 'Cargo.toml') {
        Invoke-External 'cargo' @('test')
    } elseif (Test-NpmScript 'test') {
        Invoke-External 'npm' @('test')
    } else {
        Write-Host 'No supported test command found for this project.'
    }
}

switch ($Action) {
    'env' { Invoke-EnvCheck }
    'status' { Invoke-Status }
    'task' { Invoke-Task }
    'agent' { Invoke-Agent }
    'check' { Invoke-Check }
    'build' { Invoke-Build }
    'test' { Invoke-Test }
}
"#
    .to_string())
}

#[cfg(test)]
mod tests {
    use super::{ensure_project_command_files, local_cli_script};
    use crate::project_scaffold::ProjectScaffoldRequest;
    use std::{
        fs,
        time::{SystemTime, UNIX_EPOCH},
    };

    #[test]
    fn command_files_are_created_without_overwrite() {
        let root = temp_dir("command_files_are_created_without_overwrite");
        fs::create_dir_all(root.join("scripts")).unwrap();
        fs::write(root.join("scripts").join("elon.ps1"), "custom").unwrap();

        ensure_project_command_files(&root, &request()).unwrap();

        assert_eq!(
            fs::read_to_string(root.join("scripts").join("elon.ps1")).unwrap(),
            "custom"
        );
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn local_cli_script_dispatches_core_commands() {
        let script = local_cli_script().unwrap();
        assert!(script
            .contains("ValidateSet('env', 'status', 'task', 'agent', 'check', 'build', 'test')"));
        assert!(script.contains("elon-dev-check.ps1"));
        assert!(script.contains("elon-new-task.ps1"));
        assert!(script.contains("elon-agent.ps1"));
        assert!(script.contains("AgentMode"));
        assert!(script.contains("assembleDebug"));
        assert!(script.contains("cargo"));
        assert!(script.contains("npm"));
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
