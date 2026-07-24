//! Periodic reconciliation for local tasks whose executor ownership vanished.
//!
//! A pending completion is a retry log, not proof that all terminal stores are
//! coherent. Each pass first replays the existing durable terminal reconciler;
//! only a live PID, sidecar, current handle, or fresh journal heartbeat may keep
//! a still-nonterminal row in `running` after that replay fails.

use std::{collections::HashSet, path::Path, sync::Arc, time::Duration};

use anyhow::{Context, Result};
use tracing::{info, warn};

use crate::node_agent_terminal_journal::has_finished_success;
use crate::NodeRuntime;

#[path = "node_agent_local_task_orphan_cancel.rs"]
mod cancel;
#[path = "node_agent_local_task_ghost_convergence.rs"]
mod ghost_convergence;
#[path = "node_agent_local_task_orphan_history_scan.rs"]
mod history_scan;
#[path = "node_agent_local_task_orphan_runtime_evidence.rs"]
mod runtime_evidence;
#[path = "node_agent_local_task_orphan_terminal_drift.rs"]
mod terminal_drift;

pub(crate) use ghost_convergence::receipt_conflict_is_audit_only;
pub(crate) use runtime_evidence::recorded_process_is_live;
use runtime_evidence::{exact_runtime_protects, journal_record_protects, sidecar_record_protects};

const STALE_AFTER_MS: u128 = 2 * 60 * 1_000;
const RECONCILE_INTERVAL: Duration = Duration::from_secs(30);
const COMPLETION_SCAN_LIMIT: usize = 1_000;
// A production node can retain hundreds of rejected historical completions.
// Repair remains opportunistic and fail-closed, but one pass must stay well
// below the watchdog health window before moving on to live orphan rows.
const TERMINAL_REPAIR_LIMIT: usize = 8;
const STALE_CANDIDATE_LIMIT: usize = 256;
const TERMINAL_JOURNAL_SYNC_LIMIT: usize = 1_000;

pub(crate) async fn reconcile_once(runtime: &NodeRuntime) -> Result<usize> {
    // Startup, the periodic worker, completion recovery, and the explicit
    // update gate can all request the same pass.  Running them concurrently
    // makes hundreds of per-repository admission checks contend with each
    // other and can turn a bounded audit into a multi-minute lock convoy.
    let _reconcile = runtime.local_task_reconcile.lock().await;
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
    let mut changed = terminal_drift::sync(runtime, now, TERMINAL_JOURNAL_SYNC_LIMIT).await?;
    let candidates = runtime
        .local_tasks
        .list_stale_runtime_candidates(cutoff, STALE_CANDIDATE_LIMIT)?
        .into_iter()
        .filter(|task| {
            matches!(
                task.status.as_str(),
                "running" | "recovering" | "reattaching" | "cancel_requested"
            ) && task.completion_event_id.is_none()
        })
        .collect::<Vec<_>>();
    let supervised_ids = candidates
        .iter()
        .filter(|task| is_platform_supervised(task))
        .map(|task| task.task_id.clone())
        .collect::<HashSet<_>>();
    let contracts = history_scan::contracts(&runtime.task_journal, &supervised_ids).await?;
    for candidate in candidates {
        let contract = contracts.get(&candidate.task_id).and_then(Option::as_ref);
        match reconcile_candidate(
            runtime,
            &candidate.task_id,
            contract,
            now,
            stale_after_ms,
            cutoff,
            &observed_events,
        )
        .await
        {
            Ok(reconciled) => changed += usize::from(reconciled),
            Err(error) => warn!(
                task_id = %candidate.task_id,
                %error,
                "orphan candidate remains fail-closed while later candidates continue"
            ),
        }
    }
    Ok(changed)
}

async fn repair_historical_terminal_candidates(runtime: &NodeRuntime) -> Result<usize> {
    let candidates = runtime
        .local_tasks
        .list_terminal_repair_candidates(TERMINAL_REPAIR_LIMIT)?
        .into_iter()
        .filter(is_platform_supervised)
        .collect::<Vec<_>>();
    let task_ids = candidates
        .iter()
        .map(|candidate| candidate.task_id.clone())
        .collect::<HashSet<_>>();
    let contracts = history_scan::contracts(&runtime.task_journal, &task_ids).await?;
    let mut repaired = 0;
    for candidate in candidates {
        let contract = contracts.get(&candidate.task_id).and_then(Option::as_ref);
        match repair_historical_terminal_candidate(runtime, &candidate, contract).await {
            Ok(changed) => repaired += usize::from(changed),
            Err(error) => warn!(
                task_id = %candidate.task_id,
                %error,
                "historical terminal repair remains fail-closed while later candidates continue"
            ),
        }
    }
    Ok(repaired)
}

