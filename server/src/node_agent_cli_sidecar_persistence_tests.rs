use crate::{
    node_agent_cli_sidecar::{now_ms, CliSidecarRegistry, CliSidecarSessionRecord},
    node_agent_cli_sidecar_io::{append_output, CliSidecarOutputRecord},
    node_agent_cli_sidecar_runner::{
        follow_sidecar_output, follow_sidecar_output_from_with_batch, CliSidecarOutputEvent,
        CliSidecarReplayCursor,
    },
    node_agent_task_journal::{TaskJournal, TaskJournalStart},
};
use std::{fs, path::PathBuf, time::Duration};
use tokio::sync::watch;

#[tokio::test]
async fn sessions_persistence_failure_drains_output_before_explicit_terminal_failure() {
    let root = temp_dir("sessions-persistence-degraded-follow");
    let registry_dir = root.join("sidecars");
    let registry = CliSidecarRegistry::new(&registry_dir);
    let task_id = "task-persistence-degraded-follow";
    let session_id = "sidecar-persistence-degraded-follow";
    let journal = TaskJournal::new(root.join("journal"));
    journal
        .record_started(TaskJournalStart {
            req_id: task_id,
            cli_name: "codex",
            route: Some("route_a_external_cli"),
            run_handle_id: Some(task_id),
            cwd: Some("D:/preserved-worktree"),
            runtime_permission: Some("full_access"),
        })
        .unwrap();
    let worktree_sentinel = root.join("preserved-worktree/user-change.txt");
    fs::create_dir_all(worktree_sentinel.parent().unwrap()).unwrap();
    fs::write(&worktree_sentinel, b"uncommitted user work").unwrap();
    registry
        .upsert_session(CliSidecarSessionRecord::managed_pipe_json(
            session_id,
            task_id,
            "codex",
            "route_a_external_cli",
            Some(root.to_string_lossy().to_string()),
            None,
            Some(std::process::id()),
            None,
            now_ms(),
        ))
        .unwrap();
    registry
        .upsert_session(CliSidecarSessionRecord::managed_pipe_json(
            "sidecar-backup-generation",
            "task-backup-generation",
            "codex",
            "route_a_external_cli",
            None,
            None,
            Some(std::process::id()),
            None,
            now_ms(),
        ))
        .unwrap();
    let primary = registry_dir.join("sessions.json");
    fs::remove_file(&primary).unwrap();
    fs::create_dir(&primary).unwrap();

    let output_path = registry.output_path(task_id, session_id);
    append_output(
        &output_path,
        CliSidecarOutputRecord::chunk("stdout", "before-registry-failure\n"),
    )
    .unwrap();
    let delayed_output = output_path.clone();
    let writer = tokio::spawn(async move {
        tokio::time::sleep(Duration::from_millis(50)).await;
        append_output(
            &delayed_output,
            CliSidecarOutputRecord::chunk("stdout", "after-registry-failure\n"),
        )
        .unwrap();
        append_output(&delayed_output, CliSidecarOutputRecord::exit(true, false)).unwrap();
    });

    let (_cancel_tx, mut cancel_rx) = watch::channel(false);
    let mut visible = String::new();
    let result = tokio::time::timeout(
        Duration::from_secs(5),
        follow_sidecar_output(&registry, task_id, &output_path, &mut cancel_rx, |event| {
            if let CliSidecarOutputEvent::Stdout(text) = event {
                journal.record_cli_chunk(task_id, "stdout", &text).unwrap();
                visible.push_str(&text);
            }
        }),
    )
    .await
    .expect("follower must reach the real sidecar terminal record")
    .expect("registry persistence failure must not abort output draining");
    writer.await.unwrap();

    assert!(
        result.exit_ok,
        "the child process itself completed successfully"
    );
    assert!(visible.contains("before-registry-failure"));
    assert!(visible.contains("after-registry-failure"));
    let terminal = result
        .terminal_error
        .as_deref()
        .expect("persistent registry failure must be explicit at terminal");
    assert!(terminal.contains("sessions 游标持续持久化失败"));
    assert!(terminal.contains("task journal、工作树与 sidecar JSONL 均已保留"));
    assert!(registry
        .all_sessions()
        .expect_err("persistent registry damage must remain observable")
        .to_string()
        .contains("原子重建"));
    let persisted_output = fs::read_to_string(&output_path).unwrap();
    assert!(persisted_output.contains("before-registry-failure"));
    assert!(persisted_output.contains("after-registry-failure"));
    let persisted_journal = journal.completion_output(task_id, 10_000).unwrap();
    assert!(persisted_journal.contains("before-registry-failure"));
    assert!(persisted_journal.contains("after-registry-failure"));
    assert_eq!(
        fs::read(&worktree_sentinel).unwrap(),
        b"uncommitted user work"
    );
    let _ = fs::remove_dir_all(root);
}

#[tokio::test]
async fn recovery_batch_persistence_failure_remains_fail_closed() {
    let root = temp_dir("strict-recovery-persistence");
    let registry = CliSidecarRegistry::new(root.join("sidecars"));
    let output_path = root.join("strict-recovery-output.jsonl");
    append_output(
        &output_path,
        CliSidecarOutputRecord::chunk("stdout", "must-be-durable-before-cursor\n"),
    )
    .unwrap();
    append_output(&output_path, CliSidecarOutputRecord::exit(true, false)).unwrap();
    let (_cancel_tx, mut cancel_rx) = watch::channel(false);

    let error = follow_sidecar_output_from_with_batch(
        &registry,
        "task-strict-recovery",
        &output_path,
        CliSidecarReplayCursor::default(),
        &mut cancel_rx,
        |_| {},
        |_, _| anyhow::bail!("injected durable journal failure"),
    )
    .await
    .expect_err("recovery journal failure must not advance or claim success");

    assert!(format!("{error:#}").contains("injected durable journal failure"));
    let _ = fs::remove_dir_all(root);
}

fn temp_dir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "elon-cli-sidecar-persistence-{}-{}-{}",
        name,
        std::process::id(),
        now_ms()
    ));
    fs::create_dir_all(&dir).expect("temp dir should be created");
    dir
}
