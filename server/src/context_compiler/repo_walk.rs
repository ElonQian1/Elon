use std::path::{Path, PathBuf};

use ignore::WalkBuilder;

use super::repo_snapshot::{relative_path, should_skip_dir};

const MAX_WALK_ENTRIES: usize = 50_000;

pub(crate) fn collect_matching_files<F>(
    workspace: &Path,
    max_matches: usize,
    mut is_match: F,
) -> Vec<PathBuf>
where
    F: FnMut(&Path) -> bool,
{
    let mut builder = WalkBuilder::new(workspace);
    builder
        .hidden(false)
        .parents(true)
        .ignore(true)
        .git_ignore(true)
        .git_global(true)
        .git_exclude(true)
        .require_git(false)
        .filter_entry(|entry| !should_skip_dir(entry.path()));

    let mut files = Vec::new();
    let mut entries_seen = 0usize;
    for item in builder.build() {
        if files.len() >= max_matches || entries_seen >= MAX_WALK_ENTRIES {
            break;
        }
        entries_seen += 1;
        let Ok(entry) = item else {
            continue;
        };
        if !entry
            .file_type()
            .map(|file_type| file_type.is_file())
            .unwrap_or(false)
        {
            continue;
        }
        let path = entry.into_path();
        if is_match(&path) {
            files.push(path);
        }
    }

    files.sort_by(|left, right| {
        relative_path(workspace, left).cmp(&relative_path(workspace, right))
    });
    files
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        fs,
        time::{SystemTime, UNIX_EPOCH},
    };

    #[test]
    fn respects_gitignore_patterns() {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = std::env::temp_dir().join(format!(
            "elon_context_repo_walk_{}_{}",
            std::process::id(),
            nonce
        ));
        fs::create_dir_all(dir.join("src")).unwrap();
        fs::create_dir_all(dir.join("ignored")).unwrap();
        fs::write(dir.join(".gitignore"), "ignored/\n*.log\n").unwrap();
        fs::write(dir.join("src/lib.rs"), "pub fn kept() {}\n").unwrap();
        fs::write(dir.join("ignored/lib.rs"), "pub fn ignored() {}\n").unwrap();
        fs::write(dir.join("debug.log"), "ignored\n").unwrap();

        let files = collect_matching_files(&dir, 20, |_| true)
            .into_iter()
            .map(|path| relative_path(&dir, &path))
            .collect::<Vec<_>>();

        assert!(files.contains(&"src/lib.rs".to_string()));
        assert!(!files.contains(&"ignored/lib.rs".to_string()));
        assert!(!files.contains(&"debug.log".to_string()));

        fs::remove_dir_all(dir).unwrap();
    }
}
