//! Linear proof that the exact compiled JointClose commit is the clean checkout under test.

use std::{path::Path, process::Command};

use super::{super::environment::validate_git_sha, JointCloseFamilyCohort};

#[must_use = "the clean-checkout receipt must be consumed by the JointClose family reducer"]
pub(in super::super::super) struct ValidatedJointCloseCleanCheckoutReceipt {
    pub(super) git_sha: String,
    pub(super) cohort_commitment: [u8; 32],
}

impl ValidatedJointCloseCleanCheckoutReceipt {
    /// Observes HEAD and complete porcelain status; callers cannot assert cleanliness themselves.
    pub(in super::super::super) fn capture(
        cohort: &JointCloseFamilyCohort,
    ) -> Result<Self, &'static str> {
        if !cohort.is_valid() {
            return Err("A2_JOINT_CLOSE_FAMILY_COHORT_INVALID");
        }
        let expected = validate_git_sha(
            option_env!("ELON_NODE_AGENT_GIT_SHA")
                .ok_or("A2_JOINT_CLOSE_FAMILY_COMPILED_GIT_SHA_MISSING")?,
        )?;
        let manifest = Path::new(env!("CARGO_MANIFEST_DIR"));
        let root = manifest
            .parent()
            .ok_or("A2_JOINT_CLOSE_FAMILY_CHECKOUT_ROOT_INVALID")?;
        let head = git_output(root, &["rev-parse", "HEAD"])?;
        if head.trim() != expected {
            return Err("A2_JOINT_CLOSE_FAMILY_CHECKOUT_HEAD_MISMATCH");
        }
        let status = git_output(root, &["status", "--porcelain=v1", "--untracked-files=all"])?;
        if !status.trim().is_empty() {
            return Err("A2_JOINT_CLOSE_FAMILY_CHECKOUT_NOT_CLEAN");
        }
        Ok(Self {
            git_sha: expected.to_owned(),
            cohort_commitment: cohort.commitment,
        })
    }
}

fn git_output(root: &Path, arguments: &[&str]) -> Result<String, &'static str> {
    let output = Command::new("git")
        .arg("-C")
        .arg(root)
        .args(arguments)
        .output()
        .map_err(|_| "A2_JOINT_CLOSE_FAMILY_CHECKOUT_GIT_FAILED")?;
    if !output.status.success() {
        return Err("A2_JOINT_CLOSE_FAMILY_CHECKOUT_GIT_FAILED");
    }
    String::from_utf8(output.stdout).map_err(|_| "A2_JOINT_CLOSE_FAMILY_CHECKOUT_GIT_NON_UTF8")
}
