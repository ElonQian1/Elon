//! Missing-worktree branch of the existing supervised terminal identity gate.
//!
//! PowerShell finalization may remove the registered worktree and branch before
//! the CLI completion reaches the node. This module validates the immutable
//! receipt/TaskContract evidence against the still-live common repository,
//! durable task identity, full-access grant, and supervision lineage.

use std::path::{Path, PathBuf};

use anyhow::{Context, Result};

use crate::{
    node_agent_local_task_store::LocalTaskRecord,
    node_agent_local_task_supervision::{SupervisionContract, SUPERVISION_PROTOCOL},
    NodeRuntime,
};

pub(crate) struct FinalizedIdentityEvidence {
    pub(crate) worktree: PathBuf,
    pub(crate) base_workspace: PathBuf,
    pub(crate) git_dir: PathBuf,
    pub(crate) git_common_dir: PathBuf,
    pub(crate) branch: String,
    pub(crate) origin: String,
    pub(crate) final_head: String,
    pub(crate) base_commit: String,
}

pub(crate) async fn verify(
    runtime: &NodeRuntime,
    task: &LocalTaskRecord,
    contract: &SupervisionContract,
    task_id: &str,
    evidence: &FinalizedIdentityEvidence,
) -> Result<crate::node_agent_supervision_terminal_lease_safety::VerifiedTerminalLeaseIdentity> {
    anyhow::ensure!(
        contract.protocol == SUPERVISION_PROTOCOL,
        "terminal task supervision protocol is not trusted"
    );
    let root_task_id = supervision_root(contract, task_id)?.to_string();
    let status = task
        .workspace_status
        .as_ref()
        .context("supervised terminal task is missing durable workspace identity")?;
    validate_status(status, task, &root_task_id)?;

    let creds = runtime
        .creds()
        .await
        .context("node has no bound identity for terminal lease reconciliation")?;
    anyhow::ensure!(
        task.owner_user_id == creds.owner_user_id
            && task.agent_id == creds.agent_id
            && task.install_id == runtime.install_id,
        "terminal task owner/node/install identity drifted"
    );

    let base = canonical_directory(Path::new(required(status, "base_workspace_path")?), "base")?;
    let active = absolute_recorded_path(required(status, "active_workspace_path")?, "active")?;
    anyhow::ensure!(
        !active.exists(),
        "missing-worktree finalization refuses a reappeared active workspace"
    );
    anyhow::ensure!(
        !same_path(&base, &active),
        "refusing to trust the shared base workspace as a finalized task worktree"
    );
    anyhow::ensure!(
        same_path(Path::new(&task.workspace_path), &active)
            && same_path(&evidence.worktree, &active)
            && same_path(&evidence.base_workspace, &base),
        "finalized terminal worktree/base identity drifted"
    );
    validate_platform_shape(&active, &task.project_id, &evidence.branch)?;

    let recorded_common =
        absolute_recorded_path(required(status, "git_common_dir")?, "common-dir")?;
    let base_common = PathBuf::from(git(
        &base,
        &["rev-parse", "--path-format=absolute", "--git-common-dir"],
    )?);
    anyhow::ensure!(
        same_path(&recorded_common, &base_common)
            && same_path(&evidence.git_common_dir, &base_common),
        "finalized terminal Git common-dir drifted"
    );
    anyhow::ensure!(
        required(status, "branch")? == evidence.branch
            && required(status, "git_remote")? == evidence.origin
            && git(&base, &["remote", "get-url", "origin"])? == evidence.origin,
        "finalized terminal branch or origin drifted"
    );

    let grant_identity = crate::node_agent_full_access::current_grant_identity(runtime).await?;
    anyhow::ensure!(
        runtime
            .full_access_grants
            .list(&grant_identity)
            .await
            .iter()
            .any(|grant| {
                crate::node_agent_full_access::project_ids_equivalent(
                    &grant.project_id,
                    &task.project_id,
                ) && same_path(Path::new(&grant.workspace_path), &base)
            }),
        "terminal task project/base workspace authorization drifted"
    );
    anyhow::ensure!(
        !crate::node_agent_supervision_terminal_lease_safety::lineage_or_workspace_is_active(
            runtime,
            task_id,
            &root_task_id,
            &active,
            true,
        )
        .await?,
        "terminal workspace or supervision lineage still has another active owner"
    );

    git_success(
        &base,
        &[
            "cat-file",
            "-e",
            &format!("{}^{{commit}}", evidence.final_head),
        ],
        "terminal finalHead is unavailable in the common repository",
    )?;
    git_success(
        &base,
        &[
            "merge-base",
            "--is-ancestor",
            &evidence.base_commit,
            &evidence.final_head,
        ],
        "TaskContract base is not an ancestor of finalHead",
    )?;
    git_success(
        &base,
        &[
            "merge-base",
            "--is-ancestor",
            &evidence.final_head,
            "origin/main",
        ],
        "terminal finalHead is not landed in origin/main",
    )?;

    Ok(
        crate::node_agent_supervision_terminal_lease_safety::VerifiedTerminalLeaseIdentity {
            base,
            active,
            root_task_id,
            git_dir: evidence.git_dir.clone(),
            git_common_dir: base_common,
            head: evidence.final_head.clone(),
            finalized_workspace_missing: true,
            task_id: task.task_id.clone(),
            project_id: task.project_id.clone(),
            workspace_path: task.workspace_path.clone(),
            workspace_status: status.clone(),
        },
    )
}

