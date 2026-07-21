//! Provenance and whole-lineage safety checks before a terminal lease release.

use std::path::{Path, PathBuf};

use anyhow::{bail, Context, Result};

use crate::{
    node_agent_local_task_store::LocalTaskRecord,
    node_agent_local_task_supervision::{
        load_supervision_contract, SupervisionContract, SUPERVISION_PROTOCOL,
    },
    NodeRuntime,
};

pub(crate) struct VerifiedTerminalLeaseIdentity {
    pub(crate) base: PathBuf,
    pub(crate) active: PathBuf,
    pub(crate) root_task_id: String,
}

pub(crate) async fn verify_release_identity(
    runtime: &NodeRuntime,
    task: &LocalTaskRecord,
    contract: &SupervisionContract,
    task_id: &str,
) -> Result<Option<VerifiedTerminalLeaseIdentity>> {
    anyhow::ensure!(
        contract.protocol == SUPERVISION_PROTOCOL,
        "terminal task supervision protocol is not trusted"
    );
    let root_task_id = supervision_root(contract, task_id)?.to_string();
    let status = task
        .workspace_status
        .as_ref()
        .context("supervised terminal task is missing durable workspace identity")?;
    let (base, active) = validate_recorded_identity(runtime, task, status, &root_task_id).await?;
    if lineage_or_workspace_is_active(runtime, task_id, &root_task_id, &active).await? {
        return Ok(None);
    }
    Ok(Some(VerifiedTerminalLeaseIdentity {
        base,
        active,
        root_task_id,
    }))
}

async fn validate_recorded_identity(
    runtime: &NodeRuntime,
    task: &LocalTaskRecord,
    status: &serde_json::Value,
    root_task_id: &str,
) -> Result<(PathBuf, PathBuf)> {
    anyhow::ensure!(
        status
            .get("platform_provenance")
            .and_then(serde_json::Value::as_str)
            == Some("elon.conversation_worktree.v1"),
        "terminal task workspace is not a platform supervision worktree"
    );
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
    required_project(status, &task.project_id)?;
    anyhow::ensure!(
        required(status, "root_task_id")? == root_task_id,
        "terminal task root identity drifted"
    );
    anyhow::ensure!(
        status.get("isolated").and_then(serde_json::Value::as_bool) == Some(true),
        "terminal task is not an isolated worktree"
    );
    let base = canonical_directory(Path::new(required(status, "base_workspace_path")?), "base")?;
    let active = canonical_directory(
        Path::new(required(status, "active_workspace_path")?),
        "active",
    )?;
    anyhow::ensure!(
        !same_path(&base, &active),
        "refusing to unlock shared base workspace"
    );
    let grant_identity = crate::node_agent_full_access::current_grant_identity(runtime).await?;
    let grants = runtime.full_access_grants.list(&grant_identity).await;
    anyhow::ensure!(
        grants.iter().any(|grant| {
            crate::node_agent_full_access::project_ids_equivalent(
                &grant.project_id,
                &task.project_id,
            ) && same_path(Path::new(&grant.workspace_path), &base)
        }),
        "terminal task project/base workspace authorization drifted"
    );
    anyhow::ensure!(
        same_path(Path::new(&task.workspace_path), &active),
        "terminal task active workspace identity drifted"
    );

    let expected_branch = required(status, "branch")?;
    validate_platform_shape(&active, &task.project_id, expected_branch)?;
    anyhow::ensure!(
        git(&active, &["branch", "--show-current"])? == expected_branch,
        "terminal task branch drifted"
    );
    let top = PathBuf::from(git(&active, &["rev-parse", "--show-toplevel"])?);
    anyhow::ensure!(
        same_path(&top, &active),
        "active workspace is not a worktree root"
    );
    let recorded_common = PathBuf::from(required(status, "git_common_dir")?);
    let active_common = PathBuf::from(git(
        &active,
        &["rev-parse", "--path-format=absolute", "--git-common-dir"],
    )?);
    let base_common = PathBuf::from(git(
        &base,
        &["rev-parse", "--path-format=absolute", "--git-common-dir"],
    )?);
    anyhow::ensure!(
        same_path(&recorded_common, &active_common) && same_path(&base_common, &active_common),
        "terminal task Git common-dir drifted"
    );
    anyhow::ensure!(
        required(status, "git_remote")? == git(&base, &["config", "--get", "remote.origin.url"])?,
        "terminal task origin remote drifted"
    );
    let expected_lease = crate::node_agent_supervision_worktree_lease::lease_reason(root_task_id)?;
    match crate::node_agent_supervision_worktree_lease::worktree_lock_reason(&base, &active)? {
        None => return Ok((base, active)),
        Some(actual) if actual == expected_lease => {}
        Some(actual) => bail!("refusing to release foreign terminal lease: {actual}"),
    }
    Ok((base, active))
}

