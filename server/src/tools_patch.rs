//! Workspace-scoped patch application tool for API agents.

use anyhow::{anyhow, Result};
use std::{
    path::{Path, PathBuf},
    process::Command,
    time::{SystemTime, UNIX_EPOCH},
};

pub fn apply_patch(project_root: &Path, patch: &str, check_only: bool) -> Result<String> {
    let normalized_patch = normalize_patch(patch)?;
    let touched_files = touched_files(&normalized_patch)?;
    validate_touched_files(project_root, &touched_files)?;

    let patch_file = temp_patch_file("agent-apply");
    std::fs::write(&patch_file, &normalized_patch)?;
    let cleanup = PatchFileCleanup(patch_file.clone());

    run_git_apply(project_root, &patch_file, true)?;
    if check_only {
        return Ok(format!(
            "补丁检查通过，未应用。涉及文件: {}",
            touched_files.join(", ")
        ));
    }

    run_git_apply(project_root, &patch_file, false)?;
    drop(cleanup);
    Ok(format!(
        "补丁已应用。涉及文件: {}",
        touched_files.join(", ")
    ))
}

fn normalize_patch(patch: &str) -> Result<String> {
    let stripped = strip_fenced_diff(patch);
    let trimmed = stripped.trim();
    if trimmed.is_empty() {
        return Err(anyhow!("补丁不能为空"));
    }
    if !trimmed.contains("@@") || !(trimmed.contains("diff --git ") || trimmed.contains("--- ")) {
        return Err(anyhow!("补丁必须是 unified diff 格式"));
    }
    if trimmed
        .lines()
        .any(|line| line.starts_with("Binary files "))
    {
        return Err(anyhow!("不支持二进制补丁"));
    }
    Ok(format!("{}\n", trimmed.replace("\r\n", "\n")))
}

fn strip_fenced_diff(value: &str) -> String {
    let trimmed = value.trim();
    if !trimmed.starts_with("```") {
        return trimmed.to_string();
    }
    let mut lines = trimmed.lines();
    let _fence = lines.next();
    let mut body = Vec::new();
    for line in lines {
        if line.trim() == "```" {
            break;
        }
        body.push(line);
    }
    if body.is_empty() {
        trimmed.to_string()
    } else {
        body.join("\n")
    }
}

fn touched_files(patch: &str) -> Result<Vec<String>> {
    let mut files = Vec::new();
    for line in patch.lines() {
        if let Some((old_path, new_path)) = parse_diff_git_line(line) {
            push_patch_path(&mut files, old_path);
            push_patch_path(&mut files, new_path);
            continue;
        }
        if let Some(path) = line.strip_prefix("--- ").and_then(parse_marker_path) {
            push_patch_path(&mut files, Some(path));
            continue;
        }
        if let Some(path) = line.strip_prefix("+++ ").and_then(parse_marker_path) {
            push_patch_path(&mut files, Some(path));
        }
    }
    files.sort();
    files.dedup();
    if files.is_empty() {
        return Err(anyhow!("无法从补丁中识别目标文件"));
    }
    Ok(files)
}

fn parse_diff_git_line(line: &str) -> Option<(Option<String>, Option<String>)> {
    let rest = line.strip_prefix("diff --git ")?;
    let mut parts = rest.split_whitespace();
    let old_path = parts.next().and_then(normalize_patch_path);
    let new_path = parts.next().and_then(normalize_patch_path);
    Some((old_path, new_path))
}

fn parse_marker_path(raw: &str) -> Option<String> {
    let path = raw.split_whitespace().next()?;
    normalize_patch_path(path)
}

fn normalize_patch_path(raw: &str) -> Option<String> {
    let path = raw.trim().trim_matches('"');
    if path == "/dev/null" {
        return None;
    }
    path.strip_prefix("a/")
        .or_else(|| path.strip_prefix("b/"))
        .or(Some(path))
        .map(str::to_string)
}

fn push_patch_path(files: &mut Vec<String>, path: Option<String>) {
    if let Some(path) = path {
        if !path.trim().is_empty() {
            files.push(path);
        }
    }
}

