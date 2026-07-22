//! Periodic reconciliation for local tasks whose executor ownership vanished.
//!
//! A pending completion is a retry log, not proof that all terminal stores are
//! coherent. Each pass first replays the existing durable terminal reconciler;
//! only a live PID, sidecar, current handle, or fresh journal heartbeat may keep
//! a still-nonterminal row in `running` after that replay fails.

use std::{collections::HashSet, path::Path, sync::Arc, time::Duration};

use anyhow::{Context, Result};
use tracing::{info, warn};

use crate::NodeRuntime;

const STALE_AFTER_MS: u128 = 2 * 60 * 1_000;
const RECONCILE_INTERVAL: Duration = Duration::from_secs(30);
const COMPLETION_SCAN_LIMIT: usize = 1_000;
const TERMINAL_REPAIR_LIMIT: usize = 128;
const STALE_CANDIDATE_LIMIT: usize = 256;

pub(crate) async fn reconcile_once(runtime: &NodeRuntime) -> Result<usize> {
    reconcile_with_stale_after(runtime, STALE_AFTER_MS).await
}

async fn reconcile_with_stale_after(runtime: &NodeRuntime, stale_after_ms: u128) -> Result<usize> {
    repair_historical_terminal_candidates(runtime).await?;
    let pending = runtime
        .completion_outbox
        .list_pending(COMPLETION_SCAN_LIMIT)?
        .into_iter()
        .filter(|completion| {
            completion.origin == crate::node_agent_completion_outbox::LOCAL_OFFLINE_ORIGIN
        })
        .collect::<Vec<_>>();
    let observed_events = pending
        .iter()
        .map(|completion| completion.event_id.clone())
        .collect::<HashSet<_>>();
    for completion in pending {
        match crate::node_agent_local_terminal_reconcile::LocalTerminalReconciler::from_runtime(
            runtime,
        )
        .reconcile(&completion)
        .await
        {
            Ok(()) => info!(
                req_id = %completion.req_id,
                event_id = %completion.event_id,
                "reconciled trusted local terminal state from durable outbox"
            ),
            Err(error) => warn!(
                req_id = %completion.req_id,
                event_id = %completion.event_id,
                %error,
                "durable local terminal reconciliation remains retryable"
            ),
        }
    }

    let now = crate::node_agent_cli_sidecar::now_ms();
    let cutoff = (now.min(i64::MAX as u128) as i64)
        .saturating_sub(stale_after_ms.min(i64::MAX as u128) as i64);
    let candidates = runtime
        .local_tasks
        .list_stale_runtime_candidates(cutoff, STALE_CANDIDATE_LIMIT)?;
    let mut changed = 0;
    for candidate in candidates.into_iter().filter(|task| {
        matches!(
            task.status.as_str(),
            "running" | "recovering" | "reattaching"
        ) && task.completion_event_id.is_none()
    }) {
        changed += usize::from(
            reconcile_candidate(
                runtime,
                &candidate.task_id,
                now,
                stale_after_ms,
                cutoff,
                &observed_events,
            )
            .await?,
        );
    }
    Ok(changed)
}

