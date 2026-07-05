use std::{env, ffi::OsString, path::Path};

#[cfg(windows)]
use std::{fs, io, path::PathBuf};

#[path = "node_agent_cli_tool_catalog.rs"]
mod tool_catalog;

#[cfg(windows)]
const NODE_COMMAND_SHIMS: &[&str] = &["npm", "npx", "pnpm", "yarn", "bun", "corepack"];

pub(crate) fn common_child_env_overrides(current_path: Option<OsString>) -> Vec<(String, String)> {
    let mut envs = Vec::new();

    #[cfg(windows)]
    {
        if let Ok(shim_dir) = ensure_command_shim_dir() {
            if let Some(path) = prepend_dirs_to_path(vec![shim_dir.clone()], current_path) {
                envs.push(("PATH".to_string(), path));
            }
            envs.push((
                "ELON_NODE_COMMAND_SHIM_DIR".to_string(),
                shim_dir.to_string_lossy().to_string(),
            ));
        }
    }

    #[cfg(not(windows))]
    let _ = current_path;

    envs
}

pub(crate) fn codex_child_env_overrides(
    codex_program: &str,
    current_path: Option<OsString>,
) -> Vec<(String, String)> {
    tool_catalog::codex_child_env_overrides(Path::new(codex_program), current_path)
}

pub(crate) fn apply_common_child_env_overrides(
    cmd: &mut tokio::process::Command,
    sidecar_env: &mut Vec<(String, String)>,
) {
    for (key, value) in common_child_env_overrides(env::var_os("PATH")) {
        apply_child_env_override(cmd, sidecar_env, key, value);
    }
}

pub(crate) fn apply_codex_child_env_overrides(
    cmd: &mut tokio::process::Command,
    sidecar_env: &mut Vec<(String, String)>,
    codex_program: &str,
) {
    let current_path = child_env_value(sidecar_env, "PATH").or_else(|| env::var_os("PATH"));
    for (key, value) in codex_child_env_overrides(codex_program, current_path) {
        apply_child_env_override(cmd, sidecar_env, key, value);
    }
}

fn apply_child_env_override(
    cmd: &mut tokio::process::Command,
    sidecar_env: &mut Vec<(String, String)>,
    key: String,
    value: String,
) {
    cmd.env(&key, &value);
    if let Some((_, existing_value)) = sidecar_env
        .iter_mut()
        .find(|(existing_key, _)| env_key_matches(existing_key, &key))
    {
        *existing_value = value;
    } else {
        sidecar_env.push((key, value));
    }
}

fn child_env_value(sidecar_env: &[(String, String)], key: &str) -> Option<OsString> {
    sidecar_env
        .iter()
        .rev()
        .find(|(existing_key, _)| env_key_matches(existing_key, key))
        .map(|(_, value)| OsString::from(value))
}

fn env_key_matches(left: &str, right: &str) -> bool {
    if cfg!(windows) {
        left.eq_ignore_ascii_case(right)
    } else {
        left == right
    }
}

#[cfg(windows)]
fn ensure_command_shim_dir() -> io::Result<PathBuf> {
    let shim_dir = command_shim_dir();
    fs::create_dir_all(&shim_dir)?;
    write_if_changed(
        &shim_dir.join("elon-command-shim.ps1"),
        command_shim_powershell_script(),
    )?;
    for tool in NODE_COMMAND_SHIMS {
        write_if_changed(
            &shim_dir.join(format!("{tool}.cmd")),
            &command_shim_batch_script(tool),
        )?;
    }
    Ok(shim_dir)
}

#[cfg(windows)]
fn command_shim_dir() -> PathBuf {
    env::var_os("LOCALAPPDATA")
        .or_else(|| env::var_os("APPDATA"))
        .map(PathBuf::from)
        .unwrap_or_else(env::temp_dir)
        .join("Elon")
        .join("node-agent")
        .join("command-shims")
}

#[cfg(windows)]
fn write_if_changed(path: &Path, content: &str) -> io::Result<()> {
    if fs::read_to_string(path).is_ok_and(|existing| existing == content) {
        return Ok(());
    }
    fs::write(path, content)
}

#[cfg(windows)]
fn command_shim_batch_script(tool: &str) -> String {
    format!(
        r#"@echo off
powershell -NoProfile -ExecutionPolicy Bypass -File "%~dp0elon-command-shim.ps1" {tool} %*
exit /b %ERRORLEVEL%
"#
    )
}

