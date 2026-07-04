use std::{
    env,
    ffi::OsString,
    path::{Path, PathBuf},
};

pub(crate) fn codex_child_env_overrides(codex_program: &str) -> Vec<(String, String)> {
    let rg_paths = candidate_rg_paths(Path::new(codex_program));
    let rg_dirs = rg_paths
        .iter()
        .filter_map(|path| path.parent().map(Path::to_path_buf))
        .collect::<Vec<_>>();

    let mut envs = Vec::new();
    if let Some(path) = prepend_dirs_to_path(rg_dirs, env::var_os("PATH")) {
        envs.push(("PATH".to_string(), path));
    }
    if let Some(rg_path) = rg_paths.first() {
        envs.push((
            "ELON_CODEX_RG_PATH".to_string(),
            rg_path.to_string_lossy().to_string(),
        ));
    }
    envs
}

fn candidate_rg_paths(codex_program: &Path) -> Vec<PathBuf> {
    let mut paths = Vec::new();

    if let Some(parent) = codex_program.parent() {
        push_existing_rg(&mut paths, parent);
    }

    for path in elon_pc_dev_runtime::command_candidates("rg") {
        push_existing_path(&mut paths, path);
    }

    #[cfg(windows)]
    {
        push_windows_common_rg_paths(&mut paths);
    }

    paths
}

fn push_existing_rg(paths: &mut Vec<PathBuf>, dir: &Path) {
    push_existing_path(paths, dir.join(rg_executable_name()));
}

fn push_existing_path(paths: &mut Vec<PathBuf>, path: PathBuf) {
    if path.is_file() && !contains_path(paths, &path) {
        paths.push(path);
    }
}

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

fn rg_executable_name() -> &'static str {
    if cfg!(windows) {
        "rg.exe"
    } else {
        "rg"
    }
}

#[cfg(windows)]
fn push_windows_common_rg_paths(paths: &mut Vec<PathBuf>) {
    if let Ok(localappdata) = env::var("LOCALAPPDATA") {
        let local = PathBuf::from(localappdata);
        push_existing_rg(
            paths,
            &local
                .join("ElonNode")
                .join("tools")
                .join("ripgrep")
                .join("bin"),
        );
        push_rg_from_child_dirs(paths, &local.join("ElonNode").join("tools").join("ripgrep"));
        push_rg_from_child_dirs(paths, &local.join("OpenAI").join("Codex").join("bin"));
    }

    if let Ok(userprofile) = env::var("USERPROFILE") {
        let user = PathBuf::from(userprofile);
        push_existing_rg(paths, &user.join(".cargo").join("bin"));
        push_existing_rg(paths, &user.join("scoop").join("shims"));
    }

    if let Ok(program_files) = env::var("ProgramFiles") {
        let root = PathBuf::from(program_files);
        push_existing_rg(paths, &root.join("ripgrep"));
        push_existing_rg(paths, &root.join("Ripgrep"));
        push_rg_from_codex_windows_apps(paths, &root.join("WindowsApps"));
    }

    if let Ok(programdata) = env::var("ProgramData") {
        push_existing_rg(
            paths,
            &PathBuf::from(programdata).join("chocolatey").join("bin"),
        );
    }
}

#[cfg(windows)]
fn push_rg_from_child_dirs(paths: &mut Vec<PathBuf>, root: &Path) {
    if let Ok(entries) = std::fs::read_dir(root) {
        for entry in entries.flatten() {
            let dir = entry.path();
            if !dir.is_dir() {
                continue;
            }
            push_existing_rg(paths, &dir);
            push_existing_rg(paths, &dir.join("bin"));
        }
    }
}

#[cfg(windows)]
fn push_rg_from_codex_windows_apps(paths: &mut Vec<PathBuf>, windows_apps: &Path) {
    if let Ok(entries) = std::fs::read_dir(windows_apps) {
        for entry in entries.flatten() {
            let dir = entry.path();
            let Some(name) = dir.file_name().and_then(|name| name.to_str()) else {
                continue;
            };
            if !name.to_ascii_lowercase().starts_with("openai.codex_") {
                continue;
            }
            push_existing_rg(paths, &dir.join("app").join("resources"));
        }
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
    fn candidate_rg_paths_prefers_codex_program_sibling_rg() {
        let root = unique_temp_dir("codex-rg-sibling");
        fs::create_dir_all(&root).unwrap();
        let codex = root.join(if cfg!(windows) { "codex.exe" } else { "codex" });
        let rg = root.join(rg_executable_name());
        fs::write(&codex, b"").unwrap();
        fs::write(&rg, b"").unwrap();

        let paths = candidate_rg_paths(&codex);

        assert_eq!(paths.first(), Some(&rg));
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn prepend_dirs_to_path_deduplicates_existing_entries() {
        let root = unique_temp_dir("codex-rg-path");
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
