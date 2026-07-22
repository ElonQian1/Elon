use std::{fs, path::Path, time::Duration};

use homecli_proto::CancelRequestAudit;

use super::*;
use crate::{
    node_agent_local_task_store::{LocalTaskStart, LocalTaskStore},
    node_agent_task_journal::{CancelIntentTarget, TaskJournalStart},
};

#[tokio::test]
async fn invalid_candidate_does_not_block_later_safe_orphan_reconciliation() {
    let root = std::env::temp_dir().join(format!(
        "elon-orphan-batch-{}",
        uuid::Uuid::new_v4().simple()
    ));
    let runtime = test_runtime(&root);
    for task_id in ["bad-supervised", "good-ordinary"] {
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
    runtime
        .local_tasks
        .record_initial_workspace_status(
            "bad-supervised",
            &serde_json::json!({
                "platform_provenance": "elon.conversation_worktree.v1",
                "active_workspace_path": root,
            }),
        )
        .unwrap();

    assert_eq!(reconcile_with_stale_after(&runtime, 0).await.unwrap(), 1);
    assert_eq!(
        runtime
            .local_tasks
            .get("bad-supervised")
            .unwrap()
            .unwrap()
            .status,
        "running"
    );
    assert_eq!(
        runtime
            .local_tasks
            .get("good-ordinary")
            .unwrap()
            .unwrap()
            .status,
        "resume_required"
    );
    let _ = fs::remove_dir_all(root);
}

#[tokio::test]
async fn malformed_terminal_repair_does_not_block_later_safe_orphan_reconciliation() {
    let root = std::env::temp_dir().join(format!(
        "elon-orphan-terminal-batch-{}",
        uuid::Uuid::new_v4().simple()
    ));
    let runtime = test_runtime(&root);
    for task_id in ["bad-terminal", "good-running"] {
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
    runtime
        .local_tasks
        .record_initial_workspace_status(
            "bad-terminal",
            &serde_json::json!({
                "platform_provenance": "elon.conversation_worktree.v1",
                "active_workspace_path": root,
            }),
        )
        .unwrap();
    rusqlite::Connection::open(root.join("tasks.sqlite3"))
        .unwrap()
        .execute(
            "UPDATE local_tasks
                SET status='done', completion_event_id='bad-event',
                    sync_state='local_only', finished_at_ms=started_at_ms
              WHERE task_id='bad-terminal'",
            [],
        )
        .unwrap();

    assert_eq!(reconcile_with_stale_after(&runtime, 0).await.unwrap(), 1);
    assert_eq!(
        runtime
            .local_tasks
            .get("bad-terminal")
            .unwrap()
            .unwrap()
            .status,
        "done"
    );
    assert_eq!(
        runtime
            .local_tasks
            .get("good-running")
            .unwrap()
            .unwrap()
            .status,
        "resume_required"
    );
    let _ = fs::remove_dir_all(root);
}

#[tokio::test]
async fn contended_terminal_repair_does_not_wait_or_block_later_orphan() {
    let root = std::env::temp_dir().join(format!(
        "elon-orphan-terminal-contention-{}",
        uuid::Uuid::new_v4().simple()
    ));
    let base = root.join("base");
    fs::create_dir_all(&root).unwrap();
    git(&root, &["init", "-b", "main", base.to_str().unwrap()]);
    let base = base.canonicalize().unwrap();
    let runtime = test_runtime(&root);
    for task_id in ["contended-terminal", "good-after-contention"] {
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
                workspace_path: base.to_str().unwrap(),
                prompt: "work",
                cli: "codex",
                runtime_permission: "full_access",
            })
            .unwrap();
    }
    runtime
        .local_tasks
        .record_initial_workspace_status(
            "contended-terminal",
            &serde_json::json!({
                "platform_provenance": "elon.conversation_worktree.v1",
                "root_task_id": "contended-terminal",
                "base_workspace_path": base,
                "active_workspace_path": base,
            }),
        )
        .unwrap();
    crate::node_agent_local_task_supervision::record_supervision_event(
        &runtime.task_journal,
        "contended-terminal",
        "supervision_contract",
        crate::node_agent_local_task_supervision::contract_payload(
            &crate::node_agent_local_task_supervision::SupervisionContract {
                protocol: crate::node_agent_local_task_supervision::SUPERVISION_PROTOCOL.into(),
                supervisor: "codex_desktop".into(),
                task_role: "requirement".into(),
                parent_task_id: None,
                root_task_id: Some("contended-terminal".into()),
                acceptance_criteria: vec![],
                improvement_policy: "after_task_only".into(),
            },
        ),
    )
    .unwrap();
    rusqlite::Connection::open(root.join("tasks.sqlite3"))
        .unwrap()
        .execute(
            "UPDATE local_tasks
                SET status='done', completion_event_id='contended-event',
                    sync_state='local_only', finished_at_ms=started_at_ms
              WHERE task_id='contended-terminal'",
            [],
        )
        .unwrap();
    let admission =
        crate::node_agent_supervision_worktree_lease::ResumeAdmissionGuard::acquire(&base).unwrap();

    let started = std::time::Instant::now();
    assert_eq!(reconcile_with_stale_after(&runtime, 0).await.unwrap(), 1);
    assert!(
        started.elapsed() < Duration::from_secs(2),
        "maintenance reconciliation waited for interactive admission contention"
    );
    assert_eq!(
        runtime
            .local_tasks
            .get("contended-terminal")
            .unwrap()
            .unwrap()
            .status,
        "done"
    );
    assert_eq!(
        runtime
            .local_tasks
            .get("good-after-contention")
            .unwrap()
            .unwrap()
            .status,
        "resume_required"
    );
    drop(admission);
    let _ = fs::remove_dir_all(root);
}

