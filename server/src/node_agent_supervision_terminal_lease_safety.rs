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

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum TerminalLeaseExpectation {
    Exact,
    Missing,
}

#[derive(Clone, Debug)]
pub(crate) struct VerifiedTerminalLeaseIdentity {
    pub(crate) base: PathBuf,
    pub(crate) active: PathBuf,
    pub(crate) root_task_id: String,
    pub(crate) git_dir: PathBuf,
    pub(crate) git_common_dir: PathBuf,
    pub(crate) head: String,
    pub(crate) finalized_workspace_missing: bool,
    pub(crate) task_id: String,
    pub(crate) project_id: String,
    pub(crate) workspace_path: String,
    pub(crate) workspace_status: serde_json::Value,
}

pub(crate) fn admission_base(
    task: &LocalTaskRecord,
    contract: &SupervisionContract,
    task_id: &str,
) -> Result<PathBuf> {
    anyhow::ensure!(
        contract.protocol == SUPERVISION_PROTOCOL,
        "terminal task supervision protocol is not trusted"
    );
    let root_task_id = supervision_root(contract, task_id)?;
    let status = task
        .workspace_status
        .as_ref()
        .context("supervised terminal task is missing durable workspace identity")?;
    anyhow::ensure!(
        required(status, "root_task_id")? == root_task_id,
        "terminal task root identity drifted"
    );
    canonical_directory(Path::new(required(status, "base_workspace_path")?), "base")
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
    if lineage_or_workspace_is_active(runtime, task_id, &root_task_id, &active, false).await? {
        return Ok(None);
    }
    Ok(Some(build_verified_identity(
        task,
        status,
        base,
        active,
        root_task_id,
    )?))
}

pub(crate) async fn verify_terminal_identity(
    runtime: &NodeRuntime,
    task: &LocalTaskRecord,
    contract: &SupervisionContract,
    task_id: &str,
    lease: TerminalLeaseExpectation,
) -> Result<VerifiedTerminalLeaseIdentity> {
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
    anyhow::ensure!(
        !lineage_or_workspace_is_active(runtime, task_id, &root_task_id, &active, true).await?,
        "terminal workspace or supervision lineage still has another active owner"
    );
    let identity = build_verified_identity(task, status, base, active, root_task_id)?;
    identity.revalidate_lease(lease)?;
    Ok(identity)
}

fn build_verified_identity(
    task: &LocalTaskRecord,
    status: &serde_json::Value,
    base: PathBuf,
    active: PathBuf,
    root_task_id: String,
) -> Result<VerifiedTerminalLeaseIdentity> {
    let git_dir = PathBuf::from(git(
        &active,
        &["rev-parse", "--path-format=absolute", "--git-dir"],
    )?);
    let git_common_dir = PathBuf::from(git(
        &active,
        &["rev-parse", "--path-format=absolute", "--git-common-dir"],
    )?);
    let head = git(&active, &["rev-parse", "--verify", "HEAD^{commit}"])?;
    Ok(VerifiedTerminalLeaseIdentity {
        base,
        active,
        root_task_id,
        git_dir,
        git_common_dir,
        head,
        finalized_workspace_missing: false,
        task_id: task.task_id.clone(),
        project_id: task.project_id.clone(),
        workspace_path: task.workspace_path.clone(),
        workspace_status: status.clone(),
    })
}

pub(crate) async fn verify_finalized_terminal_identity(
    runtime: &NodeRuntime,
    task: &LocalTaskRecord,
    contract: &SupervisionContract,
    task_id: &str,
    evidence: &crate::node_agent_supervision_finalized_identity::FinalizedIdentityEvidence,
) -> Result<VerifiedTerminalLeaseIdentity> {
    if Path::new(&task.workspace_path).is_dir() {
        return verify_terminal_identity(
            runtime,
            task,
            contract,
            task_id,
            TerminalLeaseExpectation::Missing,
        )
        .await;
    }
    crate::node_agent_supervision_finalized_identity::verify(
        runtime, task, contract, task_id, evidence,
    )
    .await
}