#[cfg(windows)]
fn command_shim_powershell_script() -> &'static str {
    r#"param(
    [Parameter(Mandatory = $true)][string]$Tool,
    [Parameter(ValueFromRemainingArguments = $true)][string[]]$ToolArgs = @()
)

$ErrorActionPreference = 'Stop'

function Get-NormalizedPathKey {
    param([string]$PathValue)
    if ([string]::IsNullOrWhiteSpace($PathValue)) { return '' }
    return $PathValue.TrimEnd([char[]]@('\', '/')).ToLowerInvariant()
}

$shimDir = Split-Path -Parent $PSCommandPath
$shimKey = Get-NormalizedPathKey $shimDir
$pathParts = @()
foreach ($part in ($env:PATH -split [System.IO.Path]::PathSeparator)) {
    if ([string]::IsNullOrWhiteSpace($part)) { continue }
    if ((Get-NormalizedPathKey $part) -eq $shimKey) { continue }
    $pathParts += $part
}
$env:PATH = ($pathParts -join [System.IO.Path]::PathSeparator)

foreach ($suffix in @('.cmd', '.exe', '.bat', '.com')) {
    $candidate = Get-Command "$Tool$suffix" -CommandType Application -ErrorAction SilentlyContinue | Select-Object -First 1
    if ($candidate -and $candidate.Source -and ($candidate.Source -notmatch '\.ps1$')) {
        & $candidate.Source @ToolArgs
        exit $LASTEXITCODE
    }
}

Write-Error "Elon command shim could not resolve $Tool without using a PowerShell .ps1 shim."
exit 9009
"#
}

#[cfg(windows)]
fn prepend_dirs_to_path(dirs: Vec<PathBuf>, current_path: Option<OsString>) -> Option<String> {
    let mut merged = Vec::new();
    for dir in dirs {
        if dir.is_dir() && !contains_path(&merged, &dir) {
            merged.push(dir);
        }
    }

    if let Some(path) = current_path {
        for dir in env::split_paths(&path) {
            if !contains_path(&merged, &dir) {
                merged.push(dir);
            }
        }
    }

    if merged.is_empty() {
        return None;
    }

    env::join_paths(merged)
        .ok()
        .map(|value| value.to_string_lossy().to_string())
}

#[cfg(windows)]
fn contains_path(paths: &[PathBuf], candidate: &Path) -> bool {
    let candidate_key = path_key(candidate);
    paths.iter().any(|path| path_key(path) == candidate_key)
}

#[cfg(windows)]
fn path_key(path: &Path) -> String {
    let value = path.to_string_lossy().to_string();
    if cfg!(windows) {
        value.to_ascii_lowercase()
    } else {
        value
    }
}

#[cfg(all(test, windows))]
mod tests {
    use super::{
        command_shim_batch_script, command_shim_powershell_script, common_child_env_overrides,
        path_key, NODE_COMMAND_SHIMS,
    };
    use std::{env, ffi::OsString, path::Path};

    #[test]
    fn node_command_shim_scripts_prefer_cmd_and_reject_ps1() {
        assert!(NODE_COMMAND_SHIMS.contains(&"npm"));
        let npm_wrapper = command_shim_batch_script("npm");
        assert!(npm_wrapper.contains("elon-command-shim.ps1\" npm %*"));

        let script = command_shim_powershell_script();
        assert!(script.contains("@('.cmd', '.exe', '.bat', '.com')"));
        assert!(script.contains("-notmatch '\\.ps1$'"));
    }

    #[test]
    fn common_child_env_overrides_prepends_node_command_shim_dir() {
        let envs = common_child_env_overrides(Some(OsString::from(r"C:\existing-bin")));
        let shim_dir = envs
            .iter()
            .find(|(key, _)| key == "ELON_NODE_COMMAND_SHIM_DIR")
            .map(|(_, value)| value)
            .expect("shim dir env");
        let path = envs
            .iter()
            .find(|(key, _)| key == "PATH")
            .map(|(_, value)| value)
            .expect("PATH env");
        let path_os = OsString::from(path);
        let first_path = env::split_paths(&path_os).next().expect("first PATH entry");

        assert_eq!(path_key(&first_path), path_key(Path::new(shim_dir)));
        assert!(Path::new(shim_dir).join("npm.cmd").is_file());
        assert!(Path::new(shim_dir).join("elon-command-shim.ps1").is_file());
    }
}