async fn repair_historical_terminal_candidates(runtime: &NodeRuntime) -> Result<usize> {
    let candidates = runtime
        .local_tasks
        .list_terminal_repair_candidates(TERMINAL_REPAIR_LIMIT)?;
    let mut repaired = 0;
    for candidate in candidates.into_iter().filter(is_platform_supervised) {
        let snapshot_trusted = candidate.workspace_status.as_ref().is_some_and(|status| {
            status
                .get("terminal_snapshot_status")
                .and_then(serde_json::Value::as_str)
                == Some("trusted")
        });
        let existing = runtime
            .completion_outbox
            .latest_for_req_id(&candidate.task_id)?;
        if snapshot_trusted && existing.is_some() {
            continue;
        }
        let contract = crate::node_agent_local_task_supervision::load_supervision_contract(
            &runtime.task_journal,
            &candidate.task_id,
        )?
        .context("supervised historical terminal task has no durable contract")?;
        let base = crate::node_agent_supervision_terminal_lease_safety::admission_base(
            &candidate,
            &contract,
            &candidate.task_id,
        )?;
        let admission =
            crate::node_agent_supervision_worktree_lease::ResumeAdmissionGuard::acquire(&base)?;
        let task = runtime
            .local_tasks
            .get(&candidate.task_id)?
            .context("historical terminal task disappeared after admission")?;
        if task.status != "done"
            || task.sync_state == "synced"
            || task.completion_event_id.is_none()
        {
            continue;
        }
        let active = task
            .workspace_status
            .as_ref()
            .and_then(|status| status.get("active_workspace_path"))
            .and_then(serde_json::Value::as_str)
            .map(Path::new)
            .unwrap_or_else(|| Path::new(&task.workspace_path));
        if candidate_runtime_protects(
            runtime,
            &task,
            active,
            crate::node_agent_cli_sidecar::now_ms(),
            0,
        )
        .await?
        {
            continue;
        }
        let completion = match runtime.completion_outbox.latest_for_req_id(&task.task_id)? {
            Some(completion) => completion,
            None => {
                crate::node_agent_terminal_finalization::historical_completion(&task, &contract)?
                    .context("historical terminal task has neither outbox nor completed receipt")?
            }
        };
        runtime
            .completion_outbox
            .preflight_restore_pending(&completion)?;
        crate::node_agent_local_terminal_reconcile::LocalTerminalReconciler::from_runtime(runtime)
            .reconcile_with_admission(&completion, &admission)
            .await?;
        runtime
            .local_tasks
            .mark_trusted_completion_pending(&task.task_id, &completion.event_id)?;
        runtime.completion_outbox.enqueue(&completion)?;
        runtime
            .completion_outbox
            .restore_pending(&completion.event_id, &completion.req_id)?;
        repaired += 1;
        info!(
            task_id = %task.task_id,
            event_id = %completion.event_id,
            "recovered historical terminal completion from strict durable evidence"
        );
    }
    Ok(repaired)
}

async fn reconcile_candidate(
    runtime: &NodeRuntime,
    task_id: &str,
    now: u128,
    stale_after_ms: u128,
    cutoff: i64,
    observed_events: &HashSet<String>,
) -> Result<bool> {
    let initial = runtime
        .local_tasks
        .get(task_id)?
        .context("orphan candidate disappeared before admission")?;
    let contract = crate::node_agent_local_task_supervision::load_supervision_contract(
        &runtime.task_journal,
        task_id,
    )?;
    let admission_base = if is_platform_supervised(&initial) {
        let contract = contract
            .as_ref()
            .context("supervised orphan candidate has no durable contract")?;
        Some(
            crate::node_agent_supervision_terminal_lease_safety::admission_base(
                &initial, contract, task_id,
            )?,
        )
    } else {
        None
    };
    let admission = admission_base
        .as_deref()
        .map(crate::node_agent_supervision_worktree_lease::ResumeAdmissionGuard::acquire)
        .transpose()?;

    // Every ownership probe is repeated under the same cross-process admission
    // guard used by Resume and supervised handle registration. The database
    // transition follows immediately, so a pre-guard inventory can never be
    // used as authority for resume_required.
    let task = runtime
        .local_tasks
        .get(task_id)?
        .context("orphan candidate disappeared after admission")?;
    if !matches!(
        task.status.as_str(),
        "running" | "recovering" | "reattaching"
    ) || task.completion_event_id.is_some()
        || task.started_at_ms > cutoff
    {
        return Ok(false);
    }
    if runtime
        .completion_outbox
        .latest_for_req_id(task_id)?
        .is_some_and(|completion| !observed_events.contains(&completion.event_id))
    {
        // The pass-wide replay may have raced with this just-arrived receipt.
        // Preserve the row; the next bounded pass will reconcile it through the
        // normal terminal trust boundary.
        return Ok(false);
    }

    let active_workspace = task
        .workspace_status
        .as_ref()
        .and_then(|status| status.get("active_workspace_path"))
        .and_then(serde_json::Value::as_str)
        .map(Path::new)
        .unwrap_or_else(|| Path::new(&task.workspace_path));
    if candidate_runtime_protects(runtime, &task, active_workspace, now, stale_after_ms).await? {
        return Ok(false);
    }
    if is_platform_supervised(&task) {
        let contract = crate::node_agent_local_task_supervision::load_supervision_contract(
            &runtime.task_journal,
            task_id,
        )?
        .context("supervised orphan contract disappeared under admission")?;
        if !active_workspace.is_dir() {
            if let Some(completion) =
                crate::node_agent_terminal_finalization::historical_completion(&task, &contract)?
            {
                let admission = admission
                    .as_ref()
                    .context("supervised orphan recovery lost its admission guard")?;
                runtime
                    .completion_outbox
                    .preflight_restore_pending(&completion)?;
                crate::node_agent_local_terminal_reconcile::LocalTerminalReconciler::from_runtime(
                    runtime,
                )
                .reconcile_with_admission(&completion, admission)
                .await?;
                runtime
                    .local_tasks
                    .mark_trusted_completion_pending(task_id, &completion.event_id)?;
                runtime.completion_outbox.enqueue(&completion)?;
                runtime
                    .completion_outbox
                    .restore_pending(&completion.event_id, &completion.req_id)?;
                info!(
                    task_id,
                    event_id = %completion.event_id,
                    "recovered stale missing-worktree task from completed finalization receipt"
                );
                return Ok(true);
            }
        }
    }
    runtime
        .local_tasks
        .mark_one_stale_without_runtime(task_id, cutoff)
}

