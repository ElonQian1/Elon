use std::{
    env,
    ffi::OsString,
    path::{Path, PathBuf},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ToolTier {
    Core,
    Profile,
    Optional,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum InstallPolicy {
    AutoSmall,
    ManualRepair,
    NeverAuto,
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
    let tools = resolve_codex_tools(codex_program);
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

fn resolve_codex_tools(codex_program: &Path) -> Vec<ResolvedTool> {
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
        },
        ToolSpec {
            id: "fd",
            primary_bin: "fd",
            aliases: &["fdfind"],
            env_path_var: "ELON_CODEX_FD_PATH",
            managed_dir: "fd",
            tier: ToolTier::Profile,
            install_policy: InstallPolicy::ManualRepair,
        },
        ToolSpec {
            id: "jq",
            primary_bin: "jq",
            aliases: &[],
            env_path_var: "ELON_CODEX_JQ_PATH",
            managed_dir: "jq",
            tier: ToolTier::Profile,
            install_policy: InstallPolicy::ManualRepair,
        },
        ToolSpec {
            id: "7zip",
            primary_bin: "7z",
            aliases: &["7za"],
            env_path_var: "ELON_CODEX_7Z_PATH",
            managed_dir: "7zip",
            tier: ToolTier::Optional,
            install_policy: InstallPolicy::NeverAuto,
        },
    ]
}

fn candidate_tool_paths(spec: &'static ToolSpec, codex_program: &Path) -> Vec<PathBuf> {
    let mut paths = Vec::new();

    if can_borrow_from_codex_runtime(spec) {
        if let Some(parent) = codex_program.parent() {
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
mod tests {
    use super::*;
    use std::{
        fs,
        time::{SystemTime, UNIX_EPOCH},
    };

    #[test]
    fn catalog_marks_only_core_small_tool_for_auto_install() {
        let auto_tools = codex_tool_catalog()
            .iter()
            .filter(|spec| spec.install_policy == InstallPolicy::AutoSmall)
            .map(|spec| spec.id)
            .collect::<Vec<_>>();

        assert_eq!(auto_tools, vec!["rg"]);
    }

    #[test]
    fn core_tool_prefers_codex_program_sibling() {
        let root = unique_temp_dir("codex-tool-sibling");
        fs::create_dir_all(&root).unwrap();
        let codex = root.join(executable_name("codex"));
        let rg = root.join(executable_name("rg"));
        fs::write(&codex, b"").unwrap();
        fs::write(&rg, b"").unwrap();

        let resolved = resolve_codex_tools(&codex);
        let first = resolved.first().expect("rg should resolve");

        assert_eq!(first.spec.id, "rg");
        assert_eq!(first.path, rg);
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn optional_missing_tools_do_not_add_env_vars() {
        let root = unique_temp_dir("codex-no-tools");
        fs::create_dir_all(&root).unwrap();
        let codex = root.join(executable_name("codex"));
        fs::write(&codex, b"").unwrap();

        let envs = codex_child_env_overrides(&codex, Some(OsString::new()));

        assert!(!envs.iter().any(|(key, _)| key == "ELON_CODEX_JQ_PATH"));
        assert!(!envs.iter().any(|(key, _)| key == "ELON_CODEX_7Z_PATH"));
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn path_merge_deduplicates_existing_entries() {
        let root = unique_temp_dir("codex-tool-path");
        let first = root.join("first");
        let second = root.join("second");
        fs::create_dir_all(&first).unwrap();
        fs::create_dir_all(&second).unwrap();
        let current = env::join_paths([second.clone()]).unwrap();

        let merged = prepend_dirs_to_path(vec![first.clone(), second.clone()], Some(current))
            .expect("merged PATH");
        let parts = env::split_paths(&OsString::from(merged)).collect::<Vec<_>>();

        assert_eq!(parts[0], first);
        assert_eq!(
            parts
                .iter()
                .filter(|path| path_key(path) == path_key(&second))
                .count(),
            1
        );
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
