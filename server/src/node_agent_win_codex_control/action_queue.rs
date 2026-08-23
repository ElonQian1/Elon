use super::*;

pub(super) fn enqueue(
    hub: &WinCodexControlHub,
    trace_id: &str,
    kind: &str,
    route: Option<&str>,
    provider_id: Option<&str>,
    target_release_identity: Option<&str>,
    requested_by: &str,
    current_release_identity: &str,
) -> Result<WinControlAction, String> {
    let kind = validate_action_kind(kind)?;
    let route = validate_action_route(kind, route)?;
    let provider_id = validate_action_provider(kind, provider_id)?;
    let target_release_identity =
        validate_action_target(kind, target_release_identity, requested_by)?;
    let now = now_ms();
    let already_current = kind == "update_and_restart"
        && target_release_identity.as_deref()
            == validate_release_identity(current_release_identity)
                .ok()
                .as_deref();
    let action = WinControlAction {
        action_id: format!("win_act_{}", uuid::Uuid::new_v4().simple()),
        trace_id: clean_identifier(trace_id, "win_action"),
        kind: kind.to_string(),
        route,
        provider_id,
        target_release_identity,
        requested_by: clean_identifier(requested_by, "local_admin"),
        requested_at_ms: now,
        expires_at_ms: now.saturating_add(ACTION_TTL_MS),
        status: if already_current {
            "succeeded"
        } else {
            "queued"
        }
        .to_string(),
        receipt: already_current.then(|| WinControlReceipt {
            status: "succeeded".to_string(),
            message: Some(
                "already_current_noop: 当前 Win 节点已运行精确目标版本，未排队且未重启 Tauri。"
                    .to_string(),
            ),
            route: None,
            window_state: None,
            at_ms: Some(now),
        }),
    };
    let mut state = lock(&hub.inner);
    expire_actions(&mut state, now);
    state.actions.push_back(action.clone());
    while state.actions.len() > MAX_ACTIONS {
        state.actions.pop_front();
    }
    drop(state);
    hub.record(
        &action.trace_id,
        "control",
        "info",
        if already_current { "action.noop" } else { "action.queued" },
        if already_current { "Win 节点已在精确目标版本，更新动作直接完成" } else { "已排队 Win 语义动作" },
        json!({"action_id": action.action_id, "kind": action.kind, "route": action.route, "provider_id": action.provider_id, "target_release_identity": action.target_release_identity, "completion_mode": if already_current { "already_current_noop" } else { "await_tauri_receipt" }}),
    );
    Ok(action)
}
