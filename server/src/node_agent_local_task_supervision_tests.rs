use super::*;

#[test]
fn supervised_prompt_marks_executor_and_prevents_redispatch() {
    let contract = normalize_contract(SupervisionContractInput {
        protocol: None,
        supervisor: None,
        task_role: None,
        parent_task_id: None,
        root_task_id: None,
        acceptance_criteria: vec!["测试通过".to_string()],
        improvement_policy: None,
    })
    .unwrap();
    let prompt = executor_prompt("修复登录", Some(&contract));
    assert!(prompt.contains("<elon-pc-executor"));
    assert!(prompt.contains("不得再次把写任务派发给 PC 节点"));
    assert!(prompt.contains("测试通过"));
    assert!(prompt.contains("修复登录"));
}

#[test]
fn state_keeps_latest_review_and_counts_evidence() {
    let contract = normalize_contract(SupervisionContractInput {
        protocol: None,
        supervisor: None,
        task_role: None,
        parent_task_id: None,
        root_task_id: None,
        acceptance_criteria: Vec::new(),
        improvement_policy: None,
    })
    .unwrap();
    let events = vec![
        TaskJournalEventView {
            seq: 1,
            event: json!({
                "type":"supervision_contract", "payload": contract_payload(&contract)
            }),
        },
        TaskJournalEventView {
            seq: 2,
            event: json!({
                "type":"tool_event", "event": {"type":"tool_call", "tool":"file_change", "args":{"changes":[{"path":"src/a.rs", "kind":"modify"}]}}
            }),
        },
        TaskJournalEventView {
            seq: 3,
            event: json!({
                "type":"tool_event", "event": {"type":"tool_result", "tool":"shell", "result":"exit=1 tests failed"}
            }),
        },
        TaskJournalEventView {
            seq: 4,
            event: json!({
                "type":"supervision_review", "payload": {
                    "protocol":SUPERVISION_PROTOCOL, "verdict":"needs_follow_up", "summary":"补测试",
                    "improvements":["增加回归"], "reviewed_by":"codex_desktop", "reviewed_at_ms":1
                }
            }),
        },
    ];
    let state = supervision_state(&events);
    assert!(state.enabled);
    assert_eq!(state.evidence.tool_calls, 1);
    assert_eq!(state.evidence.failed_tools, 1);
    assert_eq!(state.evidence.changed_files, vec!["src/a.rs"]);
    assert_eq!(state.review.unwrap().verdict, "needs_follow_up");
}

#[test]
fn codex_json_items_produce_precise_nonzero_evidence() {
    let events = vec![
        TaskJournalEventView {
            seq: 1,
            event: json!({
                "type":"codex_item", "lifecycle":"started",
                "item":{"id":"cmd-1","type":"command_execution","command":"cargo test"}
            }),
        },
        TaskJournalEventView {
            seq: 2,
            event: json!({
                "type":"codex_item", "lifecycle":"completed",
                "item":{"id":"cmd-1","type":"command_execution","command":"cargo test","status":"completed","exit_code":0}
            }),
        },
        TaskJournalEventView {
            seq: 3,
            event: json!({
                "type":"codex_item", "lifecycle":"started",
                "item":{"id":"cmd-2","type":"command_execution","command":"cargo test bad"}
            }),
        },
        TaskJournalEventView {
            seq: 4,
            event: json!({
                "type":"codex_item", "lifecycle":"completed",
                "item":{"id":"cmd-2","type":"command_execution","command":"cargo test bad","status":"failed","exit_code":2,
                    "output":{"tail":["assertion failed"]}}
            }),
        },
        TaskJournalEventView {
            seq: 5,
            event: json!({
                "type":"codex_item", "lifecycle":"started",
                "item":{"id":"file-1","type":"file_change","changes":[{"path":"src/main.rs","kind":"update"}]}
            }),
        },
        TaskJournalEventView {
            seq: 6,
            event: json!({
                "type":"codex_item", "lifecycle":"completed",
                "item":{"id":"file-1","type":"file_change","status":"completed","changes":[{"path":"src/main.rs","kind":"update"}]}
            }),
        },
        TaskJournalEventView {
            seq: 7,
            event: json!({
                "type":"codex_item", "lifecycle":"completed",
                "item":{"id":"msg-1","type":"agent_message","text":"done"}
            }),
        },
    ];
    let state = supervision_state(&events);
    assert_eq!(state.evidence.tool_calls, 3);
    assert_eq!(state.evidence.tool_results, 3);
    assert_eq!(state.evidence.failed_tools, 1);
    assert_eq!(state.evidence.file_change_events, 1);
    assert_eq!(state.evidence.changed_files, vec!["src/main.rs"]);
    assert_eq!(state.evidence.command_exit_codes.len(), 2);
    assert_eq!(state.evidence.failure_summaries.len(), 1);
    assert_eq!(state.evidence.agent_messages, 1);
}