async fn candidate_runtime_protects(
    runtime: &NodeRuntime,
    task: &crate::node_agent_local_task_store::LocalTaskRecord,
    active_workspace: &Path,
    now: u128,
    stale_after_ms: u128,
) -> Result<bool> {
    if runtime
        .active_cli_prompt_views_for_workspace(active_workspace)
        .await
        .into_iter()
        .any(|handle| handle.control_handle_live)
        || runtime
            .active_cli_prompt_view(&task.task_id)
            .await
            .is_some_and(|handle| handle.control_handle_live)
    {
        return Ok(true);
    }
    if is_platform_supervised(task) {
        let contract = crate::node_agent_local_task_supervision::load_supervision_contract(
            &runtime.task_journal,
            &task.task_id,
        )?
        .context("supervised orphan contract disappeared under admission")?;
        let root = contract.root_task_id.as_deref().unwrap_or(&task.task_id);
        if crate::node_agent_supervision_terminal_lease_safety::lineage_or_workspace_is_active(
            runtime,
            &task.task_id,
            root,
            active_workspace,
            false,
        )
        .await?
        {
            return Ok(true);
        }
    }
    if let Some(sidecar) = runtime.cli_sidecars.session_for_task(&task.task_id)? {
        if sidecar_record_protects(&sidecar, now)? {
            return Ok(true);
        }
    }
    if let Some(record) = runtime.task_journal.snapshot(&task.task_id, 0, 1)?.record {
        if journal_record_protects(&record, now, stale_after_ms)? {
            return Ok(true);
        }
    }
    Ok(false)
}

fn journal_record_protects(
    record: &crate::node_agent_task_journal::TaskJournalRecord,
    now: u128,
    stale_after_ms: u128,
) -> Result<bool> {
    if !matches!(
        record.status.as_str(),
        "running" | "recovering" | "reattaching"
    ) {
        return Ok(false);
    }
    let heartbeat = record.heartbeat_at_ms.unwrap_or(record.updated_at_ms);
    anyhow::ensure!(heartbeat <= now, "journal heartbeat is in the future");
    if now.saturating_sub(heartbeat) <= stale_after_ms {
        return Ok(true);
    }
    recorded_process_is_live(record)
}

pub(crate) fn recorded_process_is_live(
    record: &crate::node_agent_task_journal::TaskJournalRecord,
) -> Result<bool> {
    let Some(pid) = record.os_pid else {
        return Ok(false);
    };
    let running = crate::node_agent_cli_worker::process_is_running(pid);
    let current_identity = crate::node_agent_cli_worker::process_identity(pid);
    match (
        running,
        record.process_identity.as_deref(),
        current_identity,
    ) {
        (false, _, _) => Ok(false),
        (true, Some(expected), Some(actual)) => Ok(expected == actual),
        (true, _, _) => anyhow::bail!("live journal PID has no verifiable process identity"),
    }
}

