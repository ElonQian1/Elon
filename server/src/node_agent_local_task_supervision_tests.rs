use super::*;

#[test]
fn supervised_prompt_marks_executor_and_prevents_redispatch() {
    let contract = normalize_contract(SupervisionContractInput {
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
fn journal_state_reads_review_after_paginated_event_window() {
    let task_id = "local-supervision-long";
    let directory = std::env::temp_dir().join(format!(
        "elon-supervision-test-{}-{}",
        std::process::id(),
        now_ms()
    ));
    let journal = TaskJournal::new(&directory);
    let contract = normalize_contract(SupervisionContractInput {
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