async fn lineage_or_workspace_is_active(
    runtime: &NodeRuntime,
    current_task_id: &str,
    root_task_id: &str,
    active: &Path,
) -> Result<bool> {
    for candidate in runtime.local_tasks.list_identity_candidates()? {
        if candidate.task_id == current_task_id || is_terminal(&candidate.status) {
            continue;
        }
        let contract = load_supervision_contract(&runtime.task_journal, &candidate.task_id)?;
        if candidate_blocks_release(&candidate, contract.as_ref(), root_task_id, active)? {
            return Ok(true);
        }
    }
    for handle in runtime.active_cli_prompts.views_without_approvals().await {
        if handle.req_id == current_task_id
            || handle
                .cwd
                .as_deref()
                .is_some_and(|cwd| workspace_contains(active, Path::new(cwd)))
            || task_contract_has_root(runtime, &handle.req_id, root_task_id)?
        {
            return Ok(true);
        }
    }
    let now = crate::node_agent_cli_sidecar::now_ms();
    for sidecar in runtime.cli_sidecars.all_sessions()? {
        // Dead sidecar metadata has no execution ownership. Skip it before
        // lineage parsing so unrelated legacy contracts cannot block every new
        // terminal task or make the periodic pass scan the full history.
        if !sidecar.is_live_at(now) {
            continue;
        }
        let shares_workspace_or_root = sidecar
            .cwd
            .as_deref()
            .is_some_and(|cwd| workspace_contains(active, Path::new(cwd)))
            || task_contract_has_root(runtime, &sidecar.task_id, root_task_id)?;
        let sidecar_task = runtime.local_tasks.get(&sidecar.task_id)?;
        if sidecar_metadata_blocks_release(
            current_task_id,
            &sidecar.task_id,
            sidecar_task.as_ref().map(|task| task.status.as_str()),
            true,
            shares_workspace_or_root,
        ) {
            return Ok(true);
        }
    }
    Ok(false)
}

fn sidecar_metadata_blocks_release(
    current_task_id: &str,
    sidecar_task_id: &str,
    sidecar_task_status: Option<&str>,
    sidecar_is_live: bool,
    shares_workspace_or_root: bool,
) -> bool {
    sidecar_is_live
        && shares_workspace_or_root
        && sidecar_task_id != current_task_id
        && sidecar_task_status.is_none_or(|status| !is_terminal(status))
}

fn candidate_blocks_release(
    candidate: &LocalTaskRecord,
    contract: Option<&SupervisionContract>,
    root_task_id: &str,
    active: &Path,
) -> Result<bool> {
    let same_workspace = same_path(Path::new(&candidate.workspace_path), active)
        || candidate.workspace_status.as_ref().is_some_and(|status| {
            status
                .get("active_workspace_path")
                .and_then(serde_json::Value::as_str)
                .is_some_and(|path| same_path(Path::new(path), active))
        });
    let Some(contract) = contract else {
        if same_workspace {
            bail!("active workspace has a nonterminal task without a supervision contract");
        }
        return Ok(false);
    };
    anyhow::ensure!(
        contract.protocol == SUPERVISION_PROTOCOL,
        "nonterminal workspace task has an untrusted supervision protocol"
    );
    let candidate_root = supervision_root(contract, &candidate.task_id)?;
    if same_workspace && candidate_root != root_task_id {
        bail!("active workspace is occupied by another supervision root");
    }
    Ok(same_workspace || candidate_root == root_task_id)
}