fn sidecar_record_protects(
    session: &crate::node_agent_cli_sidecar::CliSidecarSessionRecord,
    now: u128,
) -> Result<bool> {
    anyhow::ensure!(
        session.last_seen_at_ms <= now && session.started_at_ms <= now,
        "sidecar heartbeat is in the future"
    );
    if session.is_terminal() {
        return Ok(false);
    }
    if session.protects_startup_reconcile_at(now) {
        return Ok(true);
    }
    for (pid, identity) in [
        (
            session.sidecar_pid,
            session.sidecar_process_identity.as_deref(),
        ),
        (session.child_pid, session.child_process_identity.as_deref()),
    ] {
        let Some(pid) = pid else { continue };
        if !crate::node_agent_cli_worker::process_is_running(pid) {
            continue;
        }
        let current = crate::node_agent_cli_worker::process_identity(pid);
        match (identity, current) {
            (Some(expected), Some(actual)) if expected == actual => return Ok(true),
            (Some(_), Some(_)) => continue,
            _ => anyhow::bail!("live sidecar PID has no verifiable process identity"),
        }
    }
    Ok(false)
}

fn is_platform_supervised(task: &crate::node_agent_local_task_store::LocalTaskRecord) -> bool {
    task.workspace_status
        .as_ref()
        .and_then(|status| status.get("platform_provenance"))
        .and_then(serde_json::Value::as_str)
        == Some("elon.conversation_worktree.v1")
}

pub(crate) fn spawn_reconciler(runtime: Arc<NodeRuntime>) {
    let runtime_handle = tokio::runtime::Handle::current();
    std::thread::Builder::new()
        .name("elon-local-task-orphan-reconcile".to_string())
        .spawn(move || loop {
            std::thread::sleep(RECONCILE_INTERVAL);
            runtime_handle.block_on(async {
                match reconcile_once(&runtime).await {
                    Ok(0) => {}
                    Ok(changed) => {
                        warn!(
                            changed,
                            "periodic orphan reconciliation changed durable task state"
                        )
                    }
                    Err(error) => warn!(%error, "periodic orphan reconciliation failed closed"),
                }
            });
        })
        .expect("local task orphan reconciler thread should start");
}

#[cfg(test)]
mod tests {
    use std::{fs, path::Path};

    use homecli_proto::{CliCompletionEnvelope, CliCompletionProducerIdentity, CliProjectContext};

    use super::*;
    use crate::node_agent_local_task_store::{LocalTaskStart, LocalTaskStore};

