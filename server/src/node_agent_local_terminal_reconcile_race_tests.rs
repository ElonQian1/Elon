use super::*;

#[tokio::test]
async fn failed_commit_rechecks_receipt_window_under_same_admission_guard() {
    for variant in ["prepared", "completed", "malformed", "multiple"] {
        let fixture = Fixture::new(&format!("failed-window-{variant}")).await;
        let completion = fixture.completion(false, Some("business failure"));
        let task = fixture.task();
        let contract = crate::node_agent_local_task_supervision::load_supervision_contract(
            &fixture.runtime.task_journal,
            TASK,
        )
        .unwrap()
        .unwrap();
        let _admission =
            crate::node_agent_supervision_worktree_lease::ResumeAdmissionGuard::acquire(
                &fixture.base,
            )
            .unwrap();
        let identity =
            crate::node_agent_supervision_terminal_lease_safety::verify_terminal_identity(
                &fixture.runtime,
                &task,
                &contract,
                TASK,
                crate::node_agent_supervision_terminal_lease_safety::TerminalLeaseExpectation::Exact,
            )
            .await
            .unwrap();
        let preflight = crate::node_agent_terminal_finalization::preflight_with_roots_for_test(
            &identity,
            &completion,
            &fixture.contracts,
            &fixture.receipts,
        )
        .unwrap();

        match variant {
            "malformed" => {
                fs::create_dir_all(fixture.receipt_directory()).unwrap();
                fs::write(
                    fixture
                        .receipt_directory()
                        .join(format!("{}.json", "a".repeat(64))),
                    b"{malformed",
                )
                .unwrap();
            }
            "multiple" => {
                fixture.write_completed_receipt();
                let path = fixture.receipt_path();
                let mut value: serde_json::Value =
                    serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
                let second = "b".repeat(64);
                value["taskContractId"] = serde_json::json!(second);
                fs::write(
                    fixture.receipt_directory().join(format!("{second}.json")),
                    serde_json::to_vec(&value).unwrap(),
                )
                .unwrap();
            }
            state => {
                fixture.write_completed_receipt();
                if state == "prepared" {
                    let path = fixture.receipt_path();
                    let mut value: serde_json::Value =
                        serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
                    value["state"] = serde_json::json!("prepared");
                    fs::write(path, serde_json::to_vec(&value).unwrap()).unwrap();
                }
            }
        }
        let before = fs::read_dir(fixture.receipt_directory())
            .unwrap()
            .map(|entry| {
                let path = entry.unwrap().path();
                (path.clone(), fs::read(path).unwrap())
            })
            .collect::<Vec<_>>();
        let error = preflight
            .commit(&completion)
            .expect_err("receipt appearing after preflight must fail closed");
        assert!(format!("{error:#}").contains(if variant == "multiple" {
            "multiple"
        } else if variant == "malformed" {
            "parse"
        } else {
            "refuses"
        }));
        for (path, bytes) in before {
            assert_eq!(fs::read(path).unwrap(), bytes);
        }
        assert_eq!(fixture.task().status, "running");
        assert_eq!(
            fixture.lease().as_deref(),
            Some(format!("elon-supervision:{ROOT}").as_str())
        );
    }
}

#[tokio::test]
async fn successful_commit_rechecks_unique_receipt_before_binding() {
    let fixture = Fixture::new("done-window-multiple").await;
    fixture.write_completed_receipt();
    fixture.unlock();
    let completion = fixture.completion(true, None);
    let task = fixture.task();
    let contract = crate::node_agent_local_task_supervision::load_supervision_contract(
        &fixture.runtime.task_journal,
        TASK,
    )
    .unwrap()
    .unwrap();
    let _admission =
        crate::node_agent_supervision_worktree_lease::ResumeAdmissionGuard::acquire(&fixture.base)
            .unwrap();
    let identity = crate::node_agent_terminal_finalization::verify_completed_identity_for_test(
        &fixture.runtime,
        &task,
        &contract,
        &completion,
        &fixture.contracts,
        &fixture.receipts,
    )
    .await
    .unwrap();
    let preflight = crate::node_agent_terminal_finalization::preflight_with_roots_for_test(
        &identity,
        &completion,
        &fixture.contracts,
        &fixture.receipts,
    )
    .unwrap();
    let original = fixture.receipt_path();
    let original_bytes = fs::read(&original).unwrap();
    let mut value: serde_json::Value = serde_json::from_slice(&original_bytes).unwrap();
    let second = "b".repeat(64);
    value["taskContractId"] = serde_json::json!(second);
    let second_path = fixture.receipt_directory().join(format!("{second}.json"));
    fs::write(&second_path, serde_json::to_vec(&value).unwrap()).unwrap();

    let error = preflight
        .commit(&completion)
        .expect_err("a second exact receipt must invalidate binding");
    assert!(format!("{error:#}").contains("multiple"));
    assert_eq!(fs::read(original).unwrap(), original_bytes);
    assert!(serde_json::from_slice::<serde_json::Value>(&fs::read(second_path).unwrap()).is_ok());
    assert_eq!(fixture.task().status, "running");
}

