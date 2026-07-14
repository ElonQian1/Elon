use std::{
    env,
    ffi::OsString,
    path::{Path, PathBuf},
};

use serde_json::Value;

#[path = "node_agent_cli_tool_status.rs"]
mod tool_status;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ToolTier {
    Core,
    Profile,
    Optional,
}

impl ToolTier {
    fn as_str(self) -> &'static str {
        match self {
            Self::Core => "core",
            Self::Profile => "profile",
            Self::Optional => "optional",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum InstallPolicy {
    AutoSmall,
    ManualRepair,
    NeverAuto,
}

impl InstallPolicy {
    fn as_str(self) -> &'static str {
        match self {
            Self::AutoSmall => "AutoSmall",
            Self::ManualRepair => "ManualRepair",
            Self::NeverAuto => "NeverAuto",
        }
    }
}

#[derive(Debug, Clone, Copy)]
struct ToolSpec {
    id: &'static str,
    primary_bin: &'static str,
    aliases: &'static [&'static str],
    env_path_var: &'static str,
    managed_dir: &'static str,
    tier: ToolTier,
    install_policy: InstallPolicy,
    version_args: &'static [&'static str],
}

#[derive(Debug, Clone)]
struct ResolvedTool {
    spec: &'static ToolSpec,
    path: PathBuf,
}

pub(crate) fn codex_child_env_overrides(
    codex_program: &Path,
    current_path: Option<OsString>,
) -> Vec<(String, String)> {
    let tools = resolve_codex_tools(Some(codex_program));
    let dirs = tools
        .iter()
        .filter_map(|tool| tool.path.parent().map(Path::to_path_buf))
        .collect::<Vec<_>>();

    let mut envs = Vec::new();
    if let Some(path) = prepend_dirs_to_path(dirs, current_path) {
        envs.push(("PATH".to_string(), path));
    }
    for tool in tools {
        envs.push((
            tool.spec.env_path_var.to_string(),
            tool.path.to_string_lossy().to_string(),
        ));
    }
    envs
}

pub(crate) fn codex_toolbox_status(codex_program: Option<&Path>) -> Value {
    tool_status::codex_toolbox_status(codex_program)
}

fn resolve_codex_tools(codex_program: Option<&Path>) -> Vec<ResolvedTool> {
    codex_tool_catalog()
        .iter()
        .filter_map(|spec| {
            candidate_tool_paths(spec, codex_program)
                .into_iter()
                .next()
                .map(|path| ResolvedTool { spec, path })
        })
        .collect()
}

fn codex_tool_catalog() -> &'static [ToolSpec] {
    &[
        ToolSpec {
            id: "rg",
            primary_bin: "rg",
            aliases: &[],
            env_path_var: "ELON_CODEX_RG_PATH",
            managed_dir: "ripgrep",
            tier: ToolTier::Core,
            install_policy: InstallPolicy::AutoSmall,
            version_args: &["--version"],
        },
        ToolSpec {
            id: "fd",
            primary_bin: "fd",
            aliases: &["fdfind"],
            env_path_var: "ELON_CODEX_FD_PATH",
            managed_dir: "fd",
            tier: ToolTier::Profile,
            install_policy: InstallPolicy::ManualRepair,
            version_args: &["--version"],
        },
        ToolSpec {
            id: "jq",
            primary_bin: "jq",
            aliases: &[],
            env_path_var: "ELON_CODEX_JQ_PATH",
            managed_dir: "jq",
            tier: ToolTier::Profile,
            install_policy: InstallPolicy::ManualRepair,
            version_args: &["--version"],
        },
        ToolSpec {
            id: "7zip",
            primary_bin: "7z",
            aliases: &["7za"],
            env_path_var: "ELON_CODEX_7Z_PATH",
            managed_dir: "7zip",
            tier: ToolTier::Optional,
            install_policy: InstallPolicy::NeverAuto,
            version_args: &[],
        },
    ]
}

fn candidate_tool_paths(spec: &'static ToolSpec, codex_program: Option<&Path>) -> Vec<PathBuf> {
    let mut paths = Vec::new();

    if can_borrow_from_codex_runtime(spec) {
        if let Some(parent) = codex_program.and_then(Path::parent) {
            push_existing_tool(&mut paths, parent, spec);
        }
    }

    push_command_candidates(&mut paths, spec.primary_bin);
    for alias in spec.aliases {
        push_command_candidates(&mut paths, alias);
    }

    #[cfg(windows)]
    push_windows_common_tool_paths(&mut paths, spec);

    paths
}

fn can_borrow_from_codex_runtime(spec: &ToolSpec) -> bool {
    spec.tier == ToolTier::Core && spec.install_policy == InstallPolicy::AutoSmall
}

