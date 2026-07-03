#[path = "node_agent_cli_session_bridge_state.rs"]
mod state;

use serde_json::{json, Value};

use crate::{
    node_agent_active_task::ActiveCliPromptView,
    node_agent_cli_session_bridge_capabilities::{
        capability_summary, insert_compat_fields, SIDECAR_TOOL_APPROVAL_RECOVERY_SUPPORTED,
    },
    node_agent_cli_sidecar::{now_ms, sidecar_status_view, CliSidecarSessionRecord},
    node_agent_task_journal::TaskJournalRecord,
};
use state::{
    display_summary, latest_recoverable_task, recommended_primary_action, restart_next_action,
    summary_text, ContinuitySummary,
};

pub(crate) fn status_payload() -> Value {
    status_payload_for(&[], &[], &[])
}

pub(crate) fn status_payload_for(
    active_controls: &[ActiveCliPromptView],
    recent_records: &[TaskJournalRecord],
    sidecar_sessions: &[CliSidecarSessionRecord],
) -> Value {
    let summary = ContinuitySummary::from_state(active_controls, recent_records, sidecar_sessions);
    let current_state = summary.current_state();
    let latest_recoverable_task =
        latest_recoverable_task(active_controls, recent_records, sidecar_sessions);
    let latest_sidecar_session = sidecar_sessions
        .iter()
        .filter(|session| session.can_replay_output_at(now_ms()))
        .max_by(|left, right| {
            left.last_seen_at_ms
                .cmp(&right.last_seen_at_ms)
                .then_with(|| left.started_at_ms.cmp(&right.started_at_ms))
        })
        .map(sidecar_status_view);
    let managed_sidecar_available = summary.sidecar_stream_replay_count > 0;
    let managed_tty_available = summary.sidecar_attachable_count > 0;
    let managed_pipe_available =
        summary.sidecar_stream_replay_count > summary.sidecar_attachable_count;
    let sidecar_approval_available = summary.sidecar_approval_recoverable_count > 0;
    let capability_summary = capability_summary(
        managed_tty_available,
        managed_pipe_available,
        sidecar_approval_available,
        summary.sidecar_attachable_count,
        summary.sidecar_stream_replay_count,
        summary.sidecar_approval_recoverable_count,
        summary.codex_session_count,
        summary.recent_record_count,
    );
    let status = if managed_sidecar_available {
        "sidecar_recoverable_continuity"
    } else {
        "recoverable_continuity"
    };
    let mode = if managed_sidecar_available {
        if managed_tty_available {
            "managed_pty_conpty_sidecar_attach"
        } else {
            "managed_pipe_json_sidecar_follow"
        }
    } else {
        "spawned_process_json_bridge_with_journal_recovery"
    };
    let restart_mode = if managed_sidecar_available {
        if managed_tty_available {
            "managed_pty_conpty_sidecar_attach"
        } else {
            "managed_pipe_json_sidecar_follow"
        }
    } else {
        "task_journal_snapshot_and_cli_native_resume"
    };
    let restart_reason = if managed_tty_available {
        "由一龙启动并由 sidecar 持有的 PTY/ConPTY CLI 会话可以在节点重启后通过本机 attach API 重新读写；prompt/API key 仍不写入恢复文件。"
    } else if managed_sidecar_available {
        "由一龙启动并由 pipe sidecar 持有的 Codex JSON 会话可以在节点重启后继续回放输出、取消任务并恢复任务状态；prompt/API key 仍不写入恢复文件。"
    } else {
        "节点重启后使用本机 journal 和云端任务快照恢复上下文；Codex 任务还可用已记录 session id 自动尝试 exec resume。"
    };
    let second_recommended_action = if managed_tty_available {
        "节点重启后先 attach sidecar 的 PTY/ConPTY 会话；只有 sidecar mailbox 仍可验证的审批才允许继续批准。"
    } else if managed_sidecar_available {
        "节点重启后先跟随 pipe sidecar 的 JSON 输出和 journal；该模式只支持输出回放与取消，不支持终端输入。"
    } else {
        "节点重启或任务 detached 后，回放 journal/快照并开新一轮继续，不再批准非 sidecar 的旧审批卡。"
    };
    let state_summary = json!({
        "active_control_count": summary.active_control_count,
        "sidecar_session_count": summary.sidecar_session_count,
        "sidecar_stream_replay_count": summary.sidecar_stream_replay_count,
        "sidecar_attachable_count": summary.sidecar_attachable_count,
        "sidecar_approval_recoverable_count": summary.sidecar_approval_recoverable_count,
        "recent_record_count": summary.recent_record_count,
        "detached_running_count": summary.detached_running_count,
        "terminal_record_count": summary.terminal_record_count,
        "codex_session_count": summary.codex_session_count,
        "route_a_record_count": summary.route_a_record_count,
        "route_b_record_count": summary.route_b_record_count,
        "route_c_record_count": summary.route_c_record_count,
        "last_updated_at_ms": summary.last_updated_at_ms,
    });
    let restart_recovery = json!({
        "supported": true,
        "mode": restart_mode,
        "safe_after_node_restart": true,
        "restores_prompt_or_api_key": false,
        "restores_original_tty": managed_tty_available,
        "restores_json_pipe_output": managed_sidecar_available,
        "restores_tool_approval_waiter": sidecar_approval_available,
        "restores_tool_approval_waiter_supported": SIDECAR_TOOL_APPROVAL_RECOVERY_SUPPORTED,
        "restores_tool_approval_waiter_currently_available": sidecar_approval_available,
        "next_action": restart_next_action(&summary),
        "reason": restart_reason
    });
    let resume_order = vec![
        json!({
            "kind": "managed_pty_conpty_sidecar_attach",
            "label": "重接一龙 sidecar 持有的 PTY/ConPTY CLI 会话",
            "available_when": "任务由一龙 sidecar 启动且 sidecar 心跳仍有效",
            "currently_available": managed_tty_available,
            "requires_new_task": false
        }),
        json!({
            "kind": "managed_pipe_json_sidecar_follow",
            "label": "跟随一龙 pipe sidecar 持有的 Codex JSON 会话",
            "available_when": "Codex 任务由一龙 pipe sidecar 启动且 sidecar 心跳仍有效",
            "currently_available": managed_pipe_available,
            "requires_new_task": false
        }),
        json!({
            "kind": "live_control_handle",
            "label": "重连本机控制句柄",
            "available_when": "节点仍持有该任务 run_handle",
            "currently_available": summary.active_control_count > 0,
            "requires_new_task": false
        }),
        json!({
            "kind": "journal_replay",
            "label": "回放本机 journal",
            "available_when": "本机仍有任务 journal",
            "currently_available": summary.recent_record_count > 0,
            "requires_new_task": false
        }),
        json!({
            "kind": "codex_session_resume",
            "label": "自动续接 Codex session",
            "available_when": "journal 记录了 Codex session id 和 scope_key",
            "currently_available": summary.codex_session_count > 0,
            "requires_new_task": true
        }),
        json!({
            "kind": "cloud_snapshot_continue",
            "label": "基于云端快照开启新任务",
            "available_when": "本机运行句柄或 journal 不存在",
            "currently_available": summary.recent_record_count == 0,
            "requires_new_task": true
        }),
    ];
    let routes = vec![
        json!({
            "name": "Codex CLI",
            "mode": "managed_pipe_json_sidecar_or_exec_json_resume",
            "tty_takeover_supported": managed_tty_available,
            "json_pipe_sidecar_supported": true,
            "continuity": "managed pipe JSON sidecar output replay/cancel; fallback codex exec resume --json <thread>; PTY sidecar only for explicit terminal attach/TUI"
        }),
        json!({
            "name": "Copilot CLI",
            "mode": "managed_pty_conpty_sidecar_or_continue_in_workspace",
            "tty_takeover_supported": managed_sidecar_available,
            "continuity": "managed PTY/ConPTY sidecar attach/read/write/resize; fallback copilot --continue"
        }),
        json!({
            "name": "Fallback",
            "mode": "backend_context_handoff",
            "tty_takeover_supported": false,
            "continuity": "recent backend conversation records"
        }),
    ];
    let sidecar_protocol_mode = if managed_tty_available {
        "managed_pty_conpty_attach_read_write_resize"
    } else if managed_pipe_available {
        "managed_pipe_json_output_replay_cancel"
    } else {
        "managed_pipe_json_or_pty_conpty"
    };
    let sidecar_attach_api = json!({
        "read": "/api/cli-sidecars/:task_id/attach?since=<offset>",
        "write": "/api/cli-sidecars/:task_id/input",
        "resize": "/api/cli-sidecars/:task_id/resize"
    });
    let not_supported = vec![
        "attach_external_cli_tty_not_started_by_elon_sidecar",
        "attach_non_sidecar_external_cli_tty",
        "approve_tool_after_node_restart_without_managed_sidecar",
    ];
    let continuity_modes = vec![
        "managed PTY/ConPTY sidecar attach/read/write/resize",
        "managed pipe JSON sidecar output replay/cancel",
        "codex exec resume --json <thread>",
        "copilot --continue",
        "backend conversation continuity note",
    ];
    let recommended_next_actions = vec![
        recommended_primary_action(&summary),
        second_recommended_action,
        "有 Codex session 记录时由节点自动尝试 exec resume；失败时清理旧 session 并重新开始。",
        "任意外部终端仍不可接管；只有一龙管理的 sidecar 会话进入恢复协议。",
    ];
    let future_work = vec![
        "PC 网页端应区分 pipe JSON sidecar 的结构化过程面板和 PTY/ConPTY sidecar 的终端面板。",
        "补充屏幕级终端 buffer；PTY 前端显示 PTY 字节流的 ANSI 可读视图，pipe JSON 前端显示公开过程卡片。",
    ];

    let mut payload = Value::Object(serde_json::Map::new());
    {
        let object = payload
            .as_object_mut()
            .expect("fresh JSON object should be mutable");
        object.insert("status".to_string(), json!(status));
        object.insert("mode".to_string(), json!(mode));
        object.insert("current_state".to_string(), json!(current_state));
        object.insert("tty_takeover_supported".to_string(), json!(false));
        object.insert("pty_takeover_supported".to_string(), json!(false));
        object.insert("managed_tty_reattach_supported".to_string(), json!(true));
        object.insert(
            "managed_tty_reattach_currently_available".to_string(),
            json!(managed_tty_available),
        );
        object.insert("managed_conpty_sidecar_supported".to_string(), json!(true));
        object.insert(
            "managed_conpty_sidecar_active".to_string(),
            json!(managed_tty_available),
        );
        object.insert(
            "managed_pipe_json_sidecar_supported".to_string(),
            json!(true),
        );
        object.insert(
            "managed_pipe_json_sidecar_active".to_string(),
            json!(managed_pipe_available),
        );
        object.insert("sidecar_protocol_supported".to_string(), json!(true));
        object.insert(
            "sidecar_protocol_mode".to_string(),
            json!(sidecar_protocol_mode),
        );
        object.insert("sidecar_attach_api".to_string(), sidecar_attach_api);
        object.insert(
            "sidecar_stream_replay_count".to_string(),
            json!(summary.sidecar_stream_replay_count),
        );
        object.insert(
            "sidecar_attachable_count".to_string(),
            json!(summary.sidecar_attachable_count),
        );
        object.insert(
            "sidecar_approval_recoverable_count".to_string(),
            json!(summary.sidecar_approval_recoverable_count),
        );
        object.insert(
            "process_handle_reconnect_supported".to_string(),
            json!(true),
        );
        object.insert("restart_recovery_supported".to_string(), json!(true));
        object.insert(
            "post_restart_approval_supported".to_string(),
            json!(sidecar_approval_available),
        );
        object.insert("json_stream_supported".to_string(), json!(true));
        object.insert("task_journal_supported".to_string(), json!(true));
        object.insert("codex_resume_supported".to_string(), json!(true));
        object.insert("copilot_continue_supported".to_string(), json!(true));
        object.insert(
            "backend_context_fallback_supported".to_string(),
            json!(true),
        );
        object.insert(
            "can_reconnect_live_control".to_string(),
            json!(summary.active_control_count > 0),
        );
        object.insert(
            "can_resume_after_node_restart".to_string(),
            json!(summary.can_resume_after_restart()),
        );
        object.insert(
            "can_resume_codex_session".to_string(),
            json!(summary.codex_session_count > 0),
        );
        object.insert(
            "can_continue_from_snapshot".to_string(),
            json!(summary.recent_record_count > 0),
        );
        object.insert(
            "can_approve_after_node_restart".to_string(),
            json!(sidecar_approval_available),
        );
        object.insert(
            "display_summary".to_string(),
            json!(display_summary(&summary)),
        );
        object.insert("summary".to_string(), json!(summary_text(&summary)));
        object.insert("state_summary".to_string(), state_summary);
        object.insert(
            "latest_recoverable_task".to_string(),
            json!(latest_recoverable_task),
        );
        object.insert(
            "latest_sidecar_session".to_string(),
            json!(latest_sidecar_session),
        );
        object.insert("restart_recovery".to_string(), restart_recovery);
        object.insert("not_supported".to_string(), json!(not_supported));
        object.insert("continuity_modes".to_string(), json!(continuity_modes));
        object.insert("resume_order".to_string(), json!(resume_order));
        object.insert(
            "recommended_next_actions".to_string(),
            json!(recommended_next_actions),
        );
        object.insert("future_work".to_string(), json!(future_work));
        object.insert("routes".to_string(), json!(routes));
    }
    insert_compat_fields(
        &mut payload,
        capability_summary,
        managed_tty_available,
        sidecar_approval_available,
    );
    payload
}

