//! Parent-side binding from the compiled SHA to the exact clean checkout under test.

use std::{path::Path, process::Command};

use anyhow::{anyhow, Context};

pub(super) fn verify_exact_clean_checkout(expected_git_sha: &str) -> anyhow::Result<()> {
    let manifest = Path::new(env!("CARGO_MANIFEST_DIR"));
    let root = manifest
        .parent()
        .ok_or_else(|| anyhow!("resolve Unmap repository root from Cargo manifest"))?;
    let head = git_output(root, &["rev-parse", "HEAD"])?;
    if head.trim() != expected_git_sha {
        return Err(anyhow!("A2_UNMAP_CHECKOUT_HEAD_MISMATCH"));
    }

    let status = git_output(root, &["status", "--porcelain=v1", "--untracked-files=all"])?;
    if !status.trim().is_empty() {
        return Err(anyhow!("A2_UNMAP_CHECKOUT_NOT_CLEAN"));
    }
    Ok(())
}

fn git_output(root: &Path, arguments: &[&str]) -> anyhow::Result<String> {
    let output = Command::new("git")
        .arg("-C")
        .arg(root)
        .args(arguments)
        .output()
        .context("run exact-checkout Git observation")?;
    if !output.status.success() {
        return Err(anyhow!("A2_UNMAP_CHECKOUT_GIT_FAILED"));
    }
    String::from_utf8(output.stdout).map_err(|_| anyhow!("A2_UNMAP_CHECKOUT_GIT_NON_UTF8"))
}
