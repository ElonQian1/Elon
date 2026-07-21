//! Terminal-state reconciliation for Desktop-supervised worktree leases.
//!
//! Review remains an independent verdict. Execution ownership ends once the
//! durable local task is terminal, so its exact root-bound Git lease must not
//! remain held while a reviewer is unavailable. Dirty worktrees are never
//! removed here; normal cleanup keeps preserving unknown changes.

use std::{future::Future, path::Path, sync::Arc, time::Duration};

use anyhow::{bail, Context, Result};
use serde_json::json;
use tracing::{info, warn};

use crate::{
    node_agent_local_task_supervision::{load_supervision_contract, record_supervision_event},
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
    let Some(identity) =
        crate::node_agent_supervision_terminal_lease_safety::verify_release_identity(
            runtime, &task, &contract, task_id,
        )
        .await?
    else {
        return Ok(false);
    };
    let released = release_if_eligible(
        &identity.base,
        &identity.active,
        &identity.root_task_id,
        &task.status,
        cancel_side_effect_committed,
        false,
    )?;
    if released {
        let root_task_id = identity.root_task_id.as_str();
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
        if durable_occupant_blocks_release(candidate_task_id, &occupant.task_id, &occupant.status) {
            return Ok(true);
        }
    }
    Ok(false)
}

fn durable_occupant_blocks_release(
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

fn required_path<'a>(value: &'a serde_json::Value, field: &str) -> Result<&'a Path> {
    value
        .get(field)
        .and_then(serde_json::Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(Path::new)
        .with_context(|| format!("supervised workspace identity is missing {field}"))
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
    spawn_reconciler_loop(Duration::from_secs(30), move || {
        let runtime = runtime.clone();
        async move {
            if let Err(error) = reconcile_all(&runtime).await {
                warn!(%error, "periodic terminal supervision lease reconciliation failed");
            }
        }
    });
}

fn spawn_reconciler_loop<F, Fut>(
    interval: Duration,
    mut reconcile: F,
) -> tokio::task::JoinHandle<()>
where
    F: FnMut() -> Fut + Send + 'static,
    Fut: Future<Output = ()> + Send + 'static,
{
    tokio::spawn(async move {
        loop {
            reconcile().await;
            tokio::time::sleep(interval).await;
        }
    })
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
    fn only_nonterminal_descendants_block_parent_release() {
        let fixture = GitFixture::new();
        assert!(durable_occupant_blocks_release(
            "terminal-parent",
            "running-child",
            "running",
        ));
        assert!(!durable_occupant_blocks_release(
            "terminal-parent",
            "finished-sibling",
            "done",
        ));
        assert!(!durable_occupant_blocks_release(
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

    #[tokio::test]
    async fn slow_initial_reconcile_stays_background_and_retries() {
        use std::sync::atomic::{AtomicUsize, Ordering};
        use tokio::sync::Notify;

        let attempts = Arc::new(AtomicUsize::new(0));
        let started = Arc::new(Notify::new());
        let release = Arc::new(Notify::new());
        let handle = spawn_reconciler_loop(Duration::from_millis(10), {
            let attempts = attempts.clone();
            let started = started.clone();
            let release = release.clone();
            move || {
                let attempts = attempts.clone();
                let started = started.clone();
                let release = release.clone();
                async move {
                    if attempts.fetch_add(1, Ordering::SeqCst) == 0 {
                        started.notify_one();
                        release.notified().await;
                    }
                }
            }
        });

        tokio::time::timeout(Duration::from_millis(100), started.notified())
            .await
            .expect("startup reconciliation should begin immediately in the background");
        assert!(
            !handle.is_finished(),
            "a slow pass must not block its caller"
        );
        release.notify_one();
        tokio::time::timeout(Duration::from_millis(250), async {
            while attempts.load(Ordering::SeqCst) < 2 {
                tokio::task::yield_now().await;
            }
        })
        .await
        .expect("the reconciler should retry after the initial slow pass");
        handle.abort();
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