#[cfg(test)]
mod tests {
    use super::{status_payload, status_payload_for};
    use crate::{
        node_agent_active_task::ActiveCliPromptView,
        node_agent_cli_sidecar::{now_ms, CliSidecarSessionRecord},
        node_agent_task_journal::TaskJournalRecord,
    };

    #[test]
    fn static_status_declares_recoverable_bridge_without_tty_takeover() {
        let status = status_payload();

        assert_eq!(status["tty_takeover_supported"], false);
        assert_eq!(status["status"], "recoverable_continuity");
        assert_eq!(status["current_state"], "ready_no_session");
        assert_eq!(status["restart_recovery_supported"], true);
        assert_eq!(status["post_restart_approval_supported"], false);
        assert_eq!(status["post_restart_approval_capability_supported"], true);
        assert_eq!(status["post_restart_approval_currently_available"], false);
        assert_eq!(status["can_resume_after_node_restart"], false);
        assert_eq!(status["json_stream_supported"], true);
        assert_eq!(status["codex_resume_supported"], true);
        assert!(status["display_summary"]
            .as_str()
            .unwrap_or_default()
            .contains("journal"));
        assert!(status["summary"]
            .as_str()
            .unwrap_or_default()
            .contains("当前没有可重接 sidecar"));
        assert!(status["continuity_modes"]
            .as_array()
            .is_some_and(|items| items.len() >= 3));
        assert!(status["not_supported"]
            .as_array()
            .unwrap()
            .iter()
            .any(|item| {
                item.as_str() == Some("attach_external_cli_tty_not_started_by_elon_sidecar")
            }));
        assert!(status["resume_order"]
            .as_array()
            .unwrap()
            .iter()
            .any(|item| {
                item["kind"].as_str() == Some("codex_session_resume")
                    && item["requires_new_task"].as_bool() == Some(true)
                    && item["currently_available"].as_bool() == Some(false)
            }));
        assert!(status["recommended_next_actions"]
            .as_array()
            .unwrap()
            .iter()
            .any(|item| {
                item.as_str()
                    .unwrap_or_default()
                    .contains("不再批准非 sidecar")
            }));
        assert_eq!(
            status["sidecar_protocol_mode"],
            "managed_pipe_json_or_pty_conpty"
        );
        assert_eq!(status["managed_conpty_sidecar_active"], false);
        assert_eq!(status["managed_pipe_json_sidecar_active"], false);
        assert_eq!(status["managed_tty_reattach_capability_supported"], true);
        assert_eq!(status["managed_tty_reattach_currently_available"], false);
        assert_eq!(status["sidecar_tool_approval_recovery_supported"], true);
        assert_eq!(
            status["sidecar_tool_approval_recovery_currently_available"],
            false
        );
        assert_eq!(
            status["restart_recovery"]["restores_tool_approval_waiter_supported"],
            true
        );
        assert_eq!(
            status["restart_recovery"]["restores_tool_approval_waiter_currently_available"],
            false
        );
        assert_eq!(
            status["capability_summary"]["managed_pty_conpty_sidecar"]["supported"],
            true
        );
        assert_eq!(
            status["capability_summary"]["managed_pty_conpty_sidecar"]["currently_available"],
            false
        );
        assert_eq!(
            status["capability_summary"]["managed_pipe_json_sidecar"]["supported"],
            true
        );
        assert_eq!(
            status["capability_summary"]["managed_pipe_json_sidecar"]["currently_available"],
            false
        );
        assert_eq!(
            status["capability_summary"]["post_restart_tool_approval"]["supported"],
            true
        );
        assert_eq!(
            status["capability_summary"]["post_restart_tool_approval"]["currently_available"],
            false
        );
        assert_eq!(
            status["capability_summary"]["external_tty_takeover"]["supported"],
            false
        );
        assert_eq!(
            status["sidecar_attach_api"]["write"],
            "/api/cli-sidecars/:task_id/input"
        );
    }