pub(crate) fn revalidate_repository(
    base: &Path,
    expected_common: &Path,
    expected_head: &str,
    final_head: &str,
    base_commit: &str,
    origin: &str,
    common_dir: &Path,
) -> Result<()> {
    anyhow::ensure!(
        expected_head == final_head,
        "terminal finalHead drifted from durable finalized identity"
    );
    anyhow::ensure!(
        git(base, &["branch", "--show-current"])? == "main",
        "terminal base workspace is not main"
    );
    anyhow::ensure!(
        git(base, &["remote", "get-url", "origin"])? == origin,
        "terminal finalization origin drifted"
    );
    let live_common = PathBuf::from(git(
        base,
        &["rev-parse", "--path-format=absolute", "--git-common-dir"],
    )?);
    anyhow::ensure!(
        same_path(common_dir, expected_common) && same_path(&live_common, expected_common),
        "terminal finalization common-dir drifted"
    );
    git_success(
        base,
        &["merge-base", "--is-ancestor", base_commit, final_head],
        "TaskContract base is not an ancestor of finalHead",
    )?;
    git_success(
        base,
        &["merge-base", "--is-ancestor", final_head, "origin/main"],
        "terminal finalHead is not landed in origin/main",
    )
}

fn validate_status(
    status: &serde_json::Value,
    task: &LocalTaskRecord,
    root_task_id: &str,
) -> Result<()> {
    anyhow::ensure!(
        status
            .get("platform_provenance")
            .and_then(serde_json::Value::as_str)
            == Some("elon.conversation_worktree.v1")
            && status.get("isolated").and_then(serde_json::Value::as_bool) == Some(true),
        "terminal task is not an isolated platform supervision worktree"
    );
    anyhow::ensure!(
        crate::node_agent_full_access::project_ids_equivalent(
            required(status, "project_id")?,
            &task.project_id,
        ) && required(status, "root_task_id")? == root_task_id,
        "terminal task project or root identity drifted"
    );
    Ok(())
}

fn supervision_root<'a>(contract: &'a SupervisionContract, task_id: &'a str) -> Result<&'a str> {
    let root = contract.root_task_id.as_deref().unwrap_or(task_id).trim();
    anyhow::ensure!(!root.is_empty(), "supervision root is empty");
    if contract.task_role == "requirement" {
        anyhow::ensure!(
            contract.parent_task_id.is_none()
                && contract
                    .root_task_id
                    .as_deref()
                    .is_none_or(|value| value == task_id),
            "requirement root contract is invalid"
        );
    } else {
        anyhow::ensure!(
            contract.root_task_id.as_deref() == Some(root)
                && contract
                    .parent_task_id
                    .as_deref()
                    .is_some_and(|value| !value.trim().is_empty()),
            "supervision descendant contract is incomplete"
        );
    }
    Ok(root)
}

fn validate_platform_shape(active: &Path, project: &str, branch: &str) -> Result<()> {
    let conversation = path_name(active, "conversation")?;
    let project_dir = active
        .parent()
        .map(|path| path_name(path, "project"))
        .transpose()?
        .context("active workspace has no project directory")?;
    let marker = active
        .parent()
        .and_then(Path::parent)
        .map(|path| path_name(path, "platform marker"))
        .transpose()?
        .context("active workspace has no platform marker")?;
    let expected_project = elon_pc_dev_runtime::safe_path_part(project, "project", 80);
    anyhow::ensure!(
        marker.eq_ignore_ascii_case("conversation-worktrees")
            && project_dir.eq_ignore_ascii_case(&expected_project)
            && branch
                .eq_ignore_ascii_case(&format!("ai/session/{expected_project}/{conversation}")),
        "terminal task platform path/branch/project identity drifted"
    );
    Ok(())
}

fn required<'a>(status: &'a serde_json::Value, field: &str) -> Result<&'a str> {
    status
        .get(field)
        .and_then(serde_json::Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .with_context(|| format!("terminal task workspace identity is missing {field}"))
}

fn canonical_directory(path: &Path, label: &str) -> Result<PathBuf> {
    let path = path
        .canonicalize()
        .with_context(|| format!("terminal {label} workspace is unavailable"))?;
    anyhow::ensure!(
        path.is_dir(),
        "terminal {label} workspace is not a directory"
    );
    Ok(path)
}

fn absolute_recorded_path(value: &str, label: &str) -> Result<PathBuf> {
    let path = PathBuf::from(value);
    anyhow::ensure!(path.is_absolute(), "terminal {label} path is not absolute");
    Ok(path)
}

fn path_name(path: &Path, label: &str) -> Result<String> {
    path.file_name()
        .and_then(|name| name.to_str())
        .map(str::to_string)
        .with_context(|| format!("terminal {label} path component is unavailable"))
}

fn git(cwd: &Path, args: &[&str]) -> Result<String> {
    crate::node_agent_update_checkpoint::git_output(cwd, args)
        .map(|value| value.trim().to_string())
        .with_context(|| {
            format!(
                "cannot inspect finalized Git identity: git {}",
                args.join(" ")
            )
        })
}

fn git_success(cwd: &Path, args: &[&str], message: &str) -> Result<()> {
    let status = crate::git_command_error::git_command()
        .args(args)
        .current_dir(cwd)
        .status()?;
    anyhow::ensure!(status.success(), "{message}");
    Ok(())
}

fn same_path(left: &Path, right: &Path) -> bool {
    crate::node_agent_update_checkpoint::same_path(left, right)
}