impl VerifiedTerminalLeaseIdentity {
    pub(crate) fn revalidate_lease(&self, expected: TerminalLeaseExpectation) -> Result<()> {
        if self.finalized_workspace_missing {
            anyhow::ensure!(
                !self.active.exists(),
                "cleaned terminal worktree path reappeared during finalization"
            );
        }
        let actual = crate::node_agent_supervision_worktree_lease::worktree_lock_reason(
            &self.base,
            &self.active,
        )?;
        let exact = crate::node_agent_supervision_worktree_lease::lease_reason(&self.root_task_id)?;
        match expected {
            TerminalLeaseExpectation::Exact => anyhow::ensure!(
                actual.as_deref() == Some(exact.as_str()),
                "non-finalized terminal completion requires the exact supervision lease"
            ),
            TerminalLeaseExpectation::Missing => anyhow::ensure!(
                actual.is_none(),
                "completed terminal finalization retained or reacquired a supervision lease"
            ),
        }
        Ok(())
    }

    pub(crate) fn trusted_workspace_status(
        &self,
        task_id: &str,
        project_id: &str,
        workspace_path: &str,
        current: &serde_json::Value,
    ) -> Result<String> {
        anyhow::ensure!(
            task_id == self.task_id
                && crate::node_agent_full_access::project_ids_equivalent(
                    project_id,
                    &self.project_id,
                )
                && same_path(Path::new(workspace_path), Path::new(&self.workspace_path))
                && current == &self.workspace_status,
            "local terminal workspace row drifted after trusted preflight"
        );
        let mut status = current.clone();
        let object = status
            .as_object_mut()
            .context("supervised workspace_status is not an object")?;
        object.insert(
            "git_head".into(),
            serde_json::Value::String(self.head.clone()),
        );
        object.insert(
            "terminal_snapshot_status".into(),
            serde_json::Value::String("trusted".into()),
        );
        object.remove("resume_blocked_reason");
        serde_json::to_string(&status).map_err(Into::into)
    }

