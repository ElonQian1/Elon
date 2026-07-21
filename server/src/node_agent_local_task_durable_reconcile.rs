//! Fail-closed repair for a supervised sidecar whose SQLite display row was lost.
//!
//! A sidecar proves liveness only. Durable ownership is reconstructed from the
//! journal contract, update receipt, current bound identity, an authorized base
//! workspace, the actual Git worktree shape, and the exact root lease.

use std::path::{Path, PathBuf};

use anyhow::{bail, Context, Result};
use serde_json::json;
use tracing::info;

use crate::{
    node_agent_cli_sidecar::CliSidecarSessionRecord,
    node_agent_local_task_store::{reconcile::RecoveredLocalTaskStart, LocalTaskRecord},
    node_agent_local_task_supervision::{
        load_supervision_contract, record_supervision_event, SupervisionContract,
        SUPERVISION_PROTOCOL,
    },
    node_agent_update_recovery::UpdateRecoveryReceipt,
    NodeRuntime,
};

const RECOVERED_PROMPT: &str =
    "[recovered supervised task: original local prompt row unavailable; inspect durable journal]";

pub(crate) async fn reconcile_missing_sidecar_task(
    runtime: &NodeRuntime,
    session: &CliSidecarSessionRecord,
) -> Result<Option<LocalTaskRecord>> {
    if let Some(task) = runtime.local_tasks.get(&session.task_id)? {
        return Ok(Some(task));
    }
    let receipt = runtime
        .update_recovery
        .receipt_for_task(&session.task_id)?
        .context("missing local task has no trusted update recovery receipt")?;
    let snapshot = runtime.task_journal.snapshot(&session.task_id, 0, 200)?;
    let journal = snapshot
        .record
        .as_ref()
        .context("missing local task has no durable journal record")?;
    let contract = load_supervision_contract(&runtime.task_journal, &session.task_id)?
        .context("missing local task has no durable supervision contract")?;
    let root_task_id = validate_contract_receipt(&session.task_id, &contract, &receipt)?;
    validate_runtime_evidence(session, journal, &receipt)?;

    let creds = runtime
        .creds()
        .await
        .context("node has no bound identity for local task reconciliation")?;
    let grant_identity = crate::node_agent_full_access::current_grant_identity(runtime).await?;
    let grants = runtime.full_access_grants.list(&grant_identity).await;
    let workspace = validate_workspace_identity(session, &receipt, root_task_id, &grants)?;
    let prompt = runtime
        .local_tasks
        .get_for_owner(&creds.owner_user_id, root_task_id)?
        .filter(|root| {
            root.agent_id == creds.agent_id
                && root.install_id == runtime.install_id
                && crate::node_agent_full_access::project_ids_equivalent(
                    &root.project_id,
                    &workspace.project_id,
                )
        })
        .map(|root| root.prompt)
        .unwrap_or_else(|| RECOVERED_PROMPT.to_string());
    let status = if journal.status == "cancel_requested" {
        "cancel_requested"
    } else {
        "recovering"
    };
    let record = runtime.local_tasks.reconcile_missing_supervised(
        RecoveredLocalTaskStart {
            task_id: &session.task_id,
            owner_user_id: &creds.owner_user_id,
            agent_id: &creds.agent_id,
            install_id: &runtime.install_id,
            project_id: &workspace.project_id,
            conversation_id: &workspace.conversation_id,
            workspace_path: &workspace.active.to_string_lossy(),
            prompt: &prompt,
            cli: &session.cli_name,
            runtime_permission: journal
                .runtime_permission
                .as_deref()
                .unwrap_or("full_access"),
            status,
            error: "节点更新恢复发现本机任务行缺失；已基于可信 journal、合同、授权工作区和 root lease 重建最小记录",
            workspace_status: &workspace.status,
            started_at_ms: journal.started_at_ms.min(i64::MAX as u128) as i64,
        },
    )?;
    record_supervision_event(
        &runtime.task_journal,
        &session.task_id,
        "supervision_local_task_reconciled",
        json!({
            "root_task_id": root_task_id,
            "project_id": workspace.project_id,
            "active_workspace_path": workspace.active,
            "sidecar_session_id": session.session_id,
            "source": "journal_contract_workspace_grant_root_lease",
        }),
    )?;
    info!(
        task_id = %session.task_id,
        %root_task_id,
        "reconstructed missing durable local task row from fail-closed evidence"
    );
    Ok(Some(record))
}

