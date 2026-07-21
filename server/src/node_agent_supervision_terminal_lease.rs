//! Terminal-state reconciliation for Desktop-supervised worktree leases.
//!
//! Review remains an independent verdict. Execution ownership ends once the
//! durable local task is terminal, so its exact root-bound Git lease must not
//! remain held while a reviewer is unavailable. Dirty worktrees are never
//! removed here; normal cleanup keeps preserving unknown changes.

use std::{path::Path, sync::Arc, time::Duration};

use anyhow::{bail, Context, Result};
use serde_json::json;
use tracing::{info, warn};

use crate::{
    node_agent_local_task_store::LocalTaskRecord,
    node_agent_local_task_supervision::{
        load_supervision_contract, record_supervision_event, SupervisionContract,
    },
    NodeRuntime,
};

pub(crate) async fn reconcile_task(runtime: &NodeRuntime, task_id: &str) -> Result<bool> {
    let Some(task) = runtime.local_tasks.get(task_id)? else {
        return Ok(false);
    };
    let Some(contract) = load_supervision_contract(&runtime.task_journal, task_id)? else {
        return Ok(false);
    };
    let cancel_side_effect_committed = if task.status == "cancel_requested" {
        runtime
            .task_journal
            .snapshot(task_id, 0, 1)?
            .record
            .and_then(|record| record.cancel_intent)
            .is_some_and(|intent| intent.side_effect.is_some())
    } else {
        false
    };
    let base = required_path(
        task.workspace_status
            .as_ref()
            .context("supervised terminal task is missing durable workspace identity")?,
        "base_workspace_path",
    )?;
    // Serialize terminal release with Resume's lease reacquisition and active
    // registration so neither side can observe the other's half-transition.
    let _resume_admission =
        crate::node_agent_supervision_worktree_lease::ResumeAdmissionGuard::acquire(base)?;
    let workspace_occupied = workspace_has_live_or_nonterminal_occupant(
        runtime,
        task_id,
        Path::new(&task.workspace_path),
    )
    .await?;
    if !terminal_release_eligible(
        &task.status,
        cancel_side_effect_committed,
        workspace_occupied,
    ) {
        return Ok(false);
    }

    let released = release_record_lease(&task, &contract, task_id, cancel_side_effect_committed)?;
    if released {
        let root_task_id = supervision_root(&contract, task_id);
        record_supervision_event(
            &runtime.task_journal,
            task_id,
            "supervision_worktree_lease_released",
            json!({
                "root_task_id": root_task_id,
                "terminal_status": task.status,
                "trigger": "terminal_reconcile",
            }),
        )?;
        info!(%task_id, %root_task_id, "released terminal supervision worktree lease");
    }
    Ok(released)
}

async fn workspace_has_live_or_nonterminal_occupant(
    runtime: &NodeRuntime,
    candidate_task_id: &str,
    active: &Path,
) -> Result<bool> {
    if runtime
        .active_cli_prompts
        .views_without_approvals()
        .await
        .iter()
        .filter_map(|prompt| prompt.cwd.as_deref())
        .any(|cwd| workspace_contains(active, Path::new(cwd)))
    {
        return Ok(true);
    }

    for occupant in runtime.local_tasks.list_identity_candidates()? {
        if !crate::node_agent_update_checkpoint::same_path(
            active,
            Path::new(&occupant.workspace_path),
        ) {
            continue;
        }
        if durable_nonterminal_occupant_blocks_release(
            candidate_task_id,
            &occupant.task_id,
            &occupant.status,
        ) {
            return Ok(true);
        }
        if runtime
            .cli_sidecars
            .session_for_task(&occupant.task_id)?
            .is_some_and(|session| session.is_live_at(crate::node_agent_cli_sidecar::now_ms()))
        {
            return Ok(true);
        }
    }
    Ok(false)
}

fn durable_nonterminal_occupant_blocks_release(
    candidate_task_id: &str,
    occupant_task_id: &str,
    occupant_status: &str,
) -> bool {
    occupant_task_id != candidate_task_id && !durable_terminal_status(occupant_status)
}

fn workspace_contains(workspace: &Path, candidate: &Path) -> bool {
    let workspace = crate::node_agent_workspace_match::canonical_or_original(workspace);
    let candidate = crate::node_agent_workspace_match::canonical_or_original(candidate);
    candidate.starts_with(workspace)
}

fn durable_terminal_status(status: &str) -> bool {
    matches!(
        status.trim().to_ascii_lowercase().as_str(),
        "done" | "failed" | "canceled" | "cancelled" | "finished"
    )
}

