use super::*;

#[tokio::test]
async fn same_event_terminal_conflicts_leave_task_and_receipt_bytes_unchanged() {
    for (label, error) in [
        ("done-to-failed", "late timeout"),
        ("done-to-canceled", "用户已停止 PC CLI 任务"),
    ] {
        let fixture = Fixture::new(label).await;
        fixture.write_completed_receipt();
        fixture.unlock();
        let completed = fixture.completion(true, None);
        fixture.reconcile(&completed).await.unwrap();
        crate::node_agent_supervision_worktree_lease::acquire(&fixture.base, &fixture.active, ROOT)
            .unwrap();

        let mut conflict = completed;
        conflict.exit_ok = false;
        conflict.error = Some(error.into());
        fixture
            .assert_failed_unchanged(
                conflict,
                "same local completion event conflicts with status, outcome, or finished time",
            )
            .await;
    }

    let output = Fixture::new("done-output-conflict").await;
    output.write_completed_receipt();
    output.unlock();
    let completed = output.completion(true, None);
    output.reconcile(&completed).await.unwrap();
    let mut changed_output = completed;
    changed_output.final_output = "different terminal output".into();
    output
        .assert_failed_unchanged(
            changed_output,
            "same local completion event conflicts with status, outcome, or finished time",
        )
        .await;

    let failed = Fixture::new("failed-to-done").await;
    let failure = failed.completion(false, Some("business failure"));
    failed.reconcile(&failure).await.unwrap();
    failed.write_completed_receipt();
    failed.unlock();
    failed
        .assert_failed_unchanged(
            failed.completion(true, None),
            "same local completion event conflicts with status, outcome, or finished time",
        )
        .await;
}

#[tokio::test]
async fn every_terminal_write_boundary_replays_after_restart_without_early_ack() {
    for boundary in [
        TerminalWriteBoundary::Receipt,
        TerminalWriteBoundary::LocalTask,
        TerminalWriteBoundary::Journal,
        TerminalWriteBoundary::Recovery,
    ] {
        let fixture = Fixture::new(&format!("boundary-{boundary:?}")).await;
        fixture.write_completed_receipt();
        fixture.install_recovery(&format!("update-{boundary:?}"), None);
        fixture.unlock();
        fixture.remove_worktree_and_branch();
        let completion = fixture.completion(true, None);
        fixture
            .runtime
            .completion_outbox
            .enqueue(&completion)
            .unwrap();

        let error = fixture
            .reconcile_with_failure(&completion, boundary)
            .await
            .expect_err("failure injection must interrupt reconciliation");
        assert!(format!("{error:#}").contains("injected terminal persistence failure"));
        assert_eq!(
            fixture.runtime.completion_outbox.pending_count().unwrap(),
            1,
            "no partial terminal transaction may acknowledge or delete its outbox source"
        );
        let bound_bytes = fs::read(fixture.receipt_path()).unwrap();
        let bound: serde_json::Value = serde_json::from_slice(&bound_bytes).unwrap();
        assert_eq!(bound["taskId"], TASK);
        assert_eq!(bound["completionEventId"], EVENT);

        let restarted = reopened_runtime(&fixture);
        LocalTerminalReconciler::for_test(
            &restarted,
            fixture.contracts.clone(),
            fixture.receipts.clone(),
        )
        .reconcile(&completion)
        .await
        .unwrap();

        assert_eq!(fs::read(fixture.receipt_path()).unwrap(), bound_bytes);
        let task = restarted.local_tasks.get(TASK).unwrap().unwrap();
        assert_eq!(task.status, "done");
        assert_eq!(task.completion_event_id.as_deref(), Some(EVENT));
        assert_eq!(
            task.workspace_status.as_ref().unwrap()["terminal_snapshot_status"],
            "trusted"
        );
        let terminal_events = restarted
            .task_journal
            .snapshot(TASK, 0, 500)
            .unwrap()
            .events
            .into_iter()
            .filter(|event| {
                event
                    .event
                    .get("completion_event_id")
                    .and_then(serde_json::Value::as_str)
                    == Some(EVENT)
            })
            .count();
        assert_eq!(
            terminal_events, 1,
            "journal replay must append the exact event once"
        );
        let recovery = restarted
            .update_recovery
            .terminal_receipt_for_task(TASK)
            .unwrap()
            .unwrap();
        assert_eq!(recovery.state, UpdateRecoveryState::Verified);
        assert_eq!(recovery.completion_event_id.as_deref(), Some(EVENT));
        assert_eq!(restarted.completion_outbox.pending_count().unwrap(), 1);
    }
}

fn reopened_runtime(fixture: &Fixture) -> NodeRuntime {
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
    runtime.local_tasks = LocalTaskStore::new(fixture.root.join("tasks.sqlite3"));
    runtime.task_journal =
        crate::node_agent_task_journal::TaskJournal::new(fixture.root.join("journal"));
    runtime.completion_outbox = crate::node_agent_completion_outbox::CliCompletionOutbox::new(
        fixture.root.join("outbox.sqlite3"),
    );
    runtime.update_recovery = crate::node_agent_update_recovery::UpdateRecoveryStore::new(
        fixture.root.join("recovery.json"),
    );
    runtime.full_access_grants =
        crate::node_agent_full_access::FullAccessGrantState::load_from_path(
            fixture.root.join("grants.json"),
        );
    runtime
}
