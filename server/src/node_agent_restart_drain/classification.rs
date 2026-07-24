use std::collections::{HashMap, HashSet};

use serde::{Deserialize, Serialize};

use crate::NodeRuntime;

const ACTIVE_HANDLE_STALE_AFTER_MS: u128 = 2 * 60 * 1_000;
const STALE_CANCEL_PROOF_PROTOCOL: &str = "elon.update_stale_cancel_proof.v1";

#[derive(Default)]
pub(super) struct DrainClassification {
    pub(super) blocking: Vec<String>,
    pub(super) recoverable: Vec<String>,
    pub(super) stale: Vec<String>,
    pub(super) stale_cancel_proofs: Vec<StaleCancelProof>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub(super) struct StaleCancelProof {
    protocol: String,
    task_id: String,
    target_release_identity: String,
    target_git_sha: String,
    task_status: String,
    runtime_inventory_complete: bool,
    fresh_exact_handle: bool,
    live_sidecar: bool,
    replayable_sidecar: bool,
    cancel_intent_persisted: bool,
    supervision_protocol: String,
    proven_at_ms: u128,
}

#[derive(Debug, PartialEq, Eq)]
pub(super) enum DrainTaskDisposition {
    Blocking,
    SafeStaleCancel,
}

#[derive(Debug, PartialEq, Eq)]
pub(super) enum StartupCheckpointDisposition {
    Blocking,
    Recoverable,
}

pub(super) fn drain_task_disposition(
    status: &str,
    exact_target_bound: bool,
    runtime_inventory_complete: bool,
    stale_cancel_proven: bool,
) -> DrainTaskDisposition {
    if status == "cancel_requested"
        && exact_target_bound
        && runtime_inventory_complete
        && stale_cancel_proven
    {
        DrainTaskDisposition::SafeStaleCancel
    } else {
        DrainTaskDisposition::Blocking
    }
}

pub(super) async fn classify_supervised_tasks(
    runtime: &NodeRuntime,
    target_release_identity: Option<&str>,
) -> anyhow::Result<DrainClassification> {
    let active = runtime.active_cli_prompts.views_without_approvals().await;
    let active = active
        .into_iter()
        .map(|task| (task.req_id.clone(), task))
        .collect::<HashMap<_, _>>();
    let now = super::now_ms();
    let fresh_runtime_task_ids = active
        .values()
        .filter(|handle| {
            handle.control_handle_live
                && now.saturating_sub(handle.last_heartbeat_ms) <= ACTIVE_HANDLE_STALE_AFTER_MS
        })
        .map(|handle| handle.req_id.clone())
        .collect::<HashSet<_>>();
    let exact_target = exact_target_identity(target_release_identity);
    let mut result = DrainClassification::default();
    let tasks = load_drain_candidates(&runtime.local_tasks)?;
    let proven_stale_cancels = prove_stale_cancels_for_target(
        &runtime.local_tasks,
        &runtime.task_journal,
        &runtime.cli_sidecars,
        exact_target.as_ref(),
        Some(&fresh_runtime_task_ids),
    )?;
    let mut seen = HashSet::new();
    for task in tasks {
        seen.insert(task.task_id.clone());
        let supervised = crate::node_agent_local_task_supervision::load_supervision_state(
            &runtime.task_journal,
            &task.task_id,
        )?
        .enabled;
        if !supervised {
            continue;
        }
        match drain_task_disposition(
            &task.status,
            exact_target.is_some(),
            true,
            proven_stale_cancels.contains(&task.task_id),
        ) {
            DrainTaskDisposition::SafeStaleCancel => {
                let target = exact_target
                    .as_ref()
                    .expect("safe stale cancel requires an exact target");
                result.stale.push(task.task_id.clone());
                result.stale_cancel_proofs.push(StaleCancelProof {
                    protocol: STALE_CANCEL_PROOF_PROTOCOL.to_string(),
                    task_id: task.task_id,
                    target_release_identity: target.release_identity.clone(),
                    target_git_sha: target.git_sha.clone(),
                    task_status: "cancel_requested".to_string(),
                    runtime_inventory_complete: true,
                    fresh_exact_handle: false,
                    live_sidecar: false,
                    replayable_sidecar: false,
                    cancel_intent_persisted: true,
                    supervision_protocol:
                        crate::node_agent_local_task_supervision::SUPERVISION_PROTOCOL.to_string(),
                    proven_at_ms: now,
                });
            }
            DrainTaskDisposition::Blocking => result.blocking.push(task.task_id),
        }
    }
    // A missing durable row must never turn an otherwise active supervised
    // handle into an empty blocking set. Journal read failures also propagate.
    for task in active.values().filter(|task| !seen.contains(&task.req_id)) {
        if crate::node_agent_local_task_supervision::load_supervision_state(
            &runtime.task_journal,
            &task.req_id,
        )?
        .enabled
        {
            result.blocking.push(task.req_id.clone());
        }
    }
    result.blocking.sort();
    result.recoverable.sort();
    result.stale.sort();
    result
        .stale_cancel_proofs
        .sort_by(|left, right| left.task_id.cmp(&right.task_id));
    Ok(result)
}

#[derive(Default)]
pub(super) struct StartupCheckpointClassification {
    pub(super) blocking: Vec<String>,
    pub(super) recoverable: Vec<String>,
}

pub(super) async fn classify_startup_checkpoint_tasks(
    runtime: &NodeRuntime,
    task_ids: &[String],
) -> anyhow::Result<StartupCheckpointClassification> {
    let now = super::now_ms();
    let fresh_handles = runtime
        .active_cli_prompts
        .views_without_approvals()
        .await
        .into_iter()
        .filter(|handle| {
            handle.control_handle_live
                && now.saturating_sub(handle.last_heartbeat_ms) <= ACTIVE_HANDLE_STALE_AFTER_MS
        })
        .map(|handle| handle.req_id)
        .collect::<HashSet<_>>();
    let mut result = StartupCheckpointClassification::default();
    for task_id in task_ids {
        let task = runtime.local_tasks.get(task_id)?;
        let mut has_execution_owner = fresh_handles.contains(task_id);
        if !has_execution_owner {
            has_execution_owner = runtime
                .cli_sidecars
                .session_for_task(task_id)?
                .is_some_and(|session| session.protects_startup_reconcile_at(now));
        }
        if !has_execution_owner {
            has_execution_owner = runtime.task_journal.record(task_id)?.is_some_and(|record| {
                if !matches!(
                    record.status.as_str(),
                    "running" | "recovering" | "reattaching" | "cancel_requested"
                ) {
                    return false;
                }
                let heartbeat = record.heartbeat_at_ms.unwrap_or(record.updated_at_ms);
                if heartbeat <= now && now.saturating_sub(heartbeat) <= ACTIVE_HANDLE_STALE_AFTER_MS
                {
                    return true;
                }
                crate::node_agent_local_task_orphan_reconcile::recorded_process_is_live(&record)
                    .unwrap_or(true)
            });
        }
        match startup_checkpoint_disposition(
            task.as_ref().map(|task| task.status.as_str()),
            has_execution_owner,
        ) {
            StartupCheckpointDisposition::Blocking => result.blocking.push(task_id.clone()),
            StartupCheckpointDisposition::Recoverable => result.recoverable.push(task_id.clone()),
        }
    }
    result.blocking.sort();
    result.recoverable.sort();
    Ok(result)
}

pub(super) fn startup_checkpoint_disposition(
    _status: Option<&str>,
    has_execution_owner: bool,
) -> StartupCheckpointDisposition {
    if has_execution_owner {
        StartupCheckpointDisposition::Blocking
    } else {
        StartupCheckpointDisposition::Recoverable
    }
}

#[derive(Debug, Clone)]
struct ExactTargetIdentity {
    release_identity: String,
    git_sha: String,
}

fn exact_target_identity(value: Option<&str>) -> Option<ExactTargetIdentity> {
    let value = value?.trim();
    let (version, git_sha) = value.rsplit_once('+')?;
    let version = version.trim();
    let git_sha = git_sha.trim();
    if version.is_empty()
        || git_sha.len() < 7
        || !git_sha.bytes().all(|byte| byte.is_ascii_hexdigit())
    {
        return None;
    }
    Some(ExactTargetIdentity {
        release_identity: format!("{version}+{git_sha}"),
        git_sha: git_sha.to_string(),
    })
}

fn prove_stale_cancels_for_target(
    local_tasks: &crate::node_agent_local_task_store::LocalTaskStore,
    journal: &crate::node_agent_task_journal::TaskJournal,
    sidecars: &crate::node_agent_cli_sidecar::CliSidecarRegistry,
    exact_target: Option<&ExactTargetIdentity>,
    fresh_runtime_task_ids: Option<&HashSet<String>>,
) -> anyhow::Result<HashSet<String>> {
    if exact_target.is_none() || fresh_runtime_task_ids.is_none() {
        return Ok(HashSet::new());
    }
    crate::node_agent_update_checkpoint::proven_stale_cancelled_tasks(
        local_tasks,
        journal,
        sidecars,
        fresh_runtime_task_ids,
    )
}

pub(super) fn load_drain_candidates(
    store: &crate::node_agent_local_task_store::LocalTaskStore,
) -> anyhow::Result<Vec<crate::node_agent_local_task_store::LocalTaskRecord>> {
    store
        .list_update_candidates()
        .map_err(|error| anyhow::anyhow!("durable supervised task query failed: {error:#}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::node_agent_local_task_store::LocalTaskStart;

    #[tokio::test]
    async fn prelaunch_submit_and_resume_block_update_without_becoming_stale() {
        let root = std::env::temp_dir().join(format!(
            "restart-drain-prelaunch-{}",
            uuid::Uuid::new_v4().simple()
        ));
        std::fs::create_dir_all(&root).unwrap();
        let mut runtime = crate::NodeRuntime::new(
            crate::node_agent_config::NodeConfig {
                cloud_url: "ws://127.0.0.1".into(),
                cloud_http_url: "http://127.0.0.1".into(),
                ollama_url: "http://127.0.0.1".into(),
                lm_studio_url: None,
                custom_url: None,
                price_per_1k: 0.0,
            },
            Some(crate::node_agent_config::Credentials {
                agent_id: "agent-prelaunch".into(),
                agent_secret: "unused".into(),
                owner_user_id: "owner-prelaunch".into(),
                user_token: None,
            }),
            crate::pc_storage_repo::StorageSettings::default(),
            crate::node_agent_data_root::resolve(None, None, None),
            "install-prelaunch".into(),
        );
        runtime.task_journal =
            crate::node_agent_task_journal::TaskJournal::new(root.join("journal"));
        runtime.local_tasks =
            crate::node_agent_local_task_store::LocalTaskStore::new(root.join("tasks.db"));
        runtime.cli_sidecars =
            crate::node_agent_cli_sidecar::CliSidecarRegistry::new(root.join("sidecars"));

        for (task_id, role, parent_task_id) in [
            ("fresh-submit", "requirement", None),
            ("fresh-resume", "resume_original", Some("finished-parent")),
        ] {
            runtime
                .local_tasks
                .create(LocalTaskStart {
                    task_id,
                    owner_user_id: "owner-prelaunch",
                    agent_id: "agent-prelaunch",
                    install_id: "install-prelaunch",
                    project_id: "elon-self",
                    channel_id: None,
                    conversation_id: task_id,
                    workspace_path: root.to_string_lossy().as_ref(),
                    prompt: "must survive the prelaunch registration window",
                    cli: "codex",
                    runtime_permission: "full_access",
                })
                .unwrap();
            let contract = crate::node_agent_local_task_supervision::SupervisionContract {
                protocol: crate::node_agent_local_task_supervision::SUPERVISION_PROTOCOL
                    .to_string(),
                supervisor: "codex_desktop".to_string(),
                task_role: role.to_string(),
                parent_task_id: parent_task_id.map(str::to_string),
                root_task_id: Some("root-prelaunch".to_string()),
                acceptance_criteria: Vec::new(),
                improvement_policy: "after_task_or_unblock".to_string(),
            };
            crate::node_agent_local_task_supervision::record_supervision_event(
                &runtime.task_journal,
                task_id,
                "supervision_contract",
                crate::node_agent_local_task_supervision::contract_payload(&contract),
            )
            .unwrap();
        }

        let classification = classify_supervised_tasks(&runtime, Some("0.3.70+aaaaaaaaaaaaaaaa"))
            .await
            .unwrap();
        assert_eq!(
            classification.blocking,
            vec!["fresh-resume".to_string(), "fresh-submit".to_string()]
        );
        assert!(classification.recoverable.is_empty());
        assert!(classification.stale.is_empty());
        assert!(classification.stale_cancel_proofs.is_empty());

        for task_id in ["fresh-submit", "fresh-resume"] {
            let task = runtime.local_tasks.get(task_id).unwrap().unwrap();
            assert_eq!(task.status, "running");
            assert!(task.finished_at_ms.is_none());
            assert!(task.completion_event_id.is_none());
            let snapshot = runtime.task_journal.snapshot(task_id, 0, 20).unwrap();
            assert!(snapshot.events.iter().all(|event| {
                event.event.get("type").and_then(serde_json::Value::as_str)
                    != Some("supervision_stale_runtime_resume_required")
            }));
        }
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn shared_drain_excludes_resume_required_but_installer_gate_includes_it() {
        let root = std::env::temp_dir().join(format!(
            "restart-drain-query-boundary-{}",
            uuid::Uuid::new_v4().simple()
        ));
        std::fs::create_dir_all(&root).unwrap();
        let store = crate::node_agent_local_task_store::LocalTaskStore::new(root.join("tasks.db"));
        for task_id in [
            "running",
            "recovering",
            "cancel-requested",
            "resume-required",
        ] {
            store
                .create(LocalTaskStart {
                    task_id,
                    owner_user_id: "owner",
                    agent_id: "agent",
                    install_id: "install",
                    project_id: "project",
                    channel_id: None,
                    conversation_id: task_id,
                    workspace_path: root.to_string_lossy().as_ref(),
                    prompt: "query boundary",
                    cli: "codex",
                    runtime_permission: "full_access",
                })
                .unwrap();
        }
        assert!(store.mark_recovering("recovering", "recovering").unwrap());
        assert!(store.mark_cancel_requested("cancel-requested").unwrap());
        assert!(store
            .mark_recovery_blocked("resume-required", "resume")
            .unwrap());

        let shared = load_drain_candidates(&store)
            .unwrap()
            .into_iter()
            .map(|task| task.task_id)
            .collect::<HashSet<_>>();
        assert_eq!(
            shared,
            HashSet::from([
                "running".to_string(),
                "recovering".to_string(),
                "cancel-requested".to_string()
            ])
        );
        let installer = store
            .list_update_install_candidates()
            .unwrap()
            .into_iter()
            .map(|task| task.task_id)
            .collect::<HashSet<_>>();
        assert!(installer.contains("resume-required"));
        assert_eq!(installer.len(), 4);
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn exact_target_cancel_proof_reuses_fail_closed_installer_inventory() {
        let root = std::env::temp_dir().join(format!(
            "restart-drain-cancel-proof-{}",
            uuid::Uuid::new_v4().simple()
        ));
        std::fs::create_dir_all(&root).unwrap();
        let store = crate::node_agent_local_task_store::LocalTaskStore::new(root.join("tasks.db"));
        store
            .create(LocalTaskStart {
                task_id: "stale-cancel",
                owner_user_id: "owner",
                agent_id: "agent",
                install_id: "install",
                project_id: "project",
                channel_id: None,
                conversation_id: "conversation",
                workspace_path: root.to_string_lossy().as_ref(),
                prompt: "cancel me",
                cli: "codex",
                runtime_permission: "full_access",
            })
            .unwrap();
        let journal = crate::node_agent_task_journal::TaskJournal::new(root.join("journal"));
        journal
            .record_started(crate::node_agent_task_journal::TaskJournalStart {
                req_id: "stale-cancel",
                cli_name: "codex",
                route: Some("route_a_external_cli"),
                run_handle_id: Some("stale-cancel"),
                cwd: root.to_str(),
                runtime_permission: Some("full_access"),
            })
            .unwrap();
        let contract = crate::node_agent_local_task_supervision::SupervisionContract {
            protocol: crate::node_agent_local_task_supervision::SUPERVISION_PROTOCOL.to_string(),
            supervisor: "codex_desktop".to_string(),
            task_role: "capability_repair".to_string(),
            parent_task_id: Some("parent".to_string()),
            root_task_id: Some("root".to_string()),
            acceptance_criteria: Vec::new(),
            improvement_policy: "after_task_or_unblock".to_string(),
        };
        crate::node_agent_local_task_supervision::record_supervision_event(
            &journal,
            "stale-cancel",
            "supervision_contract",
            crate::node_agent_local_task_supervision::contract_payload(&contract),
        )
        .unwrap();
        assert!(store.mark_cancel_requested("stale-cancel").unwrap());
        journal.record_cancel_requested("stale-cancel").unwrap();
        let sidecars =
            crate::node_agent_cli_sidecar::CliSidecarRegistry::new(root.join("sidecars"));
        let target = exact_target_identity(Some("0.3.69+51841351542f7cc3"));

        let safe = prove_stale_cancels_for_target(
            &store,
            &journal,
            &sidecars,
            target.as_ref(),
            Some(&HashSet::new()),
        )
        .unwrap();
        assert_eq!(safe, HashSet::from(["stale-cancel".to_string()]));
        assert_eq!(
            store.get("stale-cancel").unwrap().unwrap().status,
            "cancel_requested"
        );
        assert!(
            prove_stale_cancels_for_target(&store, &journal, &sidecars, target.as_ref(), None,)
                .unwrap()
                .is_empty()
        );
        assert!(prove_stale_cancels_for_target(
            &store,
            &journal,
            &sidecars,
            exact_target_identity(Some("0.3.69")).as_ref(),
            Some(&HashSet::new()),
        )
        .unwrap()
        .is_empty());
        assert!(prove_stale_cancels_for_target(
            &store,
            &journal,
            &sidecars,
            target.as_ref(),
            Some(&HashSet::from(["stale-cancel".to_string()])),
        )
        .unwrap()
        .is_empty());

        sidecars
            .upsert_session(
                crate::node_agent_cli_sidecar::CliSidecarSessionRecord::managed_conpty(
                    "live-sidecar",
                    "stale-cancel",
                    "codex",
                    "route_a_external_cli",
                    Some(root.to_string_lossy().into_owned()),
                    Some("npipe://elon/live-sidecar".to_string()),
                    Some(std::process::id()),
                    None,
                    crate::node_agent_cli_sidecar::now_ms(),
                ),
            )
            .unwrap();
        assert!(prove_stale_cancels_for_target(
            &store,
            &journal,
            &sidecars,
            target.as_ref(),
            Some(&HashSet::new()),
        )
        .unwrap()
        .is_empty());
        std::fs::write(root.join("sidecars/sessions.json"), "not-json").unwrap();
        std::fs::write(root.join("sidecars/sessions.json.bak"), "not-json").unwrap();
        assert!(prove_stale_cancels_for_target(
            &store,
            &journal,
            &sidecars,
            target.as_ref(),
            Some(&HashSet::new()),
        )
        .is_err());
        let _ = std::fs::remove_dir_all(root);
    }
}