    #[tokio::test]
    async fn complete_evidence_finishes_but_incomplete_outbox_becomes_resume_required() {
        let root = std::env::temp_dir().join(format!(
            "elon-orphan-reconcile-{}",
            uuid::Uuid::new_v4().simple()
        ));
        let runtime = test_runtime(&root);
        for task_id in ["complete", "incomplete"] {
            runtime
                .local_tasks
                .create(LocalTaskStart {
                    task_id,
                    owner_user_id: "owner",
                    agent_id: "agent",
                    install_id: "install",
                    project_id: "project",
                    channel_id: None,
                    conversation_id: task_id,
                    workspace_path: root.to_str().unwrap(),
                    prompt: "work",
                    cli: "codex",
                    runtime_permission: "full_access",
                })
                .unwrap();
        }
        let complete = completion("complete", "event-complete", "owner");
        let incomplete = completion("incomplete", "event-incomplete", "wrong-owner");
        runtime.completion_outbox.enqueue(&complete).unwrap();
        runtime.completion_outbox.enqueue(&incomplete).unwrap();

        assert_eq!(reconcile_with_stale_after(&runtime, 0).await.unwrap(), 1);
        assert_eq!(
            runtime.local_tasks.get("complete").unwrap().unwrap().status,
            "done"
        );
        assert_eq!(
            runtime
                .local_tasks
                .get("incomplete")
                .unwrap()
                .unwrap()
                .status,
            "resume_required"
        );
        assert_eq!(runtime.completion_outbox.pending_count().unwrap(), 2);
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn live_pid_or_fresh_heartbeat_blocks_orphan_conversion() {
        let now = crate::node_agent_cli_sidecar::now_ms();
        let mut record = crate::node_agent_task_journal::TaskJournalRecord {
            req_id: "live".into(),
            cli_name: "codex".into(),
            route: None,
            run_handle_id: None,
            cwd: None,
            runtime_permission: None,
            os_pid: Some(std::process::id()),
            process_started_at_ms: Some(1),
            process_identity: crate::node_agent_cli_worker::process_identity(std::process::id()),
            codex_session_id: None,
            codex_session_scope_key: None,
            codex_session_updated_at_ms: None,
            status: "running".into(),
            phase: "reasoning".into(),
            current_command: None,
            last_progress_ms: None,
            heartbeat_at_ms: Some(0),
            timeout_policy: None,
            dispatch: None,
            started_at_ms: 0,
            updated_at_ms: 0,
            cancel_requested_at_ms: None,
            cancel_intent: None,
        };
        assert!(journal_record_protects(&record, now, 0).unwrap());
        record.os_pid = None;
        record.process_identity = None;
        record.heartbeat_at_ms = Some(now);
        assert!(journal_record_protects(&record, now, 1).unwrap());
        record.heartbeat_at_ms = Some(0);
        assert!(!journal_record_protects(&record, now, 1).unwrap());
    }

    #[test]
    fn pid_reuse_does_not_protect_but_future_heartbeat_fails_closed() {
        let now = crate::node_agent_cli_sidecar::now_ms();
        let mut record = crate::node_agent_task_journal::TaskJournalRecord {
            req_id: "pid-reuse".into(),
            cli_name: "codex".into(),
            route: None,
            run_handle_id: None,
            cwd: None,
            runtime_permission: None,
            os_pid: Some(std::process::id()),
            process_started_at_ms: Some(1),
            process_identity: Some("reused-pid-with-another-start-time".into()),
            codex_session_id: None,
            codex_session_scope_key: None,
            codex_session_updated_at_ms: None,
            status: "running".into(),
            phase: "reasoning".into(),
            current_command: None,
            last_progress_ms: None,
            heartbeat_at_ms: Some(0),
            timeout_policy: None,
            dispatch: None,
            started_at_ms: 0,
            updated_at_ms: 0,
            cancel_requested_at_ms: None,
            cancel_intent: None,
        };
        assert!(!journal_record_protects(&record, now, 0).unwrap());
        record.os_pid = None;
        record.process_identity = None;
        record.heartbeat_at_ms = Some(now.saturating_add(1));
        assert!(journal_record_protects(&record, now, 0).is_err());
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn handle_inserted_through_admission_blocks_resume_required_race() {
        let root = std::env::temp_dir().join(format!(
            "elon-orphan-owner-race-{}",
            uuid::Uuid::new_v4().simple()
        ));
        let base = root.join("base");
        let active = root.join("conversation-worktrees/project/conversation");
        fs::create_dir_all(active.parent().unwrap()).unwrap();
        git(&root, &["init", "-b", "main", base.to_str().unwrap()]);
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
                "ai/session/project/conversation",
                active.to_str().unwrap(),
            ],
        );
        let base = base.canonicalize().unwrap();
        let active = active.canonicalize().unwrap();
        let runtime = test_runtime(&root);
        runtime
            .local_tasks
            .create(LocalTaskStart {
                task_id: "owner-race",
                owner_user_id: "owner",
                agent_id: "agent",
                install_id: "install",
                project_id: "project",
                channel_id: None,
                conversation_id: "conversation",
                workspace_path: active.to_str().unwrap(),
                prompt: "work",
                cli: "codex",
                runtime_permission: "full_access",
            })
            .unwrap();
        let common = git_output(
            &active,
            &["rev-parse", "--path-format=absolute", "--git-common-dir"],
        );
        runtime
            .local_tasks
            .record_initial_workspace_status(
                "owner-race",
                &serde_json::json!({
                    "platform_provenance":"elon.conversation_worktree.v1",
                    "root_task_id":"owner-race", "active_workspace_path":active,
                    "base_workspace_path":base, "project_id":"project", "isolated":true,
                    "branch":"ai/session/project/conversation", "git_common_dir":common,
                    "git_remote":"unused"
                }),
            )
            .unwrap();
        crate::node_agent_local_task_supervision::record_supervision_event(
            &runtime.task_journal,
            "owner-race",
            "supervision_contract",
            crate::node_agent_local_task_supervision::contract_payload(
                &crate::node_agent_local_task_supervision::SupervisionContract {
                    protocol: crate::node_agent_local_task_supervision::SUPERVISION_PROTOCOL.into(),
                    supervisor: "codex_desktop".into(),
                    task_role: "requirement".into(),
                    parent_task_id: None,
                    root_task_id: Some("owner-race".into()),
                    acceptance_criteria: vec![],
                    improvement_policy: "after_task_only".into(),
                },
            ),
        )
        .unwrap();
        let runtime = Arc::new(runtime);
        let admission =
            crate::node_agent_supervision_worktree_lease::ResumeAdmissionGuard::acquire(&base)
                .unwrap();
        let sweep_runtime = runtime.clone();
        let sweep =
            tokio::spawn(
                async move { reconcile_with_stale_after(&sweep_runtime, 0).await.unwrap() },
            );
        tokio::time::sleep(Duration::from_millis(100)).await;
        let (cancel_tx, _cancel_rx) = tokio::sync::watch::channel(false);
        let handle = crate::node_agent_active_task::ActiveCliPromptHandle::new(
            "owner-race",
            "codex",
            "route_a_external_cli",
            Some(active.to_string_lossy().to_string()),
            Some("full_access".into()),
            cancel_tx,
        )
        .with_exclusive_workspace(true);
        assert_eq!(
            runtime
                .try_register_supervised_cli_prompt(handle, Some(&admission))
                .await
                .unwrap(),
            crate::node_agent_active_task_registry::CliPromptRegistration::Inserted
        );
        drop(admission);
        assert_eq!(sweep.await.unwrap(), 0);
        assert_eq!(
            runtime
                .local_tasks
                .get("owner-race")
                .unwrap()
                .unwrap()
                .status,
            "running"
        );
        drop(runtime);
        let _ = fs::remove_dir_all(root);
    }

