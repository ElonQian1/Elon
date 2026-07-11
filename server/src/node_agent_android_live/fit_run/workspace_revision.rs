use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{bail, Context, Result};
use sha2::{Digest, Sha256};

const MAX_DIFF_BYTES: usize = 64 * 1024 * 1024;
const MAX_UNTRACKED_BYTES: u64 = 64 * 1024 * 1024;
const MAX_TOTAL_UNTRACKED_BYTES: u64 = 128 * 1024 * 1024;

/// Returns a content-sensitive workspace identity. Unlike `git rev-parse HEAD`,
/// this changes for staged, unstaged and untracked files as well.
pub(crate) fn workspace_fingerprint(project_root: &str) -> Result<Option<String>> {
    let root = PathBuf::from(project_root)
        .canonicalize()
        .with_context(|| format!("项目目录不存在: {project_root}"))?;
    let Some(head) = git_output(&root, &["rev-parse", "--verify", "HEAD"])? else {
        return Ok(None);
    };
    let diff = git_output_required(&root, &["diff", "--binary", "--no-ext-diff", "HEAD", "--"])?;
    if diff.len() > MAX_DIFF_BYTES {
        bail!("工作区差异超过 64MiB，拒绝生成不完整 Source Revision");
    }
    let untracked =
        git_output_required(&root, &["ls-files", "--others", "--exclude-standard", "-z"])?;

    let mut hasher = Sha256::new();
    hasher.update(b"elon-workspace-v1\0");
    hasher.update(head);
    hasher.update([0]);
    hasher.update(diff);
    hasher.update([0]);
    hash_untracked_files(&root, &untracked, &mut hasher)?;
    Ok(Some(format!(
        "workspace-sha256:{}",
        hex::encode(hasher.finalize())
    )))
}

fn hash_untracked_files(root: &Path, names: &[u8], hasher: &mut Sha256) -> Result<()> {
    let mut total = 0_u64;
    for raw in names
        .split(|byte| *byte == 0)
        .filter(|value| !value.is_empty())
    {
        let relative = std::str::from_utf8(raw).context("Git 返回了非 UTF-8 的未跟踪文件名")?;
        let path = root.join(relative);
        let canonical = path
            .canonicalize()
            .with_context(|| format!("未跟踪文件不可访问: {relative}"))?;
        if !canonical.starts_with(root) || !canonical.is_file() {
            bail!("未跟踪文件越出项目目录或不是普通文件: {relative}");
        }
        let metadata = fs::metadata(&canonical)?;
        if metadata.len() > MAX_UNTRACKED_BYTES {
            bail!("未跟踪文件超过 64MiB，无法纳入 Source Revision: {relative}");
        }
        total = total.saturating_add(metadata.len());
        if total > MAX_TOTAL_UNTRACKED_BYTES {
            bail!("未跟踪文件总量超过 128MiB，无法生成 Source Revision");
        }
        hasher.update(raw);
        hasher.update([0]);
        hasher.update(fs::read(&canonical)?);
        hasher.update([0xff]);
    }
    Ok(())
}

fn git_output(root: &Path, args: &[&str]) -> Result<Option<Vec<u8>>> {
    let output = Command::new("git")
        .args(args)
        .current_dir(root)
        .output()
        .context("无法执行 git")?;
    Ok(output.status.success().then_some(output.stdout))
}

fn git_output_required(root: &Path, args: &[&str]) -> Result<Vec<u8>> {
    git_output(root, args)?.ok_or_else(|| anyhow::anyhow!("git {} 执行失败", args.join(" ")))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fingerprint_changes_for_dirty_and_untracked_content() {
        let root = std::env::temp_dir().join(format!(
            "elon-workspace-fingerprint-{}",
            uuid::Uuid::new_v4().simple()
        ));
        fs::create_dir_all(&root).unwrap();
        run_git(&root, &["init"]);
        run_git(&root, &["config", "user.email", "test@example.com"]);
        run_git(&root, &["config", "user.name", "Test"]);
        fs::write(root.join("tracked.txt"), "one").unwrap();
        run_git(&root, &["add", "tracked.txt"]);
        run_git(&root, &["commit", "-m", "init"]);
        let clean = workspace_fingerprint(root.to_str().unwrap()).unwrap();
        fs::write(root.join("tracked.txt"), "two").unwrap();
        let dirty = workspace_fingerprint(root.to_str().unwrap()).unwrap();
        assert_ne!(clean, dirty);
        fs::write(root.join("new.txt"), "new").unwrap();
        let untracked = workspace_fingerprint(root.to_str().unwrap()).unwrap();
        assert_ne!(dirty, untracked);
        fs::remove_dir_all(root).unwrap();
    }

    fn run_git(root: &Path, args: &[&str]) {
        assert!(Command::new("git")
            .args(args)
            .current_dir(root)
            .status()
            .unwrap()
            .success());
    }
}
