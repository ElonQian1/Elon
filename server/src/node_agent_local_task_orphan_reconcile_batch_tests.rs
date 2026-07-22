use std::{fs, path::Path};

use super::*;
use crate::node_agent_local_task_store::{LocalTaskStart, LocalTaskStore};

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