#[tokio::test]
async fn durable_cancel_with_malformed_supervision_contract_still_converges() {
    let root = std::env::temp_dir().join(format!(
        "elon-orphan-malformed-supervised-cancel-{}",
        uuid::Uuid::new_v4().simple()
    ));
    let runtime = test_runtime(&root);
    runtime
        .local_tasks
        .create(LocalTaskStart {
            task_id: "malformed-supervised-cancel",
            owner_user_id: "owner",
            agent_id: "agent",
            install_id: "install",
            project_id: "project",
            channel_id: None,
            conversation_id: "malformed-supervised-cancel",
            workspace_path: root.to_str().unwrap(),
            prompt: "work",
            cli: "codex",
            runtime_permission: "full_access",
        })
        .unwrap();
    runtime
        .local_tasks
        .record_initial_workspace_status(
            "malformed-supervised-cancel",
            &serde_json::json!({
                "platform_provenance": "elon.conversation_worktree.v1",
                "active_workspace_path": root,
            }),
        )
        .unwrap();
    runtime
        .task_journal
        .record_started(TaskJournalStart {
            req_id: "malformed-supervised-cancel",
            cli_name: "codex",
            route: Some("route_a_external_cli"),
            run_handle_id: Some("malformed-supervised-cancel"),
            cwd: root.to_str(),
            runtime_permission: Some("full_access"),
        })
        .unwrap();
    runtime
        .task_journal
        .record_cancel_intent(
            "malformed-supervised-cancel",
            CancelIntentTarget {
                run_handle_id: Some("malformed-supervised-cancel".into()),
                active_started_at_ms: None,
                sidecar_session_id: None,
            },
            &CancelRequestAudit {
                requested_by: Some("owner".into()),
                source: Some("test".into()),
                reason: Some("stop".into()),
                requested_at_ms: Some(1),
                interruption_source: None,
            },
        )
        .unwrap();
    runtime
        .local_tasks
        .mark_cancel_requested("malformed-supervised-cancel")
        .unwrap();

    assert_eq!(reconcile_with_stale_after(&runtime, 0).await.unwrap(), 1);
    assert_eq!(
        runtime
            .local_tasks
            .get("malformed-supervised-cancel")
            .unwrap()
            .unwrap()
            .status,
        "canceled"
    );
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
    runtime.task_journal = crate::node_agent_task_journal::TaskJournal::new(root.join("journal"));
    runtime.completion_outbox =
        crate::node_agent_completion_outbox::CliCompletionOutbox::new(root.join("outbox.sqlite3"));
    runtime.update_recovery =
        crate::node_agent_update_recovery::UpdateRecoveryStore::new(root.join("recovery.json"));
    runtime
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
