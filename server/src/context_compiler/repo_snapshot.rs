use std::{
    fs,
    io::{BufRead, BufReader},
    path::{Path, PathBuf},
    process::Command,
};

use super::repo_walk;

const TOP_LEVEL_LIMIT: usize = 40;
const DOC_LIMIT: usize = 8;
const LARGE_FILE_LIMIT: usize = 8;
const MAX_SOURCE_FILES: usize = 2500;
const SOURCE_EXTENSIONS: &[&str] = &[
    "rs", "kt", "kts", "java", "ts", "tsx", "js", "jsx", "mjs", "cjs", "py", "go", "swift", "cs",
    "c", "cc", "cpp", "h", "hpp",
];
const SKIP_DIRS: &[&str] = &[
    ".git",
    ".gradle",
    ".idea",
    ".next",
    ".nuxt",
    ".venv",
    "build",
    "dist",
    "node_modules",
    "out",
    "target",
    "vendor",
];

#[derive(Debug, Clone, serde::Serialize)]
pub(crate) struct RepoSnapshot {
    pub(crate) git_head: Option<String>,
    pub(crate) git_branch: Option<String>,
    pub(crate) git_dirty: bool,
    pub(crate) git_status_short: Vec<String>,
    pub(crate) has_origin: bool,
    pub(crate) top_level_entries: Vec<String>,
    pub(crate) instruction_docs: Vec<String>,
    pub(crate) manifests: Vec<String>,
    pub(crate) large_files: Vec<SourceFileStat>,
    pub(crate) source_file_count: usize,
}

#[derive(Debug, Clone, serde::Serialize)]
pub(crate) struct SourceFileStat {
    pub(crate) path: String,
    pub(crate) lines: usize,
    pub(crate) role: &'static str,
}

pub(crate) fn collect_repo_snapshot(workspace: &Path) -> RepoSnapshot {
    let mut source_stats = Vec::new();
    collect_source_stats(workspace, workspace, &mut source_stats);
    let source_file_count = source_stats.len();
    source_stats.sort_by(|left, right| right.lines.cmp(&left.lines));
    let large_files = source_stats
        .into_iter()
        .filter(|stat| stat.lines > 500)
        .take(LARGE_FILE_LIMIT)
        .collect();

    RepoSnapshot {
        git_head: git_output(workspace, &["rev-parse", "--short", "HEAD"]),
        git_branch: git_output(workspace, &["rev-parse", "--abbrev-ref", "HEAD"]),
        git_dirty: git_is_dirty(workspace),
        git_status_short: git_status_short(workspace),
        has_origin: git_output(workspace, &["remote", "get-url", "origin"]).is_some(),
        top_level_entries: top_level_entries(workspace),
        instruction_docs: instruction_docs(workspace),
        manifests: manifests(workspace),
        large_files,
        source_file_count,
    }
}

fn git_is_dirty(workspace: &Path) -> bool {
    git_output(workspace, &["status", "--porcelain"])
        .map(|value| !value.trim().is_empty())
        .unwrap_or(false)
}

fn git_status_short(workspace: &Path) -> Vec<String> {
    git_output(workspace, &["status", "--short"])
        .map(|value| {
            value
                .lines()
                .map(|line| line.to_string())
                .take(20)
                .collect()
        })
        .unwrap_or_default()
}

fn git_output(workspace: &Path, args: &[&str]) -> Option<String> {
    let output = Command::new("git")
        .args(args)
        .current_dir(workspace)
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let text = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if text.is_empty() {
        None
    } else {
        Some(text)
    }
}

fn top_level_entries(workspace: &Path) -> Vec<String> {
    let Ok(entries) = fs::read_dir(workspace) else {
        return Vec::new();
    };
    let mut names = entries
        .flatten()
        .filter_map(|entry| {
            let name = entry.file_name().to_string_lossy().to_string();
            if name == ".git" {
                return None;
            }
            let suffix = entry
                .file_type()
                .ok()
                .and_then(|kind| kind.is_dir().then_some("/"))
                .unwrap_or("");
            Some(format!("{name}{suffix}"))
        })
        .collect::<Vec<_>>();
    names.sort();
    names.truncate(TOP_LEVEL_LIMIT);
    names
}

fn instruction_docs(workspace: &Path) -> Vec<String> {
    let candidates = [
        "AGENTS.md",
        "CODEX.md",
        "CLAUDE.md",
        "README.md",
        ".github/copilot-instructions.md",
        ".github/instructions/git-deploy-workflow.instructions.md",
        ".github/instructions/modular-architecture.instructions.md",
        "docs/ai-agent-workflow.md",
    ];
    candidates
        .iter()
        .filter(|path| workspace.join(path).is_file())
        .take(DOC_LIMIT)
        .map(|path| path.to_string())
        .collect()
}

