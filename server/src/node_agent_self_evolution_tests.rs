use super::*;

fn item(id: &str) -> SelfEvolutionItem {
    SelfEvolutionItem {
        logical_id: id.to_string(),
        root_task_id: "root-a".to_string(),
        parent_task_id: "parent-a".to_string(),
        owner_user_id: "owner-a".to_string(),
        agent_id: "agent-a".to_string(),
        install_id: "install-a".to_string(),
        project_id: "project-a".to_string(),
        channel_id: None,
        conversation_id: "self-evolution-a".to_string(),
        workspace_path: "C:/repo".to_string(),
        execution_worktree: None,
        execution_branch: None,
        execution_isolated: false,
        prompt: "improve platform".to_string(),
        runtime_permission: "full_access".to_string(),
        status: "queued".to_string(),
        active_task_id: None,
        generation: 0,
        pause_reason: None,
        yield_reason: None,
        interruption_source: None,
        review_verdict: None,
        review_note: None,
        reviewed_by: None,
        review_source: None,
        reviewed_at_ms: None,
        retry_count: 0,
        max_retries: 3,
        next_retry_at_ms: None,
        last_error: None,
        pending_action: None,
        created_at_ms: 1,
        updated_at_ms: 1,
    }
}

fn coordinator(name: &str) -> SelfEvolutionCoordinator {
    let path = std::env::temp_dir().join(format!(
        "elon-self-evolution-{name}-{}-{}.json",
        std::process::id(),
        Uuid::new_v4()
    ));
    SelfEvolutionCoordinator::new(path)
}

#[test]
fn foreground_gate_yields_then_auto_reserves_next_generation() {
    let coordinator = coordinator("yield-resume");
    coordinator.register(item("evo-a")).unwrap();
    coordinator
        .update_gates(SelfEvolutionGates {
            foreground_task_ids: vec!["foreground-a".to_string()],
            checked_at_ms: 2,
            ..Default::default()
        })
        .unwrap();
    assert!(coordinator.reserve_next().unwrap().is_none());

    coordinator
        .update_gates(SelfEvolutionGates {
            checked_at_ms: 3,
            ..Default::default()
        })
        .unwrap();
    let first = coordinator.reserve_next().unwrap().unwrap();
    assert_eq!(first.generation, 1);
    assert_eq!(first.status, "starting");

    {
        let mut state = coordinator.state.lock().unwrap();
        let current = state
            .items
            .iter_mut()
            .find(|item| item.logical_id == "evo-a")
            .unwrap();
        current.status = "running".to_string();
    }
    coordinator
        .update_gates(SelfEvolutionGates {
            publish_active: true,
            publish_status: "observed".to_string(),
            checked_at_ms: 4,
            ..Default::default()
        })
        .unwrap();
    let pauses = coordinator.request_gate_pauses().unwrap();
    assert_eq!(pauses.len(), 1);
    assert_eq!(pauses[0].3, "global_publish");
    let persisted = coordinator.list_for_owner("owner-a").unwrap().0.remove(0);
    assert_eq!(persisted.status, "running");
    assert_eq!(persisted.pending_action.as_ref().unwrap().action, "pause");

    coordinator
        .commit_action("owner-a", "evo-a", "pause")
        .unwrap();

    {
        let mut state = coordinator.state.lock().unwrap();
        let current = state
            .items
            .iter_mut()
            .find(|item| item.logical_id == "evo-a")
            .unwrap();
        current.status = "paused".to_string();
        current.active_task_id = None;
    }
    coordinator
        .update_gates(SelfEvolutionGates::default())
        .unwrap();
    let resumed = coordinator.reserve_next().unwrap().unwrap();
    assert_eq!(resumed.generation, 2);
    assert_eq!(resumed.conversation_id, first.conversation_id);
    assert_ne!(resumed.active_task_id, first.active_task_id);
}

#[test]
fn one_active_item_per_root_and_review_are_persisted() {
    let coordinator = coordinator("root-review");
    coordinator.register(item("evo-a")).unwrap();
    assert!(coordinator.register(item("evo-b")).is_err());
    {
        let mut state = coordinator.state.lock().unwrap();
        state.items[0].status = "review_required".to_string();
    }
    let (_, _) = coordinator
        .begin_action(
            "owner-a",
            "evo-a",
            "approve",
            Some("verified".to_string()),
            "pc_operator:owner-a",
            "local_pc_ui",
        )
        .unwrap();
    let reviewed = coordinator
        .commit_action("owner-a", "evo-a", "approve")
        .unwrap();
    assert_eq!(reviewed.status, "completed");
    assert_eq!(reviewed.review_verdict.as_deref(), Some("approved"));
    assert_eq!(reviewed.reviewed_by.as_deref(), Some("pc_operator:owner-a"));

    let reloaded = SelfEvolutionCoordinator::new(coordinator.path.clone());
    let (items, _) = reloaded.list_for_owner("owner-a").unwrap();
    assert_eq!(items.len(), 1);
    assert_eq!(items[0].status, "completed");
}

#[test]
fn retryable_quota_failure_waits_and_can_be_reserved_again() {
    let coordinator = coordinator("quota-retry");
    coordinator.register(item("evo-retry")).unwrap();
    coordinator
        .mark_dispatch_failed("evo-retry", "429 quota temporarily exhausted")
        .unwrap();
    let (items, _) = coordinator.list_for_owner("owner-a").unwrap();
    assert_eq!(items[0].status, "retry_wait");
    assert_eq!(items[0].retry_count, 1);
    assert!(items[0].next_retry_at_ms.is_some());

    coordinator.state.lock().unwrap().items[0].next_retry_at_ms = Some(0);
    let reserved = coordinator.reserve_next().unwrap().unwrap();
    assert_eq!(reserved.generation, 1);
    assert_eq!(reserved.status, "starting");
}

#[test]
fn action_intent_is_durable_before_queue_transition() {
    let coordinator = coordinator("action-ledger");
    let mut current = item("evo-action");
    current.status = "review_required".to_string();
    current.active_task_id = Some("task-review".to_string());
    coordinator.register(current).unwrap();

    let (pending, _) = coordinator
        .begin_action(
            "owner-a",
            "evo-action",
            "reject",
            Some("needs changes".to_string()),
            "pc_operator:owner-a",
            "local_pc_ui",
        )
        .unwrap();
    assert_eq!(pending.status, "review_required");
    assert_eq!(pending.pending_action.as_ref().unwrap().action, "reject");

    let committed = coordinator
        .commit_action("owner-a", "evo-action", "reject")
        .unwrap();
    assert_eq!(committed.status, "paused");
    assert!(committed.pending_action.is_none());
}