pub(crate) async fn reconcile_all(runtime: &NodeRuntime) -> Result<usize> {
    let mut released = 0;
    for task in runtime.local_tasks.list_terminal_lease_candidates()? {
        match reconcile_task(runtime, &task.task_id).await {
            Ok(true) => released += 1,
            Ok(false) => {}
            Err(error) => warn!(
                task_id = %task.task_id,
                %error,
                "terminal supervision lease was preserved because reconciliation was not provably safe"
            ),
        }
    }
    Ok(released)
}

pub(crate) fn spawn_reconciler(runtime: Arc<NodeRuntime>) {
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_secs(30));
        interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
        // Startup performs an explicit pass after recovery. Avoid duplicating it
        // on the interval's immediate first tick.
        interval.tick().await;
        loop {
            interval.tick().await;
            if let Err(error) = reconcile_all(&runtime).await {
                warn!(%error, "periodic terminal supervision lease reconciliation failed");
            }
        }
    });
}

fn release_record_lease(
    task: &LocalTaskRecord,
    contract: &SupervisionContract,
    task_id: &str,
    cancel_side_effect_committed: bool,
) -> Result<bool> {
    let status = task
        .workspace_status
        .as_ref()
        .context("supervised terminal task is missing durable workspace identity")?;
    anyhow::ensure!(
        status
            .get("platform_provenance")
            .and_then(serde_json::Value::as_str)
            == Some("elon.conversation_worktree.v1"),
        "terminal task workspace is not a platform supervision worktree"
    );
    let base = required_path(status, "base_workspace_path")?;
    let active = required_path(status, "active_workspace_path")?;
    anyhow::ensure!(
        crate::node_agent_update_checkpoint::same_path(Path::new(&task.workspace_path), active),
        "terminal task active workspace identity drifted"
    );
    let root_task_id = supervision_root(contract, task_id);
    if let Some(recorded_root) = status
        .get("root_task_id")
        .and_then(serde_json::Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        anyhow::ensure!(
            recorded_root == root_task_id,
            "terminal task supervision root identity drifted"
        );
    }
    release_if_eligible(
        base,
        active,
        root_task_id,
        &task.status,
        cancel_side_effect_committed,
        false,
    )
}

fn required_path<'a>(value: &'a serde_json::Value, field: &str) -> Result<&'a Path> {
    value
        .get(field)
        .and_then(serde_json::Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(Path::new)
        .with_context(|| format!("supervised workspace identity is missing {field}"))
}

fn supervision_root<'a>(contract: &'a SupervisionContract, task_id: &'a str) -> &'a str {
    contract
        .root_task_id
        .as_deref()
        .or(contract.parent_task_id.as_deref())
        .unwrap_or(task_id)
}

fn terminal_release_eligible(
    status: &str,
    cancel_side_effect_committed: bool,
    execution_active: bool,
) -> bool {
    if execution_active {
        return false;
    }
    durable_terminal_status(status)
        || (status.eq_ignore_ascii_case("cancel_requested") && cancel_side_effect_committed)
}

fn release_if_eligible(
    base: &Path,
    active: &Path,
    root_task_id: &str,
    status: &str,
    cancel_side_effect_committed: bool,
    execution_active: bool,
) -> Result<bool> {
    if !terminal_release_eligible(status, cancel_side_effect_committed, execution_active) {
        return Ok(false);
    }
    anyhow::ensure!(base.is_dir(), "supervision base workspace is unavailable");
    anyhow::ensure!(
        !crate::node_agent_update_checkpoint::same_path(base, active),
        "refusing to unlock the shared base workspace"
    );
    let expected = crate::node_agent_supervision_worktree_lease::lease_reason(root_task_id)?;
    match crate::node_agent_supervision_worktree_lease::worktree_lock_reason(base, active)? {
        None => Ok(false),
        Some(actual) if actual != expected => {
            bail!("refusing to release non-matching worktree lease: {actual}")
        }
        Some(_) => {
            crate::node_agent_supervision_worktree_lease::release(base, active, root_task_id)?;
            Ok(true)
        }
    }
}

#[cfg(test)]
mod tests {
    use std::fs;

    use uuid::Uuid;

    use super::*;
    use crate::git_command_error::git_command;