#[tokio::test]
async fn surviving_real_sidecar_exit_replays_one_completion_to_trusted_terminal() {
    let fixture = Fixture::new("real-restart").await;
    fixture
        .runtime
        .task_journal
        .record_started(crate::node_agent_task_journal::TaskJournalStart {
            req_id: TASK,
            cli_name: "codex",
            route: Some("managed_pipe_json_sidecar"),
            run_handle_id: Some(TASK),
            cwd: Some(fixture.active.to_str().unwrap()),
            runtime_permission: Some("full_access"),
        })
        .unwrap();
    fixture.write_completed_receipt();
    fixture.unlock();
    let session_id = "real-restart-sidecar";
    let output_path = fixture.runtime.cli_sidecars.output_path(TASK, session_id);
    let (program, args) = delayed_codex_command();
    let registry_dir = fixture.runtime.cli_sidecars.dir();
    let journal_dir = fixture.root.join("journal");
    let active = fixture.active.to_string_lossy().to_string();
    let worker = tokio::spawn(crate::node_agent_cli_sidecar_runner::run_sidecar(
        crate::node_agent_cli_sidecar_runner::CliSidecarLaunchConfig {
            session_id: session_id.into(),
            task_id: TASK.into(),
            cli_name: "codex".into(),
            route: "managed_pipe_json_sidecar".into(),
            program,
            args,
            cwd: Some(active),
            runtime_permission: Some("full_access".into()),
            env: vec![],
            output_path: output_path.clone(),
            registry_dir,
            task_journal_dir: Some(journal_dir),
            worker_path: None,
            worker_release: None,
            worker_sha256: None,
            codex_session_scope_key: None,
            legacy_codex_sessions_file: None,
            timeout_secs: 10,
            stdin_payload: None,
            runtime_policy: None,
            stdin_piped_empty: false,
            initial_cols: crate::node_agent_cli_pty::default_cols(),
            initial_rows: crate::node_agent_cli_pty::default_rows(),
        },
    ));
    let sidecar = loop {
        if let Some(sidecar) = fixture.runtime.cli_sidecars.session_for_task(TASK).unwrap() {
            if sidecar.can_replay_after_restart_at(crate::node_agent_cli_sidecar::now_ms()) {
                break sidecar;
            }
        }
        tokio::time::sleep(std::time::Duration::from_millis(20)).await;
    };
    assert!(
        crate::node_agent_sidecar_recovery::spawn_recovered_sidecar_monitor(
            fixture.runtime.clone(),
            fixture.task(),
            sidecar,
            None,
            None,
        )
        .await
        .unwrap()
    );
    worker.await.unwrap().unwrap();
    for _ in 0..100 {
        if fixture.runtime.completion_outbox.pending_count().unwrap() == 1
            && !fixture.runtime.cli_prompt_active(TASK).await
        {
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(20)).await;
    }
    assert_eq!(
        fixture.runtime.completion_outbox.pending_count().unwrap(),
        1
    );
    let completion = fixture
        .runtime
        .completion_outbox
        .latest_for_req_id(TASK)
        .unwrap()
        .unwrap();
    assert!(completion.exit_ok);
    assert!(completion.final_output.contains("restart done"));
    if let Err(error) = fixture.reconcile(&completion).await {
        panic!(
            "terminal replay failed: {error:#}; task={:?}; completion={completion:?}",
            fixture.task()
        );
    }
    fixture.reconcile(&completion).await.unwrap();
    let task = fixture.task();
    assert_eq!(task.status, "done");
    assert_eq!(task.completion_event_id, Some(completion.event_id.clone()));
    let terminal_events = fixture
        .runtime
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
                == Some(completion.event_id.as_str())
        })
        .count();
    assert_eq!(terminal_events, 1);
}