    #[test]
    fn live_active_control_is_exposed_as_current_reconnect_path() {
        let active = vec![active_control("req-live")];
        let status = status_payload_for(&active, &[], &[]);

        assert_eq!(status["current_state"], "live_control_available");
        assert_eq!(status["can_reconnect_live_control"], true);
        assert_eq!(status["can_resume_after_node_restart"], false);
        assert_eq!(
            status["latest_recoverable_task"]["recovery_kind"],
            "live_control_handle"
        );
        assert_eq!(status["latest_recoverable_task"]["can_cancel"], true);
        assert_eq!(status["state_summary"]["active_control_count"], 1);
    }

    #[test]
    fn detached_codex_record_makes_restart_resume_available() {
        let records = vec![codex_record("req-codex", "running", 200)];
        let status = status_payload_for(&[], &records, &[]);

        assert_eq!(status["current_state"], "codex_session_resumable");
        assert_eq!(status["can_resume_after_node_restart"], true);
        assert_eq!(status["can_resume_codex_session"], true);
        assert_eq!(
            status["restart_recovery"]["next_action"],
            "codex_exec_resume_or_snapshot_continue"
        );
        assert_eq!(
            status["latest_recoverable_task"]["recovery_kind"],
            "codex_session_resume"
        );
        assert_eq!(
            status["latest_recoverable_task"]["can_resume_codex_session"],
            true
        );
        assert_eq!(status["state_summary"]["detached_running_count"], 1);
        assert_eq!(status["state_summary"]["route_a_record_count"], 1);
    }