    #[test]
    fn completed_terminal_status_releases_matching_root_lease() {
        let fixture = GitFixture::new();
        crate::node_agent_supervision_worktree_lease::acquire(
            &fixture.base,
            &fixture.active,
            "root-terminal",
        )
        .unwrap();

        assert!(release_if_eligible(
            &fixture.base,
            &fixture.active,
            "root-terminal",
            "failed",
            false,
            false,
        )
        .unwrap());
        assert_eq!(
            crate::node_agent_supervision_worktree_lease::worktree_lock_reason(
                &fixture.base,
                &fixture.active,
            )
            .unwrap(),
            None
        );
        assert!(!release_if_eligible(
            &fixture.base,
            &fixture.active,
            "root-terminal",
            "failed",
            false,
            false,
        )
        .unwrap());
    }

    #[test]
    fn running_or_live_cancel_task_keeps_matching_lease() {
        let fixture = GitFixture::new();
        crate::node_agent_supervision_worktree_lease::acquire(
            &fixture.base,
            &fixture.active,
            "root-running",
        )
        .unwrap();

        assert!(!release_if_eligible(
            &fixture.base,
            &fixture.active,
            "root-running",
            "running",
            false,
            false,
        )
        .unwrap());
        assert!(!release_if_eligible(
            &fixture.base,
            &fixture.active,
            "root-running",
            "cancel_requested",
            true,
            true,
        )
        .unwrap());
        assert_eq!(
            crate::node_agent_supervision_worktree_lease::worktree_lock_reason(
                &fixture.base,
                &fixture.active,
            )
            .unwrap()
            .as_deref(),
            Some("elon-supervision:root-running")
        );
    }

    #[test]
    fn committed_cancel_without_live_executor_releases_lease() {
        let fixture = GitFixture::new();
        crate::node_agent_supervision_worktree_lease::acquire(
            &fixture.base,
            &fixture.active,
            "root-cancel",
        )
        .unwrap();
        assert!(release_if_eligible(
            &fixture.base,
            &fixture.active,
            "root-cancel",
            "cancel_requested",
            true,
            false,
        )
        .unwrap());
    }

    #[test]
    fn mismatched_or_non_supervision_lease_is_never_released() {
        let fixture = GitFixture::new();
        git(
            &fixture.base,
            &[
                "worktree",
                "lock",
                "--reason",
                "foreign-owner",
                &path_arg(&fixture.active),
            ],
        );
        assert!(release_if_eligible(
            &fixture.base,
            &fixture.active,
            "root-expected",
            "done",
            false,
            false,
        )
        .is_err());
        assert_eq!(
            crate::node_agent_supervision_worktree_lease::worktree_lock_reason(
                &fixture.base,
                &fixture.active,
            )
            .unwrap()
            .as_deref(),
            Some("foreign-owner")
        );
    }

    #[test]
    fn running_descendant_on_same_workspace_blocks_parent_release() {
        let fixture = GitFixture::new();
        assert!(durable_nonterminal_occupant_blocks_release(
            "terminal-parent",
            "running-child",
            "running",
        ));
        assert!(!durable_nonterminal_occupant_blocks_release(
            "terminal-parent",
            "finished-sibling",
            "done",
        ));
        assert!(!durable_nonterminal_occupant_blocks_release(
            "terminal-parent",
            "terminal-parent",
            "running",
        ));
        fs::create_dir_all(fixture.active.join("server")).unwrap();
        assert!(workspace_contains(
            &fixture.active,
            &fixture.active.join("server")
        ));
        assert!(!workspace_contains(&fixture.active, &fixture.base));
    }

    struct GitFixture {
        root: std::path::PathBuf,
        base: std::path::PathBuf,
        active: std::path::PathBuf,
    }

    impl GitFixture {
        fn new() -> Self {
            let root = std::env::temp_dir().join(format!(
                "elon-terminal-supervision-lease-{}",
                Uuid::new_v4().simple()
            ));
            let base = root.join("base");
            let active = root.join("active");
            fs::create_dir_all(&base).unwrap();
            git(&base, &["init"]);
            git(&base, &["config", "user.email", "ai@example.test"]);
            git(&base, &["config", "user.name", "AI Test"]);
            fs::write(base.join("README.md"), "seed\n").unwrap();
            git(&base, &["add", "README.md"]);
            git(&base, &["commit", "-m", "seed"]);
            git(
                &base,
                &[
                    "worktree",
                    "add",
                    "-b",
                    "ai/session/project/terminal",
                    &path_arg(&active),
                    "HEAD",
                ],
            );
            Self { root, base, active }
        }
    }

    impl Drop for GitFixture {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.root);
        }
    }

    fn git(cwd: &Path, args: &[&str]) {
        let output = git_command().args(args).current_dir(cwd).output().unwrap();
        assert!(
            output.status.success(),
            "git {args:?} failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    fn path_arg(path: &Path) -> String {
        path.to_string_lossy().to_string()
    }
}