fn manifests(workspace: &Path) -> Vec<String> {
    let candidates = [
        "Cargo.toml",
        "Cargo.lock",
        "package.json",
        "pnpm-workspace.yaml",
        "yarn.lock",
        "settings.gradle",
        "settings.gradle.kts",
        "build.gradle",
        "build.gradle.kts",
        "app/build.gradle",
        "android/app/build.gradle",
        "pyproject.toml",
        "go.mod",
    ];
    candidates
        .iter()
        .filter(|path| workspace.join(path).is_file())
        .map(|path| path.to_string())
        .collect()
}

fn collect_source_stats(base: &Path, _dir: &Path, stats: &mut Vec<SourceFileStat>) {
    for path in repo_walk::collect_matching_files(base, MAX_SOURCE_FILES, is_source_file) {
        if stats.len() >= MAX_SOURCE_FILES {
            return;
        }
        let Some(lines) = count_lines(&path) else {
            continue;
        };
        let relative_path = relative_path(base, &path);
        let role = source_role(&relative_path);
        stats.push(SourceFileStat {
            path: relative_path,
            lines,
            role,
        });
    }
}

pub(crate) fn should_skip_dir(path: &Path) -> bool {
    path.file_name()
        .and_then(|name| name.to_str())
        .map(|name| {
            let lower = name.to_ascii_lowercase();
            SKIP_DIRS.iter().any(|skip| lower == *skip)
        })
        .unwrap_or(false)
}

pub(crate) fn is_source_file(path: &Path) -> bool {
    if path
        .file_name()
        .and_then(|name| name.to_str())
        .map(|name| name.ends_with(".min.js") || name.ends_with(".generated.ts"))
        .unwrap_or(false)
    {
        return false;
    }

    path.extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| {
            let lower = ext.to_ascii_lowercase();
            SOURCE_EXTENSIONS
                .iter()
                .any(|source_ext| lower == *source_ext)
        })
        .unwrap_or(false)
}

pub(crate) fn count_lines(path: &Path) -> Option<usize> {
    let file = fs::File::open(path).ok()?;
    let reader = BufReader::new(file);
    let mut lines = 0usize;
    for line in reader.lines() {
        if line.is_err() {
            return None;
        }
        lines += 1;
    }
    Some(lines)
}

pub(crate) fn relative_path(base: &Path, path: &Path) -> String {
    path.strip_prefix(base)
        .unwrap_or(path)
        .to_string_lossy()
        .replace('\\', "/")
}

pub(crate) fn source_role(relative_path: &str) -> &'static str {
    let name = PathBuf::from(relative_path)
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or(relative_path)
        .to_string();
    if matches!(
        name.as_str(),
        "main.rs" | "router.rs" | "App.tsx" | "App.jsx" | "MainActivity.kt"
    ) {
        return "entrypoint";
    }
    if name.ends_with("Test.kt")
        || name.ends_with("_test.rs")
        || name.ends_with(".test.ts")
        || name.ends_with(".spec.ts")
        || relative_path.contains("/test/")
        || relative_path.contains("/tests/")
    {
        return "test";
    }
    if name.contains("schema") || name.contains("types") || name.ends_with(".d.ts") {
        return "schema";
    }
    "source"
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn snapshot_reports_docs_and_large_files() {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = std::env::temp_dir().join(format!(
            "elon_context_snapshot_{}_{}",
            std::process::id(),
            nonce
        ));
        fs::create_dir_all(dir.join("src")).unwrap();
        fs::write(dir.join("AGENTS.md"), "rules").unwrap();
        fs::write(dir.join("Cargo.toml"), "[package]\nname='demo'\n").unwrap();
        fs::write(dir.join("src/main.rs"), "fn main() {}\n".repeat(802)).unwrap();

        let snapshot = collect_repo_snapshot(&dir);

        assert!(snapshot.instruction_docs.contains(&"AGENTS.md".to_string()));
        assert!(snapshot.manifests.contains(&"Cargo.toml".to_string()));
        assert_eq!(snapshot.large_files[0].path, "src/main.rs");
        assert_eq!(snapshot.large_files[0].lines, 802);

        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn snapshot_respects_gitignore_when_scanning_sources() {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = std::env::temp_dir().join(format!(
            "elon_context_snapshot_ignore_{}_{}",
            std::process::id(),
            nonce
        ));
        fs::create_dir_all(dir.join("src")).unwrap();
        fs::create_dir_all(dir.join("ignored")).unwrap();
        fs::write(dir.join(".gitignore"), "ignored/\n").unwrap();
        fs::write(dir.join("src/main.rs"), "fn main() {}\n").unwrap();
        fs::write(
            dir.join("ignored/lib.rs"),
            "pub fn ignored() {}\n".repeat(900),
        )
        .unwrap();

        let snapshot = collect_repo_snapshot(&dir);

        assert_eq!(snapshot.source_file_count, 1);
        assert!(snapshot.large_files.is_empty());

        fs::remove_dir_all(dir).unwrap();
    }
}