    #[test]
    fn route_b_and_c_records_are_counted_for_snapshot_continue() {
        let records = vec![
            record(
                "req-b",
                "api-runtime",
                "route_b_api_runtime",
                "finished",
                100,
            ),
            record(
                "req-c",
                "server-runtime",
                "route_c_server_runtime",
                "cancel_requested",
                300,
            ),
        ];
        let status = status_payload_for(&[], &records, &[]);

        assert_eq!(status["current_state"], "detached_journal_recoverable");
        assert_eq!(status["can_continue_from_snapshot"], true);
        assert_eq!(
            status["restart_recovery"]["next_action"],
            "journal_replay_then_snapshot_continue"
        );
        assert_eq!(status["state_summary"]["route_b_record_count"], 1);
        assert_eq!(status["state_summary"]["route_c_record_count"], 1);
        assert_eq!(status["state_summary"]["detached_running_count"], 1);
        assert_eq!(status["state_summary"]["terminal_record_count"], 1);
        assert_eq!(status["latest_recoverable_task"]["task_id"], "req-c");
    }

    #[test]
    fn sidecar_session_takes_priority_for_restart_attach_and_approval_recovery() {
        let sidecars = vec![sidecar("sidecar-1", "req-sidecar", now_ms())];
        let records = vec![record(
            "req-sidecar",
            "codex",
            "route_a_external_cli",
            "running",
            100,
        )];
        let status = status_payload_for(&[], &records, &sidecars);

        assert_eq!(status["status"], "sidecar_recoverable_continuity");
        assert_eq!(status["current_state"], "managed_sidecar_attachable");
        assert_eq!(status["managed_tty_reattach_supported"], true);
        assert_eq!(status["managed_tty_reattach_currently_available"], true);
        assert_eq!(status["can_resume_after_node_restart"], true);
        assert_eq!(status["can_approve_after_node_restart"], true);
        assert_eq!(status["post_restart_approval_capability_supported"], true);
        assert_eq!(status["post_restart_approval_currently_available"], true);
        assert_eq!(
            status["capability_summary"]["managed_pty_conpty_sidecar"]["currently_available"],
            true
        );
        assert_eq!(
            status["capability_summary"]["post_restart_tool_approval"]["recoverable_count"],
            1
        );
        assert_eq!(
            status["restart_recovery"]["mode"],
            "managed_pty_conpty_sidecar_attach"
        );
        assert_eq!(status["restart_recovery"]["restores_original_tty"], true);
        assert_eq!(
            status["restart_recovery"]["restores_tool_approval_waiter"],
            true
        );
        assert_eq!(
            status["latest_recoverable_task"]["recovery_kind"],
            "managed_pty_conpty_sidecar_attach"
        );
        assert_eq!(
            status["latest_recoverable_task"]["can_write_terminal"],
            true
        );
        assert_eq!(
            status["latest_recoverable_task"]["can_resize_terminal"],
            true
        );
        assert_eq!(status["latest_sidecar_session"]["session_id"], "sidecar-1");
        assert_eq!(status["state_summary"]["sidecar_stream_replay_count"], 1);
        assert_eq!(status["state_summary"]["sidecar_attachable_count"], 1);
        assert_eq!(
            status["resume_order"].as_array().unwrap().first().unwrap()["kind"],
            "managed_pty_conpty_sidecar_attach"
        );
    }

