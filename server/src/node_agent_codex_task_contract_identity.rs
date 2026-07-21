//! Resolve ordinary Codex Desktop WF worktrees from their immutable preflight
//! finish contract. Supervised task lineage remains the primary identity.

use std::{
    fs,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result};
use serde::Deserialize;
use sha2::{Digest, Sha256};

const SCHEMA: &str = "elon.ai_finish_contract.v1";
const LOCK_REASON: &str = "active Codex task; finish-ai-task unlocks";
const MAX_CONTRACT_BYTES: u64 = 64 * 1024;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct FinishContract {
    schema: String,
    worktree: String,
    branch: String,
    base_commit: String,
    origin: String,
    issued_at_utc: String,
}

pub(crate) fn resolve(project_root: &Path) -> Result<Option<String>> {
    resolve_with(project_root, &contract_root())
}

fn contract_root() -> PathBuf {
    std::env::var_os("LOCALAPPDATA")
        .map(PathBuf::from)
        .unwrap_or_else(std::env::temp_dir)
        .join("ElonNode")
        .join("ai-finish-contracts-v1")
}

fn resolve_with(project_root: &Path, contracts: &Path) -> Result<Option<String>> {
    if !has_managed_lock(project_root)? {
        return Ok(None);
    }
    let canonical = project_root.canonicalize()?;
    let branch = git(project_root, &["branch", "--show-current"])?;
    let origin = git(project_root, &["remote", "get-url", "origin"])?;
    let mut matches = Vec::new();
    for entry in read_contract_entries(contracts)? {
        let path = entry.path();
        let Some(id) = path.file_stem().and_then(|value| value.to_str()) else {
            continue;
        };
        if path.extension().and_then(|value| value.to_str()) != Some("json")
            || id.len() != 64
            || !id.bytes().all(|value| value.is_ascii_hexdigit())
            || entry.metadata()?.len() > MAX_CONTRACT_BYTES
        {
            continue;
        }
        let bytes = fs::read(&path)?;
        if format!("{:x}", Sha256::digest(&bytes)) != id.to_ascii_lowercase() {
            continue;
        }
        let Ok(contract) = serde_json::from_slice::<FinishContract>(&bytes) else {
            continue;
        };
        if contract.schema != SCHEMA
            || !same_path(Path::new(&contract.worktree), &canonical)
            || contract.branch != branch
            || contract.origin != origin
            || !is_ancestor(project_root, &contract.base_commit)?
        {
            continue;
        }
        matches.push((contract.issued_at_utc, id.to_ascii_lowercase()));
    }
    matches.sort();
    if let Some((_, id)) = matches.into_iter().next() {
        return Ok(Some(format!("codex-task-contract:{id}")));
    }
    legacy_identity(project_root, &canonical, &branch, &origin)
}

fn read_contract_entries(contracts: &Path) -> Result<Vec<fs::DirEntry>> {
    if !contracts.is_dir() {
        return Ok(Vec::new());
    }
    fs::read_dir(contracts)
        .context("读取 Codex TaskContract 目录失败")?
        .collect::<std::io::Result<Vec<_>>>()
        .map_err(Into::into)
}

fn legacy_identity(
    project_root: &Path,
    canonical: &Path,
    branch: &str,
    origin: &str,
) -> Result<Option<String>> {
    if !branch.starts_with("codex/task-") || origin.trim().is_empty() {
        return Ok(None);
    }
    let git_dir =
        PathBuf::from(git(project_root, &["rev-parse", "--absolute-git-dir"])?).canonicalize()?;
    let common_dir = PathBuf::from(git(
        project_root,
        &["rev-parse", "--path-format=absolute", "--git-common-dir"],
    )?)
    .canonicalize()?;
    if same_path(&git_dir, &common_dir) {
        return Ok(None);
    }
    let material = format!(
        "{}\n{}\n{}\n{}",
        canonical.to_string_lossy().to_ascii_lowercase(),
        common_dir.to_string_lossy().to_ascii_lowercase(),
        branch,
        origin
    );
    Ok(Some(format!(
        "codex-managed-worktree:{:x}",
        Sha256::digest(material.as_bytes())
    )))
}

fn has_managed_lock(project_root: &Path) -> Result<bool> {
    let Ok(git_dir) = git(project_root, &["rev-parse", "--absolute-git-dir"]) else {
        return Ok(false);
    };
    let git_dir = PathBuf::from(git_dir);
    Ok(fs::read_to_string(git_dir.join("locked"))
        .map(|value| value.trim() == LOCK_REASON)
        .unwrap_or(false))
}