fn validate_contract_receipt<'a>(
    task_id: &'a str,
    contract: &'a SupervisionContract,
    receipt: &'a UpdateRecoveryReceipt,
) -> Result<&'a str> {
    anyhow::ensure!(
        contract.protocol == SUPERVISION_PROTOCOL,
        "unsupported supervision protocol"
    );
    anyhow::ensure!(
        receipt.allows_local_reconcile(),
        "untrusted recovery receipt"
    );
    anyhow::ensure!(
        receipt.active_task_id() == task_id,
        "recovery receipt targets another active task"
    );
    let contract_root = contract.root_task_id.as_deref().unwrap_or(task_id).trim();
    anyhow::ensure!(
        !contract_root.is_empty() && contract_root == receipt.root_task_id,
        "supervision contract and recovery root differ"
    );
    if contract.task_role == "requirement" {
        anyhow::ensure!(
            contract.parent_task_id.is_none()
                && contract
                    .root_task_id
                    .as_deref()
                    .is_none_or(|root| root == task_id),
            "requirement contract has a foreign parent or root"
        );
    } else {
        anyhow::ensure!(
            contract
                .parent_task_id
                .as_deref()
                .is_some_and(|id| !id.trim().is_empty())
                && contract.root_task_id.as_deref() == Some(contract_root),
            "supervision descendant contract is incomplete"
        );
    }
    Ok(contract_root)
}

fn validate_runtime_evidence(
    session: &CliSidecarSessionRecord,
    journal: &crate::node_agent_task_journal::TaskJournalRecord,
    receipt: &UpdateRecoveryReceipt,
) -> Result<()> {
    anyhow::ensure!(journal.req_id == session.task_id, "journal task id drifted");
    anyhow::ensure!(
        journal.cli_name.eq_ignore_ascii_case(&session.cli_name),
        "journal and sidecar CLI differ"
    );
    anyhow::ensure!(
        matches!(
            journal.status.as_str(),
            "running" | "recovering" | "reattaching" | "cancel_requested"
        ),
        "journal is not a recoverable active record"
    );
    anyhow::ensure!(
        receipt.sidecar_session_id.as_deref() == Some(session.session_id.as_str()),
        "recovery receipt sidecar identity drifted"
    );
    let cwd = required_session_cwd(session)?;
    anyhow::ensure!(
        journal
            .cwd
            .as_deref()
            .is_some_and(|value| same_path(Path::new(value), cwd)),
        "journal and sidecar workspace differ"
    );
    anyhow::ensure!(
        same_path(Path::new(&receipt.workspace.workspace_path), cwd),
        "recovery receipt and sidecar workspace differ"
    );
    Ok(())
}

#[derive(Debug)]
struct RecoveredWorkspaceIdentity {
    project_id: String,
    conversation_id: String,
    active: PathBuf,
    status: serde_json::Value,
}