    #[test]
    fn pipe_json_sidecar_session_is_followable_without_tty() {
        let sidecars = vec![pipe_sidecar("sidecar-pipe-1", "req-pipe", now_ms())];
        let records = vec![record(
            "req-pipe",
            "codex",
            "route_a_external_cli",
            "running",
            100,
        )];
        let status = status_payload_for(&[], &records, &sidecars);

        assert_eq!(status["status"], "sidecar_recoverable_continuity");
        assert_eq!(
            status["current_state"],
            "managed_pipe_json_sidecar_followable"
        );
        assert_eq!(
            status["restart_recovery"]["next_action"],
            "managed_pipe_json_sidecar_follow"
        );
        assert_eq!(
            status["restart_recovery"]["mode"],
            "managed_pipe_json_sidecar_follow"
        );
        assert_eq!(status["restart_recovery"]["restores_original_tty"], false);
        assert_eq!(
            status["restart_recovery"]["restores_json_pipe_output"],
            true
        );
        assert_eq!(status["managed_conpty_sidecar_active"], false);
        assert_eq!(status["managed_pipe_json_sidecar_active"], true);
        assert_eq!(status["managed_tty_reattach_currently_available"], false);
        assert_eq!(
            status["sidecar_protocol_mode"],
            "managed_pipe_json_output_replay_cancel"
        );
        assert_eq!(
            status["latest_recoverable_task"]["recovery_kind"],
            "managed_pipe_json_sidecar_follow"
        );
        assert_eq!(
            status["latest_recoverable_task"]["can_attach_sidecar"],
            false
        );
        assert_eq!(
            status["latest_recoverable_task"]["can_stream_live_output"],
            true
        );
        assert_eq!(
            status["latest_recoverable_task"]["can_write_terminal"],
            false
        );
        assert_eq!(status["latest_recoverable_task"]["can_cancel"], true);
        assert_eq!(
            status["capability_summary"]["managed_pipe_json_sidecar"]["currently_available"],
            true
        );
        assert_eq!(
            status["capability_summary"]["managed_pty_conpty_sidecar"]["currently_available"],
            false
        );
        assert_eq!(status["state_summary"]["sidecar_stream_replay_count"], 1);
        assert_eq!(status["state_summary"]["sidecar_attachable_count"], 0);
    }