fn is_ancestor(project_root: &Path, base: &str) -> Result<bool> {
    Ok(crate::git_command_error::git_command()
        .current_dir(project_root)
        .args(["merge-base", "--is-ancestor", base, "HEAD"])
        .status()
        .context("验证 Codex TaskContract base commit 失败")?
        .success())
}

fn git(project_root: &Path, args: &[&str]) -> Result<String> {
    let output = crate::git_command_error::git_command()
        .current_dir(project_root)
        .args(args)
        .output()
        .with_context(|| format!("运行 git {} 失败", args.join(" ")))?;
    anyhow::ensure!(output.status.success(), "git {} 返回失败", args.join(" "));
    Ok(String::from_utf8(output.stdout)?.trim().to_string())
}

fn same_path(left: &Path, right: &Path) -> bool {
    let left = left.canonicalize().unwrap_or_else(|_| left.to_path_buf());
    let right = right.canonicalize().unwrap_or_else(|_| right.to_path_buf());
    if cfg!(windows) {
        left.to_string_lossy()
            .eq_ignore_ascii_case(&right.to_string_lossy())
    } else {
        left == right
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn run(root: &Path, args: &[&str]) {
        assert!(crate::git_command_error::git_command()
            .current_dir(root)
            .args(args)
            .status()
            .unwrap()
            .success());
    }

    fn fixture() -> (PathBuf, PathBuf, String) {
        let temp =
            std::env::temp_dir().join(format!("codex-contract-{}", uuid::Uuid::new_v4().simple()));
        let base = temp.join("base");
        let root = temp.join("repo");
        let contracts = temp.join("contracts");
        fs::create_dir_all(&base).unwrap();
        fs::create_dir_all(&contracts).unwrap();
        run(&base, &["init", "-b", "main"]);
        run(&base, &["config", "user.email", "test@example.com"]);
        run(&base, &["config", "user.name", "test"]);
        run(
            &base,
            &["remote", "add", "origin", "https://example.com/repo.git"],
        );
        fs::write(base.join("file.txt"), "test").unwrap();
        run(&base, &["add", "file.txt"]);
        run(&base, &["commit", "-m", "base"]);
        run(
            &base,
            &[
                "worktree",
                "add",
                "-b",
                "codex/task-test",
                root.to_str().unwrap(),
                "HEAD",
            ],
        );
        let head = git(&root, &["rev-parse", "HEAD"]).unwrap();
        let git_dir = PathBuf::from(git(&root, &["rev-parse", "--absolute-git-dir"]).unwrap());
        fs::write(git_dir.join("locked"), LOCK_REASON).unwrap();
        (root, contracts, head)
    }

    fn write_contract(root: &Path, contracts: &Path, base: &str) -> String {
        let payload = serde_json::to_vec(&json!({
            "schema": SCHEMA,
            "worktree": root.to_string_lossy().replace('\\', "/"),
            "branch": "codex/task-test",
            "baseCommit": base,
            "origin": "https://example.com/repo.git",
            "issuedAtUtc": "2026-07-22T00:00:00Z",
            "nonce": "fixture"
        }))
        .unwrap();
        let id = format!("{:x}", Sha256::digest(&payload));
        fs::write(contracts.join(format!("{id}.json")), payload).unwrap();
        id
    }

    #[test]
    fn resolves_managed_codex_worktree_from_immutable_contract() {
        let (root, contracts, head) = fixture();
        let id = write_contract(&root, &contracts, &head);
        assert_eq!(
            resolve_with(&root, &contracts).unwrap(),
            Some(format!("codex-task-contract:{id}"))
        );
        fs::remove_dir_all(root.parent().unwrap()).unwrap();
    }

    #[test]
    fn falls_back_for_legacy_managed_worktree_and_rejects_unmanaged_worktree() {
        let (root, contracts, head) = fixture();
        let id = write_contract(&root, &contracts, &head);
        fs::write(contracts.join(format!("{id}.json")), b"{}").unwrap();
        assert!(resolve_with(&root, &contracts)
            .unwrap()
            .unwrap()
            .starts_with("codex-managed-worktree:"));
        let git_dir = PathBuf::from(git(&root, &["rev-parse", "--absolute-git-dir"]).unwrap());
        fs::remove_file(git_dir.join("locked")).unwrap();
        assert_eq!(resolve_with(&root, &contracts).unwrap(), None);
        fs::remove_dir_all(root.parent().unwrap()).unwrap();
    }
}