fn push_command_candidates(paths: &mut Vec<PathBuf>, name: &str) {
    for path in elon_pc_dev_runtime::command_candidates(name) {
        push_existing_path(paths, path);
    }
}

fn push_existing_tool(paths: &mut Vec<PathBuf>, dir: &Path, spec: &ToolSpec) {
    push_existing_path(paths, dir.join(executable_name(spec.primary_bin)));
    for alias in spec.aliases {
        push_existing_path(paths, dir.join(executable_name(alias)));
    }
}

fn push_existing_path(paths: &mut Vec<PathBuf>, path: PathBuf) {
    if path.is_file() && !contains_path(paths, &path) {
        paths.push(path);
    }
}

pub(super) fn prepend_dirs_to_path(
    dirs: Vec<PathBuf>,
    current_path: Option<OsString>,
) -> Option<String> {
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

fn path_to_string(path: &Path) -> String {
    path.to_string_lossy().to_string()
}

#[cfg(windows)]
fn push_windows_common_tool_paths(paths: &mut Vec<PathBuf>, spec: &ToolSpec) {
    if let Ok(localappdata) = env::var("LOCALAPPDATA") {
        let local = PathBuf::from(localappdata);
        let managed_root = local.join("ElonNode").join("tools").join(spec.managed_dir);
        push_existing_tool(paths, &managed_root.join("bin"), spec);
        push_tool_from_child_dirs(paths, &managed_root, spec);

        if can_borrow_from_codex_runtime(spec) {
            push_tool_from_child_dirs(paths, &local.join("OpenAI").join("Codex").join("bin"), spec);
        }
    }

    if let Ok(userprofile) = env::var("USERPROFILE") {
        let user = PathBuf::from(userprofile);
        push_existing_tool(paths, &user.join(".cargo").join("bin"), spec);
        push_existing_tool(paths, &user.join("scoop").join("shims"), spec);
    }

    if let Ok(program_files) = env::var("ProgramFiles") {
        let root = PathBuf::from(program_files);
        push_existing_tool(paths, &root.join(spec.managed_dir), spec);
        push_existing_tool(paths, &root.join(display_dir_name(spec.managed_dir)), spec);
        push_existing_tool(paths, &root.join("7-Zip"), spec);
        if can_borrow_from_codex_runtime(spec) && spec.id == "rg" {
            push_rg_from_codex_windows_apps(paths, &root.join("WindowsApps"));
        }
    }

    if let Ok(programdata) = env::var("ProgramData") {
        push_existing_tool(
            paths,
            &PathBuf::from(programdata).join("chocolatey").join("bin"),
            spec,
        );
    }
}

#[cfg(windows)]
fn push_tool_from_child_dirs(paths: &mut Vec<PathBuf>, root: &Path, spec: &ToolSpec) {
    if let Ok(entries) = std::fs::read_dir(root) {
        for entry in entries.flatten() {
            let dir = entry.path();
            if !dir.is_dir() {
                continue;
            }
            push_existing_tool(paths, &dir, spec);
            push_existing_tool(paths, &dir.join("bin"), spec);
        }
    }
}

#[cfg(windows)]
fn push_rg_from_codex_windows_apps(paths: &mut Vec<PathBuf>, windows_apps: &Path) {
    let Some(rg) = codex_tool_catalog().iter().find(|spec| spec.id == "rg") else {
        return;
    };
    if let Ok(entries) = std::fs::read_dir(windows_apps) {
        for entry in entries.flatten() {
            let dir = entry.path();
            let Some(name) = dir.file_name().and_then(|name| name.to_str()) else {
                continue;
            };
            if name.to_ascii_lowercase().starts_with("openai.codex_") {
                push_existing_tool(paths, &dir.join("app").join("resources"), rg);
            }
        }
    }
}

fn executable_name(name: &str) -> String {
    if cfg!(windows) && !name.ends_with(".exe") {
        format!("{name}.exe")
    } else {
        name.to_string()
    }
}

#[cfg(windows)]
fn display_dir_name(name: &str) -> String {
    let mut chars = name.chars();
    match chars.next() {
        Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
        None => String::new(),
    }
}

fn contains_path(paths: &[PathBuf], candidate: &Path) -> bool {
    let candidate_key = path_key(candidate);
    paths.iter().any(|path| path_key(path) == candidate_key)
}

fn path_key(path: &Path) -> String {
    let value = path
        .to_string_lossy()
        .trim_end_matches(&['\\', '/'][..])
        .to_string();
    if cfg!(windows) {
        value.to_ascii_lowercase()
    } else {
        value
    }
}

#[cfg(test)]
#[path = "node_agent_cli_tool_catalog_tests.rs"]
mod node_agent_cli_tool_catalog_tests;
