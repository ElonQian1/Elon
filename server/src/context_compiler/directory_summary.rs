use std::{
    collections::{BTreeMap, BTreeSet},
    path::Path,
};

use super::{
    repo_snapshot::{count_lines, is_source_file, relative_path, source_role},
    repo_walk,
};

const MAX_DIRECTORY_FILES: usize = 20_000;
const MAX_DIRECTORY_SUMMARIES: usize = 80;
const MAX_KEY_FILES: usize = 12;

#[derive(Debug, Clone, Default, serde::Serialize)]
pub(crate) struct DirectorySummary {
    pub(crate) path: String,
    pub(crate) direct_files: usize,
    pub(crate) subtree_source_files: usize,
    pub(crate) subtree_lines: usize,
    pub(crate) role_counts: Vec<DirectoryRoleCount>,
    pub(crate) key_files: Vec<String>,
    pub(crate) child_directories: Vec<String>,
}

#[derive(Debug, Clone, serde::Serialize)]
pub(crate) struct DirectoryRoleCount {
    pub(crate) role: String,
    pub(crate) files: usize,
}

#[derive(Default)]
struct DirectoryAccumulator {
    direct_files: usize,
    subtree_source_files: usize,
    subtree_lines: usize,
    roles: BTreeMap<String, usize>,
    key_files: BTreeSet<String>,
    child_directories: BTreeSet<String>,
}

pub(crate) fn collect_directory_summaries(workspace: &Path) -> Vec<DirectorySummary> {
    let files = repo_walk::collect_matching_files(workspace, MAX_DIRECTORY_FILES, is_summary_file);
    let mut dirs = BTreeMap::<String, DirectoryAccumulator>::new();
    dirs.entry(".".to_string()).or_default();

    for path in files {
        let relative = relative_path(workspace, &path);
        let dir = parent_dir(&relative);
        dirs.entry(dir.clone()).or_default().direct_files += 1;
        register_child_dirs(&mut dirs, &dir);
        if is_key_file(&relative) {
            push_key_file(&mut dirs, &dir, &relative);
        }
        if !is_source_file(&path) {
            continue;
        }
        let lines = count_lines(&path).unwrap_or(0);
        let role = source_role(&relative).to_string();
        for ancestor in ancestor_dirs(&dir) {
            let entry = dirs.entry(ancestor).or_default();
            entry.subtree_source_files += 1;
            entry.subtree_lines += lines;
            *entry.roles.entry(role.clone()).or_insert(0) += 1;
        }
    }

    let mut summaries = dirs
        .into_iter()
        .map(|(path, acc)| DirectorySummary {
            path,
            direct_files: acc.direct_files,
            subtree_source_files: acc.subtree_source_files,
            subtree_lines: acc.subtree_lines,
            role_counts: role_counts(acc.roles),
            key_files: acc.key_files.into_iter().take(MAX_KEY_FILES).collect(),
            child_directories: acc.child_directories.into_iter().collect(),
        })
        .collect::<Vec<_>>();
    summaries.sort_by(|left, right| {
        (left.path != ".")
            .cmp(&(right.path != "."))
            .then_with(|| right.subtree_source_files.cmp(&left.subtree_source_files))
            .then_with(|| right.subtree_lines.cmp(&left.subtree_lines))
            .then_with(|| left.path.cmp(&right.path))
    });
    summaries.truncate(MAX_DIRECTORY_SUMMARIES);
    summaries
}

fn is_summary_file(path: &Path) -> bool {
    is_source_file(path)
        || path
            .file_name()
            .and_then(|name| name.to_str())
            .map(is_manifest_name)
            .unwrap_or(false)
}

fn is_key_file(relative: &str) -> bool {
    let name = relative.rsplit('/').next().unwrap_or(relative);
    is_manifest_name(name)
        || matches!(
            source_role(relative),
            "entrypoint" | "test" | "config" | "validation"
        )
}

fn is_manifest_name(name: &str) -> bool {
    matches!(
        name,
        "README.md" | "README" | "Cargo.toml" | "package.json" | "pyproject.toml"
    )
}

fn parent_dir(relative: &str) -> String {
    relative
        .rsplit_once('/')
        .map(|(parent, _)| parent.to_string())
        .unwrap_or_else(|| ".".to_string())
}

fn ancestor_dirs(dir: &str) -> Vec<String> {
    if dir == "." {
        return vec![".".to_string()];
    }
    let mut out = vec![".".to_string()];
    let mut current = String::new();
    for part in dir.split('/') {
        if !current.is_empty() {
            current.push('/');
        }
        current.push_str(part);
        out.push(current.clone());
    }
    out
}

fn register_child_dirs(dirs: &mut BTreeMap<String, DirectoryAccumulator>, dir: &str) {
    if dir == "." {
        return;
    }
    let mut parent = ".".to_string();
    let mut current = String::new();
    for part in dir.split('/') {
        if !current.is_empty() {
            current.push('/');
        }
        current.push_str(part);
        dirs.entry(parent.clone())
            .or_default()
            .child_directories
            .insert(current.clone());
        dirs.entry(current.clone()).or_default();
        parent = current.clone();
    }
}

fn push_key_file(dirs: &mut BTreeMap<String, DirectoryAccumulator>, dir: &str, relative: &str) {
    let entry = dirs.entry(dir.to_string()).or_default();
    if entry.key_files.len() < MAX_KEY_FILES {
        entry.key_files.insert(relative.to_string());
    }
}

fn role_counts(roles: BTreeMap<String, usize>) -> Vec<DirectoryRoleCount> {
    let mut counts = roles
        .into_iter()
        .map(|(role, files)| DirectoryRoleCount { role, files })
        .collect::<Vec<_>>();
    counts.sort_by(|left, right| {
        right
            .files
            .cmp(&left.files)
            .then_with(|| left.role.cmp(&right.role))
    });
    counts
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        fs,
        time::{SystemTime, UNIX_EPOCH},
    };

    #[test]
    fn summarizes_directories_and_respects_gitignore() {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = std::env::temp_dir().join(format!(
            "elon_context_directory_summary_{}_{}",
            std::process::id(),
            nonce
        ));
        fs::create_dir_all(dir.join("server/src")).unwrap();
        fs::create_dir_all(dir.join("ignored")).unwrap();
        fs::write(dir.join(".gitignore"), "ignored/\n").unwrap();
        fs::write(dir.join("README.md"), "# Demo\n").unwrap();
        fs::write(dir.join("server/src/main.rs"), "fn main() {}\n").unwrap();
        fs::write(dir.join("ignored/lib.rs"), "pub fn ignored() {}\n").unwrap();

        let summaries = collect_directory_summaries(&dir);
        let root = summaries.iter().find(|item| item.path == ".").unwrap();
        let server = summaries
            .iter()
            .find(|item| item.path == "server/src")
            .unwrap();

        assert_eq!(root.subtree_source_files, 1);
        assert!(root.key_files.contains(&"README.md".to_string()));
        assert_eq!(server.key_files, vec!["server/src/main.rs".to_string()]);

        fs::remove_dir_all(dir).unwrap();
    }
}
