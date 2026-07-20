use std::{
    env,
    ffi::OsString,
    path::{Path, PathBuf},
};

#[cfg(windows)]
use std::{fs, io};

#[path = "node_agent_cli_tool_catalog.rs"]
mod tool_catalog;

#[cfg(windows)]
const NODE_COMMAND_SHIMS: &[&str] = &["npm", "npx", "pnpm", "yarn", "bun", "corepack"];

pub(crate) fn cli_child_env_overrides(
    cli_name: &str,
    cli_program: &str,
    cwd: Option<&str>,
) -> Vec<(String, String)> {
    cli_child_env_overrides_with_path(cli_name, cli_program, cwd, env::var_os("PATH"))
}

pub(crate) fn apply_env(
    cmd: &mut tokio::process::Command,
    sidecar_env: &mut Vec<(String, String)>,
    task_id: &str,
    cli_name: &str,
    cli_program: &str,
    cwd: Option<&str>,
) {
    // Desktop review authority belongs to the supervising desktop session, not
    // to the PC executor. Never let a spawned CLI or its sidecar inherit it.
    cmd.env_remove(crate::node_agent_desktop_review_auth::DESKTOP_REVIEW_CREDENTIAL_ENV);
    for (key, value) in cli_child_env_overrides(cli_name, cli_program, cwd) {
        cmd.env(&key, &value);
        sidecar_env.push((key, value));
    }
    if let Some(build_environment) = crate::node_agent_build_runtime::cli_run_environment(task_id) {
        build_environment.apply_tokio(cmd);
        build_environment.merge_into(sidecar_env);
    }
}

fn cli_child_env_overrides_with_path(
    cli_name: &str,
    cli_program: &str,
    cwd: Option<&str>,
    current_path: Option<OsString>,
) -> Vec<(String, String)> {
    let mut envs = common_child_env_overrides(current_path.clone());
    let common_path = child_env_value(&envs, "PATH").or(current_path);

    let router = project_tool_router_bin(cwd);
    let router_dirs = router.iter().cloned().collect::<Vec<_>>();
    let path_with_router = router
        .as_ref()
        .and_then(|_| tool_catalog::prepend_dirs_to_path(router_dirs, common_path.clone()));
    let merged_current_path = path_with_router.clone().map(OsString::from).or(common_path);

    if cli_name == "codex" {
        merge_child_env_overrides(
            &mut envs,
            tool_catalog::codex_child_env_overrides(Path::new(cli_program), merged_current_path),
        );
    } else if let Some(path) = path_with_router {
        set_child_env_override(&mut envs, "PATH".to_string(), path);
    }

    if router.is_some() {
        if let Some(root) = project_root(cwd) {
            set_child_env_override(
                &mut envs,
                "ELON_ROUTER_PROJECT_ROOT".to_string(),
                root.to_string_lossy().to_string(),
            );
        }
    }
    envs
}

pub(crate) fn common_child_env_overrides(current_path: Option<OsString>) -> Vec<(String, String)> {
    #[cfg(not(windows))]
    {
        let _ = current_path;
        Vec::new()
    }

    #[cfg(windows)]
    {
        let mut envs = Vec::new();

        if let Ok(shim_dir) = ensure_command_shim_dir() {
            if let Some(path) = prepend_dirs_to_path(vec![shim_dir.clone()], current_path) {
                envs.push(("PATH".to_string(), path));
            }
            envs.push((
                "ELON_NODE_COMMAND_SHIM_DIR".to_string(),
                shim_dir.to_string_lossy().to_string(),
            ));
        }

        envs
    }
}

fn merge_child_env_overrides(envs: &mut Vec<(String, String)>, overrides: Vec<(String, String)>) {
    for (key, value) in overrides {
        set_child_env_override(envs, key, value);
    }
}

fn set_child_env_override(envs: &mut Vec<(String, String)>, key: String, value: String) {
    if let Some((_, existing_value)) = envs
        .iter_mut()
        .find(|(existing_key, _)| env_key_matches(existing_key, &key))
    {
        *existing_value = value;
    } else {
        envs.push((key, value));
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

fn project_tool_router_bin(cwd: Option<&str>) -> Option<PathBuf> {
    let root = project_root(cwd)?;
    let router = root.join("scripts").join("tool-router-bin");
    router.is_dir().then_some(router)
}

fn project_root(cwd: Option<&str>) -> Option<PathBuf> {
    cwd.map(str::trim)
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        fs,
        time::{SystemTime, UNIX_EPOCH},
    };

    #[test]
    fn generic_cli_gets_project_tool_router_on_path() {
        let root = unique_temp_dir("route-a-router");
        let router = root.join("scripts").join("tool-router-bin");
        let original = root.join("original-path");
        fs::create_dir_all(&router).unwrap();
        fs::create_dir_all(&original).unwrap();
        let current_path = env::join_paths([original.clone()]).unwrap();

        let envs = cli_child_env_overrides_with_path(
            "copilot",
            "copilot",
            Some(root.to_str().unwrap()),
            Some(current_path),
        );
        let path = envs
            .iter()
            .find_map(|(key, value)| (key == "PATH").then_some(value))
            .expect("PATH should be overridden");
        let parts = env::split_paths(&OsString::from(path)).collect::<Vec<_>>();

        assert_eq!(parts[0], router);
        assert!(parts.iter().any(|path| path == &original));
        assert!(envs.iter().any(|(key, value)| {
            key == "ELON_ROUTER_PROJECT_ROOT" && Path::new(value) == root.as_path()
        }));
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn missing_project_router_skips_project_router_env() {
        let root = unique_temp_dir("route-a-no-router");
        fs::create_dir_all(&root).unwrap();

        let envs = cli_child_env_overrides_with_path(
            "copilot",
            "copilot",
            Some(root.to_str().unwrap()),
            None,
        );

        assert!(!envs
            .iter()
            .any(|(key, _)| key == "ELON_ROUTER_PROJECT_ROOT"));
        #[cfg(not(windows))]
        assert!(envs.is_empty());
        let _ = fs::remove_dir_all(root);
    }

    fn unique_temp_dir(label: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        env::temp_dir().join(format!("elon-{label}-{}-{nanos}", std::process::id()))
    }
}

#[cfg(all(test, windows))]
mod windows_tests {
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

pub(crate) fn codex_toolbox_status(codex_program: Option<&str>) -> serde_json::Value {
    tool_catalog::codex_toolbox_status(codex_program.map(Path::new))
}