    fn test_runtime(root: &Path) -> NodeRuntime {
        let mut runtime = NodeRuntime::new(
            crate::node_agent_config::NodeConfig {
                cloud_url: "ws://127.0.0.1".into(),
                cloud_http_url: "http://127.0.0.1".into(),
                ollama_url: "http://127.0.0.1".into(),
                lm_studio_url: None,
                custom_url: None,
                price_per_1k: 0.0,
            },
            Some(crate::node_agent_config::Credentials {
                agent_id: "agent".into(),
                agent_secret: "unused".into(),
                owner_user_id: "owner".into(),
                user_token: None,
            }),
            crate::pc_storage_repo::StorageSettings::default(),
            crate::node_agent_data_root::resolve(None, None, None),
            "install".into(),
        );
        runtime.local_tasks = LocalTaskStore::new(root.join("tasks.sqlite3"));
        runtime.task_journal =
            crate::node_agent_task_journal::TaskJournal::new(root.join("journal"));
        runtime.completion_outbox = crate::node_agent_completion_outbox::CliCompletionOutbox::new(
            root.join("outbox.sqlite3"),
        );
        runtime.update_recovery =
            crate::node_agent_update_recovery::UpdateRecoveryStore::new(root.join("recovery.json"));
        runtime
    }

    fn completion(task_id: &str, event_id: &str, owner: &str) -> CliCompletionEnvelope {
        CliCompletionEnvelope {
            event_id: event_id.into(),
            req_id: task_id.into(),
            cli: "codex".into(),
            origin: crate::node_agent_completion_outbox::LOCAL_OFFLINE_ORIGIN.into(),
            producer_identity: Some(CliCompletionProducerIdentity {
                owner_user_id: owner.into(),
                agent_id: "agent".into(),
                install_id: "install".into(),
            }),
            project_context: Some(CliProjectContext {
                project_id: "project".into(),
                conversation_id: task_id.into(),
                runtime_permission: Some("full_access".into()),
            }),
            channel_id: None,
            prompt: None,
            final_output: "done".into(),
            exit_ok: true,
            error: None,
            session_id: None,
            prompt_tokens: None,
            cached_input_tokens: None,
            completion_tokens: None,
            reasoning_tokens: None,
            total_tokens: None,
            model: None,
            workspace_status: None,
            created_at_ms: 10,
        }
    }

    fn git(cwd: &Path, args: &[&str]) {
        let output = crate::git_command_error::git_command()
            .args(args)
            .current_dir(cwd)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "git {} failed: {}",
            args.join(" "),
            String::from_utf8_lossy(&output.stderr)
        );
    }

    fn git_output(cwd: &Path, args: &[&str]) -> String {
        let output = crate::git_command_error::git_command()
            .args(args)
            .current_dir(cwd)
            .output()
            .unwrap();
        assert!(output.status.success());
        String::from_utf8_lossy(&output.stdout).trim().to_string()
    }
}