    fn active_control(req_id: &str) -> ActiveCliPromptView {
        ActiveCliPromptView {
            req_id: req_id.to_string(),
            run_handle_id: req_id.to_string(),
            cli_name: "codex".to_string(),
            route: "route_a_external_cli".to_string(),
            cwd: Some("D:/demo".to_string()),
            runtime_permission: Some("project_write".to_string()),
            started_at_ms: 1,
            last_heartbeat_ms: 2,
            control_lease_expires_at_ms: 47_000,
            os_pid: Some(4242),
            control_handle_live: true,
            pending_approvals: Vec::new(),
        }
    }

    fn codex_record(req_id: &str, status: &str, updated_at_ms: u128) -> TaskJournalRecord {
        let mut record = record(
            req_id,
            "codex",
            "route_a_external_cli",
            status,
            updated_at_ms,
        );
        record.codex_session_id = Some("codex-session-uuid".to_string());
        record.codex_session_scope_key = Some("scope-key".to_string());
        record.codex_session_updated_at_ms = Some(updated_at_ms);
        record
    }

    fn sidecar(session_id: &str, task_id: &str, last_seen_at_ms: u128) -> CliSidecarSessionRecord {
        let mut session = CliSidecarSessionRecord::managed_conpty(
            session_id,
            task_id,
            "codex",
            "route_a_external_cli",
            Some("D:/demo".to_string()),
            Some("npipe://elon/sidecar-1".to_string()),
            Some(100),
            Some(200),
            last_seen_at_ms,
        );
        session.last_seen_at_ms = last_seen_at_ms;
        session
    }