async fn repair_historical_terminal_candidate(
    runtime: &NodeRuntime,
    candidate: &crate::node_agent_local_task_store::LocalTaskRecord,
    contract: Option<&crate::node_agent_local_task_supervision::SupervisionContract>,
) -> Result<bool> {
    let candidate_repairable_orphan = candidate.status == "resume_required"
        && candidate.completion_event_id.is_none()
        && crate::node_agent_local_task_store::is_orphan_runtime_resume_required_reason(
            candidate.error.as_deref(),
        )
        && has_finished_success(runtime, &candidate.task_id)?;
    let candidate_repairable_done =
        candidate.status == "done" && candidate.completion_event_id.is_some();
    if !candidate_repairable_done && !candidate_repairable_orphan {
        return Ok(false);
    }
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
        return Ok(false);
    }
    let contract =
        contract.context("supervised historical terminal task has no durable contract")?;
    let base = crate::node_agent_supervision_terminal_lease_safety::admission_base(
        candidate,
        contract,
        &candidate.task_id,
    )?;
    let Some(admission) =
        crate::node_agent_supervision_worktree_lease::ResumeAdmissionGuard::try_acquire(&base)?
    else {
        info!(
            task_id = %candidate.task_id,
            "historical terminal repair deferred because Resume admission is busy"
        );
        return Ok(false);
    };
    let task = runtime
        .local_tasks
        .get(&candidate.task_id)?
        .context("historical terminal task disappeared after admission")?;
    let repairable_done = task.status == "done" && task.completion_event_id.is_some();
    let repairable_orphan = task.status == "resume_required"
        && task.completion_event_id.is_none()
        && crate::node_agent_local_task_store::is_orphan_runtime_resume_required_reason(
            task.error.as_deref(),
        )
        && has_finished_success(runtime, &task.task_id)?;
    if (!repairable_done && !repairable_orphan) || task.sync_state == "synced" {
        return Ok(false);
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
        Some(contract),
        crate::node_agent_cli_sidecar::now_ms(),
        0,
    )
    .await?
    {
        return Ok(false);
    }
    let completion = match runtime.completion_outbox.latest_for_req_id(&task.task_id)? {
        Some(completion) => completion,
        None => crate::node_agent_terminal_finalization::historical_completion(&task, &contract)?
            .context("historical terminal task has neither outbox nor completed receipt")?,
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
    info!(
        task_id = %task.task_id,
        event_id = %completion.event_id,
        "recovered historical terminal completion from strict durable evidence"
    );
    Ok(true)
}

async fn reconcile_candidate(
    runtime: &NodeRuntime,
    task_id: &str,
    contract: Option<&crate::node_agent_local_task_supervision::SupervisionContract>,
    now: u128,
    stale_after_ms: u128,
    cutoff: i64,
    observed_events: &HashSet<String>,
) -> Result<bool> {
    let initial = runtime
        .local_tasks
        .get(task_id)?
        .context("orphan candidate disappeared before admission")?;
    if initial.status == "cancel_requested" {
        return cancel::reconcile_candidate(
            runtime,
            &initial,
            now,
            stale_after_ms,
            cutoff,
            observed_events,
        )
        .await;
    }
    let admission_base = if is_platform_supervised(&initial) {
        let contract = contract.context("supervised orphan candidate has no durable contract")?;
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
        "running" | "recovering" | "reattaching" | "cancel_requested"
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
    if !exact_runtime_protects(runtime, &task, active_workspace, now, stale_after_ms).await?
        && ghost_convergence::converge_verified_history(runtime, &task, active_workspace, cutoff)?
    {
        return Ok(true);
    }
    if candidate_runtime_protects(
        runtime,
        &task,
        active_workspace,
        contract,
        now,
        stale_after_ms,
    )
    .await?
    {
        return Ok(false);
    }
    if is_platform_supervised(&task) {
        let contract =
            contract.context("supervised orphan contract disappeared under admission")?;
        if !active_workspace.is_dir() || has_finished_success(runtime, task_id)? {
            if let Some(completion) =
                crate::node_agent_terminal_finalization::historical_completion(&task, contract)?
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
        crate::node_agent_sidecar_recovery::persist_orphan_resume_receipt(runtime, &task)
            .context("persist supervised orphan resume receipt")?;
    }
    let changed = runtime
        .local_tasks
        .mark_one_stale_without_runtime(task_id, cutoff)?;
    if changed {
        runtime.task_journal.record_finished_with_outcome(
            task_id,
            "resume_required",
            Some("执行器所有权已过期；现场已保留，请检查后继续"),
        )?;
    }
    Ok(changed)
}

async fn candidate_runtime_protects(
    runtime: &NodeRuntime,
    task: &crate::node_agent_local_task_store::LocalTaskRecord,
    active_workspace: &Path,
    contract: Option<&crate::node_agent_local_task_supervision::SupervisionContract>,
    now: u128,
    stale_after_ms: u128,
) -> Result<bool> {
    if exact_runtime_protects(runtime, task, active_workspace, now, stale_after_ms).await? {
        return Ok(true);
    }
    if is_platform_supervised(task) {
        let contract =
            contract.context("supervised orphan contract disappeared under admission")?;
        let root = contract.root_task_id.as_deref().unwrap_or(&task.task_id);
        if crate::node_agent_supervision_terminal_lease_safety::runtime_lineage_or_workspace_is_active(
            runtime,
            &task.task_id,
            root,
            active_workspace,
        )
        .await?
        {
            return Ok(true);
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
#[path = "node_agent_local_task_orphan_reconcile_batch_tests.rs"]
mod batch_tests;

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