fn validate_workspace_identity(
    session: &CliSidecarSessionRecord,
    receipt: &UpdateRecoveryReceipt,
    root_task_id: &str,
    grants: &[crate::node_agent_full_access::FullAccessGrant],
) -> Result<RecoveredWorkspaceIdentity> {
    anyhow::ensure!(
        receipt.workspace.isolated && receipt.workspace.has_sufficient_identity(),
        "recovery receipt lacks isolated workspace identity"
    );
    let active = canonical_directory(required_session_cwd(session)?, "active workspace")?;
    let base = canonical_directory(
        Path::new(
            receipt
                .workspace
                .base_workspace_path
                .as_deref()
                .context("recovery receipt lacks base workspace")?,
        ),
        "base workspace",
    )?;
    anyhow::ensure!(
        !same_path(&base, &active),
        "active workspace equals shared base"
    );
    let branch = git(&active, &["branch", "--show-current"])?;
    anyhow::ensure!(
        receipt.workspace.branch.as_deref() == Some(branch.as_str()),
        "recovery receipt branch drifted"
    );
    let (project_part, conversation_id) = platform_shape(&active, &branch)?;
    let project_id = authorized_project(grants, &base, &project_part)?;
    let top = PathBuf::from(git(&active, &["rev-parse", "--show-toplevel"])?);
    anyhow::ensure!(
        same_path(&top, &active),
        "active path is not the worktree root"
    );
    let active_common = PathBuf::from(git(
        &active,
        &["rev-parse", "--path-format=absolute", "--git-common-dir"],
    )?);
    let base_common = PathBuf::from(git(
        &base,
        &["rev-parse", "--path-format=absolute", "--git-common-dir"],
    )?);
    anyhow::ensure!(
        same_path(&active_common, &base_common),
        "active worktree belongs to another repository"
    );
    let remote = git(&base, &["config", "--get", "remote.origin.url"])?;
    anyhow::ensure!(
        !remote.trim().is_empty(),
        "base workspace has no origin remote"
    );
    let expected_lease = crate::node_agent_supervision_worktree_lease::lease_reason(root_task_id)?;
    anyhow::ensure!(
        crate::node_agent_supervision_worktree_lease::worktree_lock_reason(&base, &active)?
            .as_deref()
            == Some(expected_lease.as_str()),
        "exact root supervision lease is missing or foreign"
    );
    let head = git(&active, &["rev-parse", "--verify", "HEAD^{commit}"])?;
    let base_revision = git(&base, &["rev-parse", "--verify", "HEAD^{commit}"])?;
    Ok(RecoveredWorkspaceIdentity {
        project_id: project_id.clone(),
        conversation_id,
        active: active.clone(),
        status: json!({
            "platform_provenance": "elon.conversation_worktree.v1",
            "durable_reconcile_provenance": "journal_contract_workspace_grant_root_lease",
            "project_id": project_id,
            "root_task_id": root_task_id,
            "base_workspace_path": base,
            "active_workspace_path": active,
            "isolated": true,
            "branch": branch,
            "git_head": head,
            "base_revision": base_revision,
            "git_common_dir": active_common,
            "git_remote": remote,
            "prepare_status": "reconciled_missing_local_task_row",
            "merge_status": "preserved",
            "original_prompt_available": false,
        }),
    })
}

fn authorized_project(
    grants: &[crate::node_agent_full_access::FullAccessGrant],
    base: &Path,
    project_part: &str,
) -> Result<String> {
    let mut matches = grants
        .iter()
        .filter(|grant| {
            same_path(Path::new(&grant.workspace_path), base)
                && elon_pc_dev_runtime::safe_path_part(&grant.project_id, "project", 80)
                    .eq_ignore_ascii_case(project_part)
        })
        .map(|grant| grant.project_id.clone())
        .collect::<Vec<_>>();
    matches.sort_by_key(|value| value.to_ascii_lowercase());
    matches
        .dedup_by(|left, right| crate::node_agent_full_access::project_ids_equivalent(left, right));
    match matches.as_slice() {
        [project] => Ok(project.clone()),
        [] => bail!("base workspace is not authorized for the recovered project"),
        _ => bail!("base workspace has ambiguous project authorization"),
    }
}

fn platform_shape(active: &Path, branch: &str) -> Result<(String, String)> {
    let conversation = path_name(active, "conversation")?;
    let project = active
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
    anyhow::ensure!(
        marker.eq_ignore_ascii_case("conversation-worktrees"),
        "active workspace is not platform managed"
    );
    anyhow::ensure!(
        branch.eq_ignore_ascii_case(&format!("ai/session/{project}/{conversation}")),
        "platform path and branch identity differ"
    );
    Ok((project, conversation))
}

fn required_session_cwd(session: &CliSidecarSessionRecord) -> Result<&Path> {
    session
        .cwd
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(Path::new)
        .context("sidecar has no workspace")
}

fn canonical_directory(path: &Path, label: &str) -> Result<PathBuf> {
    let path = path
        .canonicalize()
        .with_context(|| format!("{label} is unavailable: {}", path.display()))?;
    anyhow::ensure!(path.is_dir(), "{label} is not a directory");
    Ok(path)
}

fn path_name(path: &Path, label: &str) -> Result<String> {
    path.file_name()
        .and_then(|name| name.to_str())
        .map(str::to_string)
        .with_context(|| format!("{label} path component is unavailable"))
}

fn git(cwd: &Path, args: &[&str]) -> Result<String> {
    crate::node_agent_update_checkpoint::git_output(cwd, args)
        .map(|value| value.trim().to_string())
        .with_context(|| format!("git {} failed in {}", args.join(" "), cwd.display()))
}

fn same_path(left: &Path, right: &Path) -> bool {
    crate::node_agent_update_checkpoint::same_path(left, right)
}

#[cfg(test)]
#[path = "node_agent_local_task_durable_reconcile_tests.rs"]
mod tests;