fn task_contract_has_root(runtime: &NodeRuntime, task_id: &str, root: &str) -> Result<bool> {
    let Some(contract) = load_supervision_contract(&runtime.task_journal, task_id)? else {
        return Ok(false);
    };
    Ok(contract.protocol == SUPERVISION_PROTOCOL && supervision_root(&contract, task_id)? == root)
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

fn required_project(status: &serde_json::Value, project: &str) -> Result<()> {
    anyhow::ensure!(
        crate::node_agent_full_access::project_ids_equivalent(
            required(status, "project_id")?,
            project,
        ),
        "terminal task project identity drifted"
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
                "cannot inspect terminal Git identity: git {}",
                args.join(" ")
            )
        })
}

fn same_path(left: &Path, right: &Path) -> bool {
    crate::node_agent_update_checkpoint::same_path(left, right)
}

fn workspace_contains(workspace: &Path, candidate: &Path) -> bool {
    let workspace = crate::node_agent_workspace_match::canonical_or_original(workspace);
    let candidate = crate::node_agent_workspace_match::canonical_or_original(candidate);
    candidate.starts_with(workspace)
}

fn is_terminal(status: &str) -> bool {
    matches!(
        status.trim().to_ascii_lowercase().as_str(),
        "done" | "failed" | "canceled" | "cancelled" | "finished"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn terminal_parent_cannot_release_while_resume_descendant_is_active() {
        let active = Path::new("C:/workspace/conversation-worktrees/elon-self/root");
        let task = record("resume-child", active);
        let contract = descendant("root-task");
        assert!(candidate_blocks_release(&task, Some(&contract), "root-task", active).unwrap());
        assert!(candidate_blocks_release(
            &record("same-root-other-worktree", Path::new("C:/other")),
            Some(&contract),
            "root-task",
            active,
        )
        .unwrap());
    }

    #[test]
    fn unknown_or_wrong_root_workspace_occupancy_fails_closed() {
        let active = Path::new("C:/workspace/conversation-worktrees/elon-self/root");
        let task = record("unknown", active);
        assert!(candidate_blocks_release(&task, None, "root-task", active).is_err());
        assert!(candidate_blocks_release(
            &task,
            Some(&descendant("foreign-root")),
            "root-task",
            active,
        )
        .is_err());
    }

    #[test]
    fn terminal_sidecar_metadata_does_not_outlive_execution_ownership() {
        assert!(!sidecar_metadata_blocks_release(
            "terminal-parent",
            "terminal-parent",
            Some("done"),
            true,
            true,
        ));
        assert!(!sidecar_metadata_blocks_release(
            "terminal-parent",
            "terminal-sibling",
            Some("failed"),
            true,
            true,
        ));
        assert!(sidecar_metadata_blocks_release(
            "terminal-parent",
            "running-descendant",
            Some("running"),
            true,
            true,
        ));
        assert!(sidecar_metadata_blocks_release(
            "terminal-parent",
            "unknown-live-task",
            None,
            true,
            true,
        ));
    }

    fn descendant(root: &str) -> SupervisionContract {
        SupervisionContract {
            protocol: SUPERVISION_PROTOCOL.to_string(),
            supervisor: "codex_desktop".to_string(),
            task_role: "resume_original".to_string(),
            parent_task_id: Some("old-parent".to_string()),
            root_task_id: Some(root.to_string()),
            acceptance_criteria: vec![],
            improvement_policy: "after_task_only".to_string(),
        }
    }

    fn record(task_id: &str, workspace: &Path) -> LocalTaskRecord {
        LocalTaskRecord {
            task_id: task_id.to_string(),
            owner_user_id: "owner".to_string(),
            agent_id: "agent".to_string(),
            install_id: "install".to_string(),
            project_id: "elon-self".to_string(),
            channel_id: None,
            conversation_id: "root".to_string(),
            workspace_path: workspace.to_string_lossy().to_string(),
            prompt: "prompt".to_string(),
            cli: "codex".to_string(),
            runtime_permission: "full_access".to_string(),
            execution_origin: "local_offline".to_string(),
            billing_source: "own_codex".to_string(),
            status: "running".to_string(),
            error: None,
            final_reply: None,
            model: None,
            codex_session_id: None,
            input_tokens: None,
            cached_input_tokens: None,
            output_tokens: None,
            reasoning_tokens: None,
            total_tokens: None,
            workspace_status: None,
            sync_state: "local_only".to_string(),
            completion_event_id: None,
            started_at_ms: 1,
            finished_at_ms: None,
            server_ack_at_ms: None,
        }
    }
}