    pub(crate) fn verify_successful_git_state(
        &self,
        final_head: &str,
        base_commit: &str,
        branch: &str,
        origin: &str,
        common_dir: &Path,
    ) -> Result<()> {
        if self.finalized_workspace_missing {
            return crate::node_agent_supervision_finalized_identity::revalidate_repository(
                &self.base,
                &self.git_common_dir,
                &self.head,
                final_head,
                base_commit,
                origin,
                common_dir,
            );
        }
        anyhow::ensure!(
            git(&self.active, &["rev-parse", "--verify", "HEAD^{commit}"])? == self.head
                && self.head == final_head,
            "terminal finalHead drifted from current HEAD"
        );
        anyhow::ensure!(
            git(&self.active, &["branch", "--show-current"])? == branch,
            "terminal finalization branch drifted"
        );
        anyhow::ensure!(
            git(&self.base, &["branch", "--show-current"])? == "main",
            "terminal base workspace is not main"
        );
        anyhow::ensure!(
            git(&self.active, &["remote", "get-url", "origin"])? == origin
                && git(&self.base, &["remote", "get-url", "origin"])? == origin,
            "terminal finalization origin drifted"
        );
        anyhow::ensure!(
            same_path(common_dir, &self.git_common_dir)
                && same_path(
                    Path::new(&git(
                        &self.active,
                        &["rev-parse", "--path-format=absolute", "--git-common-dir"]
                    )?),
                    &self.git_common_dir,
                )
                && same_path(
                    Path::new(&git(
                        &self.base,
                        &["rev-parse", "--path-format=absolute", "--git-common-dir"]
                    )?),
                    &self.git_common_dir,
                ),
            "terminal finalization common-dir drifted"
        );
        anyhow::ensure!(
            git(
                &self.active,
                &["status", "--porcelain=v1", "--untracked-files=all"]
            )?
            .is_empty(),
            "terminal worktree is dirty after finalization"
        );
        git_success(
            &self.active,
            &["merge-base", "--is-ancestor", base_commit, final_head],
            "TaskContract base is not an ancestor of finalHead",
        )?;
        git_success(
            &self.active,
            &["merge-base", "--is-ancestor", final_head, "origin/main"],
            "terminal finalHead is not landed in origin/main",
        )
    }
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

pub(crate) async fn lineage_or_workspace_is_active(
    runtime: &NodeRuntime,
    current_task_id: &str,
    root_task_id: &str,
    active: &Path,
    allow_current_prompt: bool,
) -> Result<bool> {
    ownership_in_lineage_or_workspace_is_active(
        runtime,
        current_task_id,
        root_task_id,
        active,
        allow_current_prompt,
        true,
    )
    .await
}

/// Orphan reconciliation already holds the cross-process admission guard and
/// proves concrete runtime ownership separately. Ignoring sibling database
/// rows here prevents two stale nonterminal descendants from keeping each
/// other alive forever, while live handles, sidecars, and worker PIDs remain
/// fail-closed blockers.
pub(crate) async fn runtime_lineage_or_workspace_is_active(
    runtime: &NodeRuntime,
    current_task_id: &str,
    root_task_id: &str,
    active: &Path,
) -> Result<bool> {
    ownership_in_lineage_or_workspace_is_active(
        runtime,
        current_task_id,
        root_task_id,
        active,
        false,
        false,
    )
    .await
}

async fn ownership_in_lineage_or_workspace_is_active(
    runtime: &NodeRuntime,
    current_task_id: &str,
    root_task_id: &str,
    active: &Path,
    allow_current_prompt: bool,
    persisted_candidates_block: bool,
) -> Result<bool> {
    if persisted_candidates_block {
        for candidate in runtime.local_tasks.list_identity_candidates()? {
            if candidate.task_id == current_task_id || is_terminal(&candidate.status) {
                continue;
            }
            let same_workspace = candidate_shares_workspace(&candidate, active);
            if !candidate_requires_contract_lookup(
                candidate.workspace_status.as_ref(),
                same_workspace,
                root_task_id,
            ) {
                continue;
            }
            let contract = load_supervision_contract(&runtime.task_journal, &candidate.task_id)?;
            if candidate_blocks_release(&candidate, contract.as_ref(), root_task_id, active)? {
                return Ok(true);
            }
        }
    }
    for handle in runtime.active_cli_prompts.views_without_approvals().await {
        if handle.req_id == current_task_id {
            if allow_current_prompt {
                continue;
            }
            return Ok(true);
        }
        let shares_workspace = handle
            .cwd
            .as_deref()
            .is_some_and(|cwd| workspace_contains(active, Path::new(cwd)));
        if shares_workspace {
            return Ok(true);
        }
        let handle_task = runtime.local_tasks.get(&handle.req_id)?;
        if candidate_requires_contract_lookup(
            handle_task
                .as_ref()
                .and_then(|task| task.workspace_status.as_ref()),
            false,
            root_task_id,
        ) && task_contract_has_root(runtime, &handle.req_id, root_task_id)?
        {
            return Ok(true);
        }
    }
    let now = crate::node_agent_cli_sidecar::now_ms();
    for sidecar in runtime.cli_sidecars.all_sessions()? {
        // Dead sidecar metadata has no execution ownership. Skip it before
        // lineage parsing so unrelated legacy contracts cannot block every new
        // terminal task or make the periodic pass scan the full history.
        if !sidecar.is_live_at(now) || sidecar.task_id == current_task_id {
            continue;
        }
        let shares_workspace = sidecar
            .cwd
            .as_deref()
            .is_some_and(|cwd| workspace_contains(active, Path::new(cwd)));
        let sidecar_task = runtime.local_tasks.get(&sidecar.task_id)?;
        let shares_workspace_or_root = shares_workspace
            || (candidate_requires_contract_lookup(
                sidecar_task
                    .as_ref()
                    .and_then(|task| task.workspace_status.as_ref()),
                false,
                root_task_id,
            ) && task_contract_has_root(runtime, &sidecar.task_id, root_task_id)?);
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
    let same_workspace = candidate_shares_workspace(candidate, active);
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

fn candidate_shares_workspace(candidate: &LocalTaskRecord, active: &Path) -> bool {
    same_path(Path::new(&candidate.workspace_path), active)
        || candidate.workspace_status.as_ref().is_some_and(|status| {
            status
                .get("active_workspace_path")
                .and_then(serde_json::Value::as_str)
                .is_some_and(|path| same_path(Path::new(path), active))
        })
}

fn candidate_requires_contract_lookup(
    workspace_status: Option<&serde_json::Value>,
    same_workspace: bool,
    root_task_id: &str,
) -> bool {
    same_workspace
        || workspace_status
            .and_then(|status| status.get("root_task_id"))
            .and_then(serde_json::Value::as_str)
            .map(str::trim)
            .filter(|root| !root.is_empty())
            .is_none_or(|root| root == root_task_id)
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
#[path = "node_agent_supervision_terminal_lease_safety_tests.rs"]
mod tests;