fn validate_touched_files(project_root: &Path, files: &[String]) -> Result<()> {
    let canonical_root = project_root
        .canonicalize()
        .unwrap_or_else(|_| project_root.to_path_buf());
    for file in files {
        if file.contains("..") || Path::new(file).is_absolute() {
            return Err(anyhow!("补丁目标路径不安全: {}", file));
        }
        if file == ".git" || file.starts_with(".git/") {
            return Err(anyhow!("补丁不允许修改 .git 目录"));
        }

        let full = canonical_root.join(file);
        let resolved = resolve_existing_parent(&full)?;
        if !resolved.starts_with(&canonical_root) {
            return Err(anyhow!("补丁目标越界: {}", file));
        }
    }
    Ok(())
}

fn resolve_existing_parent(path: &Path) -> Result<PathBuf> {
    if path.exists() {
        return path
            .canonicalize()
            .map_err(|error| anyhow!("补丁目标路径规范化失败: {}", error));
    }

    let mut current = path;
    while let Some(parent) = current.parent() {
        if parent.exists() {
            return parent
                .canonicalize()
                .map_err(|error| anyhow!("补丁目标父目录规范化失败: {}", error));
        }
        current = parent;
    }
    Err(anyhow!("补丁目标路径无有效父目录"))
}

fn run_git_apply(project_root: &Path, patch_file: &Path, check: bool) -> Result<()> {
    let mut args = vec!["apply", "--whitespace=nowarn"];
    if check {
        args.push("--check");
    }
    let patch_arg = patch_file
        .to_str()
        .ok_or_else(|| anyhow!("补丁临时文件路径不是有效 UTF-8"))?;
    args.push(patch_arg);

    let output = Command::new("git")
        .args(args)
        .current_dir(project_root)
        .output()?;
    if output.status.success() {
        return Ok(());
    }

    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);
    let phase = if check { "检查" } else { "应用" };
    Err(anyhow!(
        "补丁{}失败: {}{}",
        phase,
        stdout.trim(),
        stderr.trim()
    ))
}

fn temp_patch_file(kind: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or_default();
    std::env::temp_dir().join(format!("elon-{kind}-{nanos}.patch"))
}

struct PatchFileCleanup(PathBuf);

impl Drop for PatchFileCleanup {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.0);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn apply_patch_changes_file() {
        let workspace = temp_workspace("apply_patch_changes_file");
        init_git(&workspace);
        std::fs::create_dir_all(workspace.join("src")).unwrap();
        std::fs::write(
            workspace.join("src/lib.rs"),
            "pub fn answer() -> i32 {\n    1\n}\n",
        )
        .unwrap();

        let patch = r#"diff --git a/src/lib.rs b/src/lib.rs
--- a/src/lib.rs
+++ b/src/lib.rs
@@ -1,3 +1,3 @@
 pub fn answer() -> i32 {
-    1
+    2
 }
"#;

        let result = apply_patch(&workspace, patch, false).unwrap();
        assert!(result.contains("src/lib.rs"));
        let content = std::fs::read_to_string(workspace.join("src/lib.rs")).unwrap();
        assert!(content.contains("    2"));
    }

    #[test]
    fn check_only_does_not_change_file() {
        let workspace = temp_workspace("check_only_does_not_change_file");
        init_git(&workspace);
        std::fs::write(workspace.join("README.md"), "old\n").unwrap();

        let patch = r#"diff --git a/README.md b/README.md
--- a/README.md
+++ b/README.md
@@ -1 +1 @@
-old
+new
"#;

        let result = apply_patch(&workspace, patch, true).unwrap();
        assert!(result.contains("补丁检查通过"));
        assert_eq!(
            std::fs::read_to_string(workspace.join("README.md")).unwrap(),
            "old\n"
        );
    }

    #[test]
    fn rejects_path_escape() {
        let workspace = temp_workspace("rejects_path_escape");
        init_git(&workspace);

        let patch = r#"diff --git a/../outside.txt b/../outside.txt
--- a/../outside.txt
+++ b/../outside.txt
@@ -1 +1 @@
-old
+new
"#;

        let error = apply_patch(&workspace, patch, true).unwrap_err();
        assert!(error.to_string().contains("不安全"));
    }

    fn temp_workspace(name: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("elon-tools-patch-{name}-{nanos}"));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn init_git(workspace: &Path) {
        let output = Command::new("git")
            .args(["init"])
            .current_dir(workspace)
            .output()
            .unwrap();
        assert!(output.status.success());
    }
}