#[test]
fn journal_state_reads_review_after_paginated_event_window() {
    let task_id = "local-supervision-long";
    let directory = std::env::temp_dir().join(format!(
        "elon-supervision-test-{}-{}",
        std::process::id(),
        now_ms()
    ));
    let journal = TaskJournal::new(&directory);
    let contract = normalize_contract(SupervisionContractInput {
        protocol: None,
        supervisor: None,
        task_role: None,
        parent_task_id: None,
        root_task_id: None,
        acceptance_criteria: Vec::new(),
        improvement_policy: None,
    })
    .unwrap();
    record_supervision_event(
        &journal,
        task_id,
        "supervision_contract",
        contract_payload(&contract),
    )
    .unwrap();
    for index in 0..250 {
        journal
            .append_event(json!({"type":"tool_event", "req_id":task_id, "event":{"type":"tool_call", "tool":format!("tool-{index}")}}))
            .unwrap();
    }
    record_supervision_event(
        &journal,
        task_id,
        "supervision_review",
        json!({
            "protocol":SUPERVISION_PROTOCOL, "verdict":"accepted", "summary":"通过",
            "improvements":[], "reviewed_by":"codex_desktop", "reviewed_at_ms":2
        }),
    )
    .unwrap();

    let state = load_supervision_state(&journal, task_id).unwrap();
    assert_eq!(state.evidence.event_count, 252);
    assert_eq!(state.evidence.tool_calls, 250);
    assert_eq!(state.review.unwrap().verdict, "accepted");
    std::fs::remove_dir_all(directory).unwrap();
}

#[test]
fn resume_requires_explicit_current_protocol() {
    let missing = normalize_contract(SupervisionContractInput {
        protocol: None,
        supervisor: None,
        task_role: Some("resume_original".to_string()),
        parent_task_id: Some("local-parent".to_string()),
        root_task_id: Some("local-parent".to_string()),
        acceptance_criteria: Vec::new(),
        improvement_policy: None,
    })
    .expect_err("resume must not infer its security protocol");
    assert!(missing.contains("必须显式携带 supervision.protocol"));

    let wrong = normalize_contract(SupervisionContractInput {
        protocol: Some("elon.desktop_pc_supervision.v0".to_string()),
        supervisor: None,
        task_role: Some("resume_original".to_string()),
        parent_task_id: Some("local-parent".to_string()),
        root_task_id: Some("local-parent".to_string()),
        acceptance_criteria: Vec::new(),
        improvement_policy: None,
    })
    .expect_err("unknown protocol must fail closed");
    assert!(wrong.contains("supervision.protocol 必须是"));

    let current = normalize_contract(SupervisionContractInput {
        protocol: Some(SUPERVISION_PROTOCOL.to_string()),
        supervisor: None,
        task_role: Some("resume_original".to_string()),
        parent_task_id: Some("local-parent".to_string()),
        root_task_id: Some("local-parent".to_string()),
        acceptance_criteria: Vec::new(),
        improvement_policy: None,
    })
    .expect("current protocol should admit resume");
    assert_eq!(current.protocol, SUPERVISION_PROTOCOL);
}

#[test]
fn post_task_improvement_requires_explicit_protocol_parent_and_root() {
    let missing_protocol = normalize_contract(SupervisionContractInput {
        protocol: None,
        supervisor: None,
        task_role: Some("post_task_improvement".to_string()),
        parent_task_id: Some("local-parent".to_string()),
        root_task_id: Some("local-root".to_string()),
        acceptance_criteria: Vec::new(),
        improvement_policy: Some("after_task_only".to_string()),
    })
    .expect_err("self evolution must not infer its security protocol");
    assert!(missing_protocol.contains("必须显式携带 supervision.protocol"));

    let missing_root = normalize_contract(SupervisionContractInput {
        protocol: Some(SUPERVISION_PROTOCOL.to_string()),
        supervisor: None,
        task_role: Some("post_task_improvement".to_string()),
        parent_task_id: Some("local-parent".to_string()),
        root_task_id: None,
        acceptance_criteria: Vec::new(),
        improvement_policy: Some("after_task_only".to_string()),
    })
    .expect_err("self evolution must carry its exact root identity");
    assert!(missing_root.contains("parent_task_id 和 root_task_id"));
}

#[test]
fn review_identity_prevents_pc_operator_from_impersonating_desktop() {
    let rejected = normalize_review(SupervisionReviewRequest {
        verdict: "accepted".to_string(),
        summary: "operator review".to_string(),
        improvements: Vec::new(),
        reviewed_by: Some("codex_desktop".to_string()),
        review_source: Some("local_pc_ui".to_string()),
    })
    .expect_err("PC review must not impersonate the independent desktop supervisor");
    assert!(rejected.contains("不能冒充"));

    let desktop = normalize_review(SupervisionReviewRequest {
        verdict: "accepted".to_string(),
        summary: "desktop review".to_string(),
        improvements: Vec::new(),
        reviewed_by: Some("codex_desktop".to_string()),
        review_source: Some("codex_desktop_helper".to_string()),
    })
    .expect("trusted desktop helper provenance should be accepted");
    assert_eq!(desktop.review_source, "codex_desktop_helper");

    let operator = normalize_review(SupervisionReviewRequest {
        verdict: "needs_follow_up".to_string(),
        summary: "local review".to_string(),
        improvements: Vec::new(),
        reviewed_by: None,
        review_source: None,
    })
    .expect("local operator defaults should remain compatible");
    assert_eq!(operator.reviewed_by, "pc_operator");
    assert_eq!(operator.review_source, "local_pc_api");
}