    fn pipe_sidecar(
        session_id: &str,
        task_id: &str,
        last_seen_at_ms: u128,
    ) -> CliSidecarSessionRecord {
        let mut session = CliSidecarSessionRecord::managed_pipe_json(
            session_id,
            task_id,
            "codex",
            "route_a_external_cli",
            Some("D:/demo".to_string()),
            Some("D:/state/output.jsonl".to_string()),
            Some(100),
            Some(200),
            last_seen_at_ms,
        );
        session.last_seen_at_ms = last_seen_at_ms;
        session
    }

    fn record(
        req_id: &str,
        cli_name: &str,
        route: &str,
        status: &str,
        updated_at_ms: u128,
    ) -> TaskJournalRecord {
        TaskJournalRecord {
            req_id: req_id.to_string(),
            cli_name: cli_name.to_string(),
            route: Some(route.to_string()),
            run_handle_id: Some(req_id.to_string()),
            cwd: Some("D:/demo".to_string()),
            runtime_permission: Some("project_write".to_string()),
            os_pid: None,
            process_started_at_ms: None,
            codex_session_id: None,
            codex_session_scope_key: None,
            codex_session_updated_at_ms: None,
            status: status.to_string(),
            started_at_ms: 1,
            updated_at_ms,
            cancel_requested_at_ms: None,
        }
    }
}
